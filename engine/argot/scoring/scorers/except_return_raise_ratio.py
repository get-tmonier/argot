"""Except-block return/raise ratio primitive.

Scalar: ratio of ``return_statement`` nodes to
``(return_statement + raise_statement | throw_statement)`` nodes that
appear inside ``except_clause`` (Python) or ``catch_clause`` (TypeScript)
handler subtrees.  Per-cluster baseline is ``(mean, std)`` across cluster
files.  Score-time contribution is a tail-z penalty clipped at
``cluster_bonus_clip``.

Ramp curve:
  contribution = min(cluster_bonus_clip, max(0.0, abs(tail_z) - 1.0))
  Ramps linearly from 0 at |z|=1σ to cluster_bonus_clip at
  (1 + cluster_bonus_clip)σ.  Two-sided: catches anomalously HIGH or
  LOW ratios.  Monotone and bounded.
"""

from __future__ import annotations

import statistics
from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from pathlib import Path

from tree_sitter import Node

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER
from argot.scoring.scorers.shape_primitive import Language

# ---------------------------------------------------------------------------
# Baseline payload
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _ExceptRatioBaseline:
    mean: float
    std: float


# ---------------------------------------------------------------------------
# AST helpers
# ---------------------------------------------------------------------------

_PY_HANDLER_TYPE = "except_clause"
_TS_HANDLER_TYPE = "catch_clause"
_RETURN_TYPE = "return_statement"
_PY_RAISE_TYPE = "raise_statement"
_TS_RAISE_TYPE = "throw_statement"


def _walk(root: Node) -> Iterator[Node]:
    """Pre-order DFS over the subtree rooted at *root*."""
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.children))


def _count_in_handlers(
    root: Node,
    handler_type: str,
    raise_type: str,
) -> tuple[int, int]:
    """Return ``(returns, raises)`` inside all handler blocks under *root*.

    Descends the full tree looking for ``handler_type`` nodes (except_clause
    / catch_clause), then counts return and raise nodes inside each one.
    Nested handlers are walked independently.
    """
    returns = 0
    raises = 0
    for node in _walk(root):
        if node.type == handler_type:
            for child in _walk(node):
                if child.type == _RETURN_TYPE:
                    returns += 1
                elif child.type == raise_type:
                    raises += 1
    return returns, raises


def _ratio_for_source(source: str, language: Language) -> float | None:
    """Compute except-block ratio for a single source string, or None to skip.

    Returns None when the source has no return/raise inside handler blocks
    (ratio is mathematically undefined — the hunk/file is not exercising
    the exception-handling pattern this primitive measures).
    """
    parser = _PY_PARSER if language == "python" else _TS_PARSER
    handler_type = _PY_HANDLER_TYPE if language == "python" else _TS_HANDLER_TYPE
    raise_type = _PY_RAISE_TYPE if language == "python" else _TS_RAISE_TYPE
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return None
    returns, raises = _count_in_handlers(tree.root_node, handler_type, raise_type)
    total = returns + raises
    if total == 0:
        return None
    return returns / total


def _ratio_for_hunk(hunk_content: str) -> float | None:
    """Try Python grammar then TypeScript grammar; return ratio or None.

    The grammars are disjoint on handler-block node types (except_clause
    vs catch_clause), so probing both is safe: whichever grammar matches
    the hunk's syntax will find the relevant nodes; the other returns None.
    """
    ratio = _ratio_for_source(hunk_content, "python")
    if ratio is not None:
        return ratio
    return _ratio_for_source(hunk_content, "typescript")


# Minimum number of cluster files that must have a defined ratio before
# the cluster baseline is trusted.
_MIN_VALID_FILES = 3


# ---------------------------------------------------------------------------
# Primitive
# ---------------------------------------------------------------------------


@dataclass
class ExceptReturnRaiseRatio:
    """Except-block return/raise ratio primitive.

    Implements ``ShapePrimitive[_ExceptRatioBaseline]``.
    """

    name: str = "except_return_raise_ratio"
    min_cluster_size: int = 10
    cluster_bonus_clip: float = 5.0

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _ExceptRatioBaseline | None:
        """Compute mean ± std of per-file except-block ratios.

        Returns ``None`` if fewer than ``_MIN_VALID_FILES`` files have at
        least one return/raise inside a handler block (insufficient sample).
        """
        ratios: list[float] = []
        for _path, source in cluster_files:
            r = _ratio_for_source(source, language)
            if r is not None:
                ratios.append(r)
        if len(ratios) < _MIN_VALID_FILES:
            return None
        mean = statistics.mean(ratios)
        std = statistics.pstdev(ratios) if len(ratios) > 1 else 0.0
        return _ExceptRatioBaseline(mean=mean, std=std)

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _ExceptRatioBaseline | None,
        cluster_size: int,
    ) -> float:
        """Return tail-z contribution clipped to ``cluster_bonus_clip``.

        Abstains (returns 0.0) when:
        - ``baseline`` is None (cluster had insufficient sample),
        - ``cluster_size < self.min_cluster_size`` (size floor),
        - hunk has no return/raise inside handler blocks (math undefined).

        Ramp: ``min(clip, max(0.0, |tail_z| - 1.0))``.
        """
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0
        hunk_ratio = _ratio_for_hunk(hunk_content)
        if hunk_ratio is None:
            return 0.0
        tail_z = (hunk_ratio - baseline.mean) / max(baseline.std, 1e-6)
        return min(self.cluster_bonus_clip, max(0.0, abs(tail_z) - 1.0))


__all__ = ["ExceptReturnRaiseRatio"]
