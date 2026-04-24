"""Call-receiver scorer (era 6, research phase).

Presence-based Stage 1.5 predicate: flags hunks that introduce
call-expression receivers (full dotted callee strings) absent from the
repo's own call sites. Lives inside the benchmark sandbox; production
scorer in ``engine/argot/scoring/`` is untouched on this branch.

See docs/superpowers/specs/2026-04-24-era6-call-receiver.md for design.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import tree_sitter_python as tspython
import tree_sitter_typescript as tstypescript
from tree_sitter import Language as TsLanguage
from tree_sitter import Node
from tree_sitter import Parser as TsParser

_PY_LANGUAGE = TsLanguage(tspython.language())
_TS_LANGUAGE = TsLanguage(tstypescript.language_typescript())
_PY_PARSER = TsParser(_PY_LANGUAGE)
_TS_PARSER = TsParser(_TS_LANGUAGE)

_PY_CALL_TYPES: frozenset[str] = frozenset({"call"})
_PY_MEMBER_TYPES: frozenset[str] = frozenset({"attribute"})
_PY_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier"})

_TS_CALL_TYPES: frozenset[str] = frozenset({"call_expression", "new_expression"})
_TS_MEMBER_TYPES: frozenset[str] = frozenset({"member_expression"})
_TS_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier", "type_identifier"})

Language = Literal["python", "typescript"]


def _walk_nodes(root: Node):  # noqa: ANN201
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
    return None


def extract_callees(source: str, language: Language) -> list[str | None]:
    """Return dotted-callee signatures for every call-expression in *source*.

    Each call-expression maps to either a dotted string (``"Math.random"``,
    ``"app.route"``, ``"fetch"``) or ``None`` when the callee bottoms out
    at a non-identifier (another call, subscript, parenthesized expression).
    ``None`` entries are counted for auditing but excluded from set membership.

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
        raise ValueError(f"unsupported language: {language}")

    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:
        return []
    root = tree.root_node
    out: list[str | None] = []
    for node in _walk_nodes(root):
        if node.type in call_types:
            out.append(extractor(node))
    del tree
    return out


@dataclass(frozen=True)
class CallReceiverResult:
    """Result of scoring a hunk's call-expression receivers against the attested set."""

    unattested: tuple[str, ...]
    flagged: bool


class CallReceiverScorer:
    """Stage-1.5 presence-based scorer.

    Fit: scan *model_a_files*, union all non-None callees into a frozenset.
    Score: extract callees from a hunk, flag if ``len(unattested) >= k``.
    """

    def __init__(
        self,
        model_a_files: list[Path],
        *,
        language: Language,
        k: int = 1,
    ) -> None:
        if not model_a_files:
            raise ValueError("model_a_files must be non-empty")
        if k < 1:
            raise ValueError(f"k must be >= 1, got {k}")
        self._language: Language = language
        self._k: int = k
        attested: set[str] = set()
        for path in model_a_files:
            try:
                src = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for callee in extract_callees(src, language):
                if callee is not None:
                    attested.add(callee)
        self.attested: frozenset[str] = frozenset(attested)

    def score_hunk(self, hunk_content: str) -> CallReceiverResult:
        raise NotImplementedError
