"""Call-receiver scorer — Stage 1.5 (production port of era-6 research scorer).

Presence-based scorer: tracks distinct call-expression callees in the
model-A corpus and counts unattested callees in a hunk.  Used by
SequentialImportBpeScorer to apply a soft additive BPE penalty:

    adjusted_bpe = raw_bpe + alpha * min(count_unattested(hunk), cap)

Reuses module-level parsers from filters.typicality to avoid the linear
memory growth that occurs when TsParser is instantiated per-hunk.
"""

from __future__ import annotations

import math
from collections.abc import Iterator
from pathlib import Path
from typing import Literal, Protocol

from tree_sitter import Node

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER

Language = Literal["python", "typescript"]

_PY_CALL_TYPES: frozenset[str] = frozenset({"call"})
_PY_MEMBER_TYPES: frozenset[str] = frozenset({"attribute"})
_PY_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier"})

_TS_CALL_TYPES: frozenset[str] = frozenset({"call_expression", "new_expression"})
_TS_MEMBER_TYPES: frozenset[str] = frozenset({"member_expression"})
_TS_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier", "type_identifier"})


class _DataDominantAdapter(Protocol):
    def is_data_dominant(self, source: str, threshold: float = 0.65) -> bool: ...


def _walk_nodes(root: Node) -> Iterator[Node]:
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.children))


def _text(node: Node) -> str:
    return node.text.decode("utf-8", errors="replace") if node.text else ""


def _extract_python_callee(call_node: Node) -> str | None:
    callee = call_node.child_by_field_name("function")
    if callee is None:
        return None
    parts: list[str] = []
    while callee.type in _PY_MEMBER_TYPES:
        attr = callee.child_by_field_name("attribute")
        obj = callee.child_by_field_name("object")
        if attr is None or obj is None:
            return None
        parts.insert(0, _text(attr))
        callee = obj
    if callee.type in _PY_IDENTIFIER_TYPES:
        parts.insert(0, _text(callee))
        return ".".join(parts)
    if callee.type in _PY_CALL_TYPES:
        parts.insert(0, "<call>")
        return ".".join(parts)
    return None


def _extract_typescript_callee(call_node: Node) -> str | None:
    field_name = "constructor" if call_node.type == "new_expression" else "function"
    callee = call_node.child_by_field_name(field_name)
    if callee is None:
        return None
    parts: list[str] = []
    while callee.type in _TS_MEMBER_TYPES:
        prop = callee.child_by_field_name("property")
        obj = callee.child_by_field_name("object")
        if prop is None or obj is None:
            return None
        parts.insert(0, _text(prop))
        callee = obj
    if callee.type in _TS_IDENTIFIER_TYPES:
        parts.insert(0, _text(callee))
        return ".".join(parts)
    if callee.type in _TS_CALL_TYPES:
        parts.insert(0, "<call>")
        return ".".join(parts)
    return None


def _has_root_error(source: str, language: Language) -> bool:
    """Return True if any direct child of the parse tree root is an ERROR node.

    Hunk slices extracted out of file context (docstring bodies, method-shorthand
    definitions without their enclosing object literal) produce root-level ERROR
    nodes.  Callee extraction from such fragments is unreliable and should be
    skipped to avoid false positives.
    """
    parser = _PY_PARSER if language == "python" else _TS_PARSER
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return True
    has_error = any(child.type == "ERROR" for child in tree.root_node.children)
    del tree
    return has_error


def extract_callees(source: str, language: Language) -> list[str | None]:
    """Return dotted-callee signatures for every call-expression in *source*.

    Each call-expression maps to a dotted string (``"Math.random"``, ``"app.route"``,
    ``"fetch"``), a canonical ``"<call>.X"`` string when the callee chain root is itself
    a call expression (e.g., ``Router().route('/x')`` → ``"<call>.route"``), or ``None``
    when the callee bottoms out at a subscript or parenthesised expression.  ``None``
    entries are included for auditing but excluded from set membership.

    Returns ``[]`` on parse error or empty source.
    """
    if not source.strip():
        return []
    if language == "python":
        parser = _PY_PARSER
        call_types = _PY_CALL_TYPES
        extractor = _extract_python_callee
    elif language == "typescript":
        parser = _TS_PARSER
        call_types = _TS_CALL_TYPES
        extractor = _extract_typescript_callee
    else:
        raise ValueError(f"unsupported language: {language!r}")

    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return []
    out: list[str | None] = []
    for node in _walk_nodes(tree.root_node):
        if node.type in call_types:
            out.append(extractor(node))
    del tree
    return out


def callee_weight(
    c: str,
    attested_counts: dict[str, int],
    total_count: int,
    max_weight: float,
) -> float:
    """Per-callee penalty contribution, ∈ [0, max_weight].

    Unattested callees: max_weight (full penalty).
    Attested callees: smoothed -log(P(c)), capped at max_weight.
    Common attested callees get small weight; rare attested get moderate weight.
    """
    epsilon = 1.0
    vocab_size = len(attested_counts)
    denom = total_count + epsilon * (vocab_size + 1)
    p = (attested_counts[c] + epsilon) / denom if c in attested_counts else epsilon / denom
    return min(-math.log(p), max_weight)


class CallReceiverScorer:
    """Stage-1.5 call-receiver scorer.

    Fit: scan *model_a_files*, union all non-None callees into a frozenset.
    Score: count distinct unattested callees in a hunk (0 if parse fragment).
    Used by SequentialImportBpeScorer to compute adjusted_bpe.
    """

    def __init__(
        self,
        model_a_files: list[Path],
        *,
        language: Language,
        alpha: float = 1.0,
        cap: int = 5,
        adapter: _DataDominantAdapter | None = None,
    ) -> None:
        if not model_a_files:
            raise ValueError("model_a_files must be non-empty")
        self._language: Language = language
        self.alpha: float = alpha
        self.cap: int = cap
        attested_counts: dict[str, int] = {}
        skipped: int = 0
        for path in model_a_files:
            try:
                src = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if adapter is not None and adapter.is_data_dominant(src):
                skipped += 1
                continue
            for callee in extract_callees(src, language):
                if callee is not None:
                    attested_counts[callee] = attested_counts.get(callee, 0) + 1
        self.attested_counts: dict[str, int] = attested_counts
        self.total_count: int = sum(attested_counts.values())
        self.attested: frozenset[str] = frozenset(attested_counts.keys())
        self.attested_roots: frozenset[str] = frozenset(c.split(".", 1)[0] for c in self.attested)
        self.n_skipped_data_dominant: int = skipped

    def _get_distinct_unattested(self, hunk_content: str) -> list[str]:
        if _has_root_error(hunk_content, self._language):
            return []
        callees = extract_callees(hunk_content, self._language)
        seen: set[str] = set()
        deduped: list[str] = []
        for c in callees:
            if c is not None and c not in self.attested and c not in seen:
                seen.add(c)
                deduped.append(c)
        return deduped

    def count_unattested(self, hunk_content: str) -> int:
        """Return count of distinct unattested callees in *hunk_content*.

        Returns 0 if the hunk has root-level ERROR nodes (parse fragment).
        """
        return len(self._get_distinct_unattested(hunk_content))

    def weighted_contribution(
        self,
        hunk_content: str,
        *,
        alpha: float = 2.0,
        root_bonus: float = 2.0,
        cap: float = 5.0,
    ) -> float:
        """Return weighted penalty for unattested callees; 0.0 on parse fragment."""
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        weights: list[float] = []
        seen: set[str] = set()
        for c in callees:
            if c is None or c in seen:
                continue
            seen.add(c)
            if c in self.attested:
                continue
            root = c.split(".", 1)[0]
            if root in self.attested_roots:
                weights.append(alpha + root_bonus)
            else:
                weights.append(alpha)
        return min(sum(weights), cap)

    def fraction_weighted_contribution(
        self,
        hunk_content: str,
        *,
        max_weight: float,
        min_callees: int = 1,
    ) -> float:
        """Return per-hunk contribution proportional to fraction of unattested callees.

        Returns 0 if the hunk has root-level ERROR nodes, no callees, no unattested
        callees, or fewer than min_callees unique callees.
        """
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        unique_callees = {c for c in callees if c is not None}
        n_total = len(unique_callees)
        if n_total < min_callees:
            return 0.0
        n_unattested = sum(1 for c in unique_callees if c not in self.attested)
        if n_unattested == 0:
            return 0.0
        return max_weight * (n_unattested / n_total)

    def weighted_contribution_log(
        self,
        hunk_content: str,
        *,
        max_weight: float,
        cap: float,
    ) -> float:
        """Return log-frequency-weighted penalty; 0.0 on parse fragment.

        Each callee contributes callee_weight(c) — unattested callees receive max_weight
        (full penalty); attested callees receive smoothed -log(P(c)), capped at max_weight.
        Parked as private API — superseded by fraction_weighted_contribution in era-10 v2.
        """
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        seen: set[str] = set()
        weights: list[float] = []
        for c in callees:
            if c is None or c in seen:
                continue
            seen.add(c)
            w = callee_weight(c, self.attested_counts, self.total_count, max_weight)
            if w > 0:
                weights.append(w)
        return min(sum(weights), cap)
