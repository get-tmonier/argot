"""Receiver-namespace coverage divergence shape primitive.

Computes the Jensen-Shannon distance between the hunk's distribution over
distinct callee namespace prefixes and the cluster pooled
namespace-prefix distribution.

Math:
  namespace prefix = first segment before '.' in a dotted callee, or the
  full identifier for bare callees (no dot).
  JSD = Jensen-Shannon divergence (base-2 logs, range [0, 1])
  JS distance = sqrt(JSD), range [0, 1]
  contribution = min(cluster_bonus_clip, js_distance * cluster_bonus_clip)
    — linear ramp; js_distance ∈ [0, 1] maps directly to [0, cluster_bonus_clip].

OOV policy: hunk namespaces absent from the cluster alphabet contribute their
probability mass to a single OOV bucket appended to the projected
distribution. The cluster distribution gets zero probability for OOV
(the cluster alphabet exhausts its namespace space by construction). This
preserves JSD ∈ [0, 1] and avoids silent mass loss when the hunk uses
novel namespaces. Two disjoint distributions (all hunk mass in OOV, all
cluster mass in alphabet) yield JS distance = 1.0 = max contribution.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from argot.scoring.scorers.call_receiver import extract_callees
from argot.scoring.scorers.shape_primitive import Language


def _namespace_prefix(callee: str) -> str:
    """Return the first segment of a dotted callee, or the whole callee if bare."""
    return callee.split(".", 1)[0]


@dataclass
class _NamespaceBaseline:
    """Per-cluster namespace-prefix histogram.

    language:     language used to parse cluster files (stored so score() can
                  parse the hunk without a separate language parameter).
    alphabet:     set of namespace prefixes observed across cluster files.
    distribution: probability distribution over alphabet (values sum to 1.0).
    """

    language: Language
    alphabet: frozenset[str]
    distribution: dict[str, float]


def _jsd_distance(p: np.ndarray, q: np.ndarray) -> float:
    """Jensen-Shannon distance: sqrt(JSD) with base-2 logs, range [0, 1].

    Uses the identity JSD(P||Q) = (KL(P||M) + KL(Q||M)) / 2 where M = (P+Q)/2.
    Handles zero-probability entries by masking (0 * log(0) = 0 by convention).
    By construction M_i >= max(P_i, Q_i) / 2 > 0 whenever P_i > 0 or Q_i > 0,
    so log(X / M) is never log(X / 0). Minor floating-point overshoot is clamped
    to [0, 1] before taking the square root.
    """
    m = (p + q) / 2.0
    mask_p = p > 0.0
    mask_q = q > 0.0
    kl_pm = float(np.sum(p[mask_p] * np.log2(p[mask_p] / m[mask_p])))
    kl_qm = float(np.sum(q[mask_q] * np.log2(q[mask_q] / m[mask_q])))
    jsd = (kl_pm + kl_qm) / 2.0
    jsd = max(0.0, min(1.0, jsd))
    return float(np.sqrt(jsd))


@dataclass
class NamespaceJsd:
    """Receiver-namespace coverage divergence primitive.

    Computes the Jensen-Shannon distance between the hunk's namespace-prefix
    distribution and the cluster's pooled namespace-prefix distribution.
    A hunk that uses entirely different namespaces from the cluster scores
    JS distance ≈ 1.0; a hunk whose namespace mix mirrors the cluster scores 0.0.

    Contribution mapping: linear ramp — js_distance * cluster_bonus_clip.
    This preserves the JSD metric's natural units and needs no extra
    hyperparameters. The existing min(sum(weights), cap) clip in
    weighted_contribution_for_file bounds the final total.
    """

    name: str = "namespace_jsd"
    min_cluster_size: int = 10
    cluster_bonus_clip: float = 5.0

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _NamespaceBaseline | None:
        """Build the cluster's pooled namespace-prefix histogram.

        Collects namespace prefixes from every call expression across all cluster
        files, counts occurrences, then normalises to a probability distribution.

        Returns None when:
        - fewer than 3 cluster files have at least one callee (too sparse to
          form a reliable distribution), or
        - the resulting alphabet has fewer than 2 distinct namespaces (JSD is
          ill-defined for a point-mass distribution — all probability is on one
          namespace, so any hunk with a different single namespace has JSD = 1
          trivially, with no discriminative power).
        """
        namespace_counts: dict[str, int] = {}
        files_with_callees = 0

        for _path, source in cluster_files:
            callees = [c for c in extract_callees(source, language) if c is not None]
            if not callees:
                continue
            files_with_callees += 1
            for callee in callees:
                ns = _namespace_prefix(callee)
                namespace_counts[ns] = namespace_counts.get(ns, 0) + 1

        if files_with_callees < 3:
            return None
        if len(namespace_counts) < 2:
            return None

        total = float(sum(namespace_counts.values()))
        distribution = {ns: count / total for ns, count in namespace_counts.items()}
        alphabet = frozenset(namespace_counts.keys())
        return _NamespaceBaseline(language=language, alphabet=alphabet, distribution=distribution)

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _NamespaceBaseline | None,
        cluster_size: int,
    ) -> float:
        """Return JS-distance contribution clipped to cluster_bonus_clip, 0.0 on abstain.

        Abstains (returns 0.0) when:
        - baseline is None (cluster has no trusted baseline),
        - cluster_size < min_cluster_size (below the statistical floor),
        - hunk has 0 callees (JSD is undefined; abstain rather than fabricate signal).

        OOV projection: hunk namespaces absent from the cluster alphabet are pooled
        into a single OOV bucket. The cluster distribution receives 0.0 probability
        for OOV. The resulting vectors have length len(alphabet) + 1. Two fully
        disjoint distributions yield JSD = 1.0 (in bits, base-2), JS distance = 1.0,
        and contribution = cluster_bonus_clip.
        """
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0

        callees = [c for c in extract_callees(hunk_content, baseline.language) if c is not None]
        if not callees:
            return 0.0

        # Build hunk namespace counts
        hunk_counts: dict[str, int] = {}
        for callee in callees:
            ns = _namespace_prefix(callee)
            hunk_counts[ns] = hunk_counts.get(ns, 0) + 1

        # Project onto baseline alphabet + OOV bucket
        # Dimension: one slot per alphabet namespace + one OOV slot at the end.
        alphabet_list = sorted(baseline.alphabet)  # deterministic ordering
        n = len(alphabet_list) + 1  # +1 for OOV
        oov_idx = len(alphabet_list)

        hunk_total = float(sum(hunk_counts.values()))
        cluster_vec = np.zeros(n, dtype=np.float64)
        hunk_vec = np.zeros(n, dtype=np.float64)

        for i, ns in enumerate(alphabet_list):
            cluster_vec[i] = baseline.distribution.get(ns, 0.0)
            hunk_vec[i] = hunk_counts.get(ns, 0) / hunk_total

        # OOV mass: hunk namespaces not in the cluster alphabet
        for ns, count in hunk_counts.items():
            if ns not in baseline.alphabet:
                hunk_vec[oov_idx] += count / hunk_total

        jsd_dist = _jsd_distance(cluster_vec, hunk_vec)
        return min(self.cluster_bonus_clip, jsd_dist * self.cluster_bonus_clip)


__all__ = ["NamespaceJsd"]
