"""Call-scope distribution primitive.

Computes the fraction of call-expression nodes that appear at module scope
(i.e. not nested inside any ``function_definition`` ancestor) vs all
call-expression nodes in the file.

Per-cluster baseline: (mean, std) of that fraction across the cluster's files.
Score-time: two-sided tail-z penalty clipped at ``cluster_bonus_clip``.

**Two-sided rationale.** The interesting anomaly can be in either direction:
"unusually all-top-level" (a fetch at module scope where the cluster only
calls inside functions) OR "unusually all-nested" (the bare hunk lacks
host-file context so module-scope calls aren't visible).  Using
``abs(tail_z)`` instead of a one-sided check fires on extreme deviation in
either direction.

**Ramp.** Contribution = ``min(cluster_bonus_clip, max(0.0, |tail_z| − 1.0))``.
Linear ramp starts at 1σ; clipped at ``cluster_bonus_clip`` so a single
primitive cannot dominate the total.
"""

from __future__ import annotations

import math
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER
from argot.scoring.scorers.call_receiver import _PY_CALL_TYPES, _TS_CALL_TYPES, _walk_nodes
from argot.scoring.scorers.shape_primitive import Language

# Node type that acts as the boundary between module scope and nested scope.
# This kind is present in both tree-sitter-python and tree-sitter-typescript.
_FUNCTION_BOUNDARY: str = "function_definition"

# Minimum number of files with at least one call expression for the cluster
# baseline to be considered statistically meaningful. Fewer files than this
# causes fit_cluster_baseline to return None (abstain on that cluster).
_MIN_FILES_FOR_BASELINE: int = 3


@dataclass(frozen=True)
class _CallScopeBaseline:
    mean: float
    std: float


def _has_function_ancestor(node: object, boundary: str) -> bool:
    """Return True iff *node* has an ancestor of type *boundary*."""
    from tree_sitter import Node as TsNode  # local import avoids top-level coupling

    parent = getattr(node, "parent", None)
    while parent is not None and isinstance(parent, TsNode):
        if parent.type == boundary:
            return True
        parent = getattr(parent, "parent", None)
    return False


def _fraction_module_scope(source: str, language: Language) -> float | None:
    """Return module-scope call fraction for *source*, or None if 0 calls.

    ``None`` signals that this file contributes no denominator and should be
    skipped when computing the per-cluster baseline.
    """
    if language == "python":
        parser = _PY_PARSER
        call_types = _PY_CALL_TYPES
    else:
        parser = _TS_PARSER
        call_types = _TS_CALL_TYPES

    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return None

    total = 0
    module_scope = 0
    for node in _walk_nodes(tree.root_node):
        if node.type in call_types:
            total += 1
            if not _has_function_ancestor(node, _FUNCTION_BOUNDARY):
                module_scope += 1
    del tree

    if total == 0:
        return None
    return module_scope / total


class CallScopeFraction:
    """Scalar shape primitive: fraction of calls at module scope.

    Implements the ``ShapePrimitive[_CallScopeBaseline]`` Protocol.
    Stateless at construction time; language is captured on the first
    ``fit_cluster_baseline`` call and reused at score time.
    """

    name: str = "call_scope_fraction"
    min_cluster_size: int = 10
    cluster_bonus_clip: float = 5.0

    def __init__(self) -> None:
        # Language is not known at construction time; captured during fit.
        self._language: Language | None = None

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _CallScopeBaseline | None:
        """Compute (mean, std) of module-scope call fraction across cluster files.

        Skips files with 0 call expressions (no denominator). Returns None when
        fewer than ``_MIN_FILES_FOR_BASELINE`` files have at least one call.
        """
        self._language = language
        fractions: list[float] = []
        for _path, source in cluster_files:
            frac = _fraction_module_scope(source, language)
            if frac is not None:
                fractions.append(frac)
        if len(fractions) < _MIN_FILES_FOR_BASELINE:
            return None
        mean = sum(fractions) / len(fractions)
        variance = sum((f - mean) ** 2 for f in fractions) / len(fractions)
        std = math.sqrt(variance)
        return _CallScopeBaseline(mean=mean, std=std)

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _CallScopeBaseline | None,
        cluster_size: int,
    ) -> float:
        """Return two-sided tail-z contribution, clipped at ``cluster_bonus_clip``.

        Abstains (returns 0.0) when:
          - ``baseline`` is None (cluster had too few files with calls), or
          - ``cluster_size < self.min_cluster_size`` (cluster too small), or
          - ``self._language`` is unknown (fit was never called), or
          - the hunk has 0 call expressions.
        """
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0
        language = self._language
        if language is None:
            return 0.0
        hunk_frac = _fraction_module_scope(hunk_content, language)
        if hunk_frac is None:
            return 0.0
        tail_z = (hunk_frac - baseline.mean) / max(baseline.std, 1e-6)
        return min(self.cluster_bonus_clip, max(0.0, abs(tail_z) - 1.0))


__all__ = ["CallScopeFraction"]
