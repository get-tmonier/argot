"""Typical-call-density shape primitive.

Measures under-coverage of cluster-typical callees: the fraction of
``call_expression`` nodes in a hunk whose callee is among the cluster's
top-10 most-attested callees.

Per-cluster baseline: ``(top10_set, mean, std)`` of per-file typical-call
density across the cluster's files.  Only callees that are NOT ``None``
(i.e. whose AST callee chain resolves to a dotted name) contribute to the
numerator; all call-expression nodes — including those whose callee cannot
be resolved — contribute to the denominator.  This is intentional: a hunk
whose calls bottom out at subscripts or string literals has density 0, which
is the anomaly that fires the primitive.

Score-time: one-sided tail-z (positive when hunk is below cluster mean,
i.e. the hunk under-uses cluster-typical callees).
Ramp: ``min(cluster_bonus_clip, max(0.0, z - 1.0))``.

Callee-frequency convention: top-10 is ranked by per-file presence count
(how many cluster files contain each callee), not by raw occurrence count.
This is consistent with ``cluster_callee_counts`` in
``CallReceiverScorer._build_clusters``.

Asymmetric-by-construction: cal hunks come from ``model_a_files`` attached
to typical functions that call cluster-typical things, so their density sits
at or above the cluster mean.  Only hunks that structurally avoid
cluster-typical callees (e.g. string-formula synthesis) fire the primitive.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from argot.scoring.scorers.call_receiver import extract_callees
from argot.scoring.scorers.shape_primitive import Language

# Number of most-frequent cluster callees used as the "typical-call set".
_TOP_N: int = 10

# Minimum number of cluster files that must contribute a defined density
# for the per-cluster baseline to be trusted.
_MIN_VALID_FILES: int = 3


@dataclass(frozen=True)
class _TypicalCallDensityBaseline:
    top10_set: frozenset[str]
    mean: float
    std: float


def _compute_density(
    source: str,
    language: Language,
    top10_set: frozenset[str],
) -> float | None:
    """Return fraction of call nodes whose callee is in top10_set.

    Returns ``None`` when the source has 0 call-expression nodes (no
    denominator — the ratio is undefined; score-time abstains on None).
    ``None`` callees count toward the denominator but never toward the
    numerator.
    """
    callees = extract_callees(source, language)
    denom = len(callees)
    if denom == 0:
        return None
    hits = sum(1 for c in callees if c is not None and c in top10_set)
    return hits / denom


class TypicalCallDensity:
    """Scalar shape primitive: typical-call-density under-coverage.

    Implements ``ShapePrimitive[_TypicalCallDensityBaseline]``.
    Language is not known at construction time; it is captured on the first
    ``fit_cluster_baseline`` call and reused at score time.
    """

    name: str = "typical_call_density"
    min_cluster_size: int = 10
    cluster_bonus_clip: float = 5.0

    def __init__(self) -> None:
        self._language: Language | None = None

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _TypicalCallDensityBaseline | None:
        """Compute the top-10 callee set and (mean, std) of per-file densities.

        Callee frequency is measured by per-file presence count: each callee
        that appears in a cluster file contributes 1 to its count regardless
        of how many times it is called within that file.  ``None`` callees
        (unresolvable callee chains) are excluded from frequency counting.

        Skips files with 0 call-expression nodes (undefined denominator).
        Returns ``None`` when fewer than ``_MIN_VALID_FILES`` files contribute
        a density.
        """
        self._language = language

        files: list[tuple[Path, str]] = list(cluster_files)

        # Build per-file presence counts for each callee across the cluster.
        callee_file_counts: Counter[str] = Counter()
        for _path, source in files:
            seen: set[str] = set()
            for callee in extract_callees(source, language):
                if callee is not None and callee not in seen:
                    callee_file_counts[callee] += 1
                    seen.add(callee)

        # Top-N by per-file frequency; frozenset for O(1) membership test.
        top10_set: frozenset[str] = frozenset(c for c, _ in callee_file_counts.most_common(_TOP_N))

        # Per-file densities; skip files with 0 call nodes.
        densities: list[float] = []
        for _path, source in files:
            d = _compute_density(source, language, top10_set)
            if d is not None:
                densities.append(d)

        if len(densities) < _MIN_VALID_FILES:
            return None

        mean = sum(densities) / len(densities)
        variance = sum((d - mean) ** 2 for d in densities) / len(densities)
        std = math.sqrt(variance)
        return _TypicalCallDensityBaseline(top10_set=top10_set, mean=mean, std=std)

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _TypicalCallDensityBaseline | None,
        cluster_size: int,
    ) -> float:
        """Return one-sided tail-z contribution, clipped at ``cluster_bonus_clip``.

        Positive z signals under-usage of cluster-typical callees.
        Ramp: ``min(clip, max(0.0, z - 1.0))``.

        Abstains (returns 0.0) when:
        - ``baseline`` is None (cluster had too few files with calls),
        - ``cluster_size < self.min_cluster_size`` (cluster too small),
        - language was never set (fit was never called), or
        - the hunk has 0 call-expression nodes, or
        - the top-10 callee set is empty (cluster had no resolvable callees).
        """
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0
        language = self._language
        if language is None:
            return 0.0
        if not baseline.top10_set:
            return 0.0
        hunk_density = _compute_density(hunk_content, language, baseline.top10_set)
        if hunk_density is None:
            return 0.0
        z = (baseline.mean - hunk_density) / max(baseline.std, 1e-6)
        return min(self.cluster_bonus_clip, max(0.0, z - 1.0))


__all__ = ["TypicalCallDensity"]
