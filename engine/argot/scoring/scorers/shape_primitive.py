"""Phase-4 shape-primitive interface.

Era-13 §Phase 4 introduces a family of additive AST-shape primitives
(4a return/raise ratio, 4b call-scope distribution, 4c receiver-namespace
JSD, 4d fall-through-guard count). Each primitive rides alongside
``CallReceiverScorer.weighted_contribution_for_file`` as an additive
penalty term. This module defines the swappable interface they all share.

Design constraints (era-13 §Phase 4 design principles, lines 263-292,
binding):

- One primitive per sub-phase. Each primitive computes a single scalar
  contribution; composition happens at the registry level (clipped to
  ``cluster_bonus`` per the existing weighted_contribution_for_file
  contract).
- Swappable. Adding/removing a primitive is a drop-in: same Protocol,
  same per-cluster baseline mechanism, same calibration plumbing.
  Primitives MUST NOT depend on each other.
- Language-agnostic. Primitives are defined on tree-sitter generic node
  kinds (``function_definition``, ``call_expression``, ``try_statement``,
  ``return_statement``). One implementation runs on both Python and
  TypeScript via the existing per-language tree-sitter grammars.
- Domain-blind. No primitive may reference framework names, function
  names, decorator names, or string literals.
- Per-cluster baseline only. Every primitive compares the hunk against
  its OWN cluster's distribution, never against a global baseline. This
  is the cross-domain-collapse hedge from era 2.
- Cluster-size floor. A primitive's per-cluster baseline is trusted only
  when ``cluster_size >= self.min_cluster_size`` (default 10). Below the
  floor, the primitive abstains (returns 0.0 contribution).

A primitive that fires on a calibration hunk inflates the per-corpus
threshold (the same way ``cluster_bonus`` fires inflate it under
era-11). The calibration metadata path in
``SequentialImportBpeScorer.__init__`` automatically routes primitive
contributions through the threshold computation, so symmetric firing
on cal+fixture cancels. Asymmetric firing (target hunk fires, cal
hunks don't) is the catch mechanism.
"""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path
from typing import Generic, Literal, Protocol, TypeVar

Language = Literal["python", "typescript"]

# The baseline payload shape is primitive-defined: 4a stores
# (mean, std), 4c stores a reference histogram, etc.
B = TypeVar("B")


class ShapePrimitive(Protocol, Generic[B]):
    """Per-cluster scalar AST-shape primitive (Phase 4 swappable interface).

    Lifecycle:
      1. ``fit_cluster_baseline(cluster_files, language) -> B | None``
         called once per cluster at scorer-construction time. Return
         ``None`` to permanently abstain on this cluster (e.g. when the
         primitive doesn't apply to the language, or the cluster has no
         exemplar with the relevant AST shape).
      2. ``score(hunk, baseline, cluster_size) -> float``
         called per hunk at score time. MUST return 0.0 when
         ``cluster_size < self.min_cluster_size`` or ``baseline is None``.
         Otherwise returns a non-negative contribution clipped to
         ``self.cluster_bonus_clip``.

    Implementations MUST be deterministic given the same
    (cluster_files, language) and (hunk, baseline, cluster_size); the
    bench's reproducibility tests rely on it.
    """

    name: str
    """Unique identifier (e.g. ``phase4a_except_return_raise_ratio``).
    Used in scorer-config.json and in the rare-counter stderr lines."""

    min_cluster_size: int
    """Below this cluster size, the primitive abstains (returns 0.0)."""

    cluster_bonus_clip: float
    """Per-primitive cap on the score contribution. Typically equal to
    the SequentialImportBpeScorer ``cluster_bonus`` (5.0)."""

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> B | None: ...

    def score(
        self,
        hunk_content: str,
        *,
        baseline: B | None,
        cluster_size: int,
    ) -> float: ...


__all__ = ["Language", "ShapePrimitive"]
