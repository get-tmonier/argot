"""Fall-through-guard count primitive.

Scalar: count of ``if_statement`` nodes that appear (by source position)
before the first ``return_statement`` inside each ``function_definition`` body.
Per-cluster baseline is ``(mean, std)`` of the per-file average guard count
across cluster files.  Score-time contribution is a tail-z penalty clipped at
``cluster_bonus_clip``.

"Before return" definition:
  Within a ``function_definition`` subtree, count all ``if_statement`` nodes
  whose start_byte is strictly less than the start_byte of the earliest
  ``return_statement`` in that same subtree.  If the function contains no
  ``return_statement``, guard count is 0 (no guards before a return that
  does not exist).

Ramp curve:
  contribution = min(cluster_bonus_clip, max(0.0, abs(tail_z) - 1.0))
  Two-sided: catches average guard counts anomalously HIGH or LOW versus
  cluster baseline.  Ramps linearly from 0 at |z|=1σ up to cluster_bonus_clip
  at |z| = 1 + cluster_bonus_clip σ (=6σ for the default clip of 5.0).
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
class _GuardCountBaseline:
    mean: float
    std: float


# ---------------------------------------------------------------------------
# AST helpers
# ---------------------------------------------------------------------------

# tree-sitter node kind for function definitions (both grammars).
# Python: function_definition; TypeScript: function_declaration.
_PY_FUNC_TYPE = "function_definition"
_TS_FUNC_TYPE = "function_declaration"

_IF_TYPE = "if_statement"
_RETURN_TYPE = "return_statement"


def _walk(root: Node) -> Iterator[Node]:
    """Pre-order DFS over all nodes in the subtree rooted at *root*."""
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.children))


def _guards_before_return(func_node: Node) -> int:
    """Count ``if_statement`` nodes before the first ``return_statement`` in *func_node*.

    Returns 0 when the function contains no ``return_statement`` (no guards
    can precede a return that does not exist).
    """
    first_return_byte: int | None = None
    if_bytes: list[int] = []

    for node in _walk(func_node):
        if node.type == _RETURN_TYPE:
            if first_return_byte is None or node.start_byte < first_return_byte:
                first_return_byte = node.start_byte
        elif node.type == _IF_TYPE:
            if_bytes.append(node.start_byte)

    if first_return_byte is None:
        return 0
    return sum(1 for b in if_bytes if b < first_return_byte)


def _file_avg_guards(source: str, language: Language) -> float | None:
    """Return mean guard-count per function for *source*, or None if no functions."""
    parser = _PY_PARSER if language == "python" else _TS_PARSER
    func_type = _PY_FUNC_TYPE if language == "python" else _TS_FUNC_TYPE
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return None

    guard_counts: list[int] = []
    for node in _walk(tree.root_node):
        if node.type == func_type:
            guard_counts.append(_guards_before_return(node))

    del tree
    if not guard_counts:
        return None
    return statistics.mean(guard_counts)


# ---------------------------------------------------------------------------
# Primitive
# ---------------------------------------------------------------------------

_MIN_VALID_FILES = 3


@dataclass
class FallThroughGuards:
    """Fall-through-guard count primitive.

    Implements ``ShapePrimitive[_GuardCountBaseline]``.
    """

    name: str = "fall_through_guards"
    min_cluster_size: int = 10
    cluster_bonus_clip: float = 5.0

    # ------------------------------------------------------------------
    # Fit
    # ------------------------------------------------------------------

    def fit_cluster_baseline(
        self,
        cluster_files: Iterable[tuple[Path, str]],
        language: Language,
    ) -> _GuardCountBaseline | None:
        """Compute mean ± std of per-file average guard count across cluster files.

        Returns ``None`` if fewer than ``_MIN_VALID_FILES`` files have at least
        one ``function_definition`` (insufficient sample).
        """
        avgs: list[float] = []
        for _path, source in cluster_files:
            avg = _file_avg_guards(source, language)
            if avg is not None:
                avgs.append(avg)
        if len(avgs) < _MIN_VALID_FILES:
            return None
        mean = statistics.mean(avgs)
        std = statistics.pstdev(avgs) if len(avgs) > 1 else 0.0
        return _GuardCountBaseline(mean=mean, std=std)

    # ------------------------------------------------------------------
    # Score
    # ------------------------------------------------------------------

    def score(
        self,
        hunk_content: str,
        *,
        baseline: _GuardCountBaseline | None,
        cluster_size: int,
    ) -> float:
        """Return tail-z contribution clipped to ``cluster_bonus_clip``.

        Abstains (returns 0.0) when:
        - ``baseline`` is None (cluster had insufficient sample),
        - ``cluster_size < self.min_cluster_size`` (floor),
        - hunk has 0 ``function_definition`` nodes (shape undefined).

        Ramp: ``min(clip, max(0.0, |tail_z| - 1.0))`` — two-sided,
        linearly rises from 0 at 1σ to clip at (1 + clip)σ.
        """
        if baseline is None or cluster_size < self.min_cluster_size:
            return 0.0
        # Try the language inferred from the corpus parser; fall back to the
        # other grammar so the primitive works on mixed-language hunks.
        hunk_avg = _score_hunk_avg(hunk_content)
        if hunk_avg is None:
            return 0.0
        tail_z = (hunk_avg - baseline.mean) / max(baseline.std, 1e-6)
        return min(self.cluster_bonus_clip, max(0.0, abs(tail_z) - 1.0))


def _score_hunk_avg(hunk_content: str) -> float | None:
    """Try python then typescript grammar; return per-function avg or None."""
    for language in ("python", "typescript"):
        avg = _file_avg_guards(hunk_content, language)  # type: ignore[arg-type]
        if avg is not None:
            return avg
    return None


__all__ = ["FallThroughGuards"]
