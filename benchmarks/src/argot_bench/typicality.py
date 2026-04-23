"""Typicality filter — AST-derived structural predicate.

Used inside the benchmark sandbox to drop structurally atypical hunks
(data tables, generated boilerplate, assertion-heavy tests) from both
the calibration pool and inference-time control scoring.

No imports from ``engine/argot/scoring/``. Parses source directly with
``tree_sitter_python`` and ``tree_sitter_typescript``.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterator
from typing import Literal, NamedTuple

import tree_sitter_python as tspython
from tree_sitter import Language, Node
from tree_sitter import Parser as TsParser

Language_ = Literal["python", "typescript"]

_PY_LANGUAGE = Language(tspython.language())

# Node types per language. Values are tree-sitter grammar node-type strings.
_PY_LITERAL_NODE_TYPES: frozenset[str] = frozenset(
    {
        "string",
        "concatenated_string",
        "integer",
        "float",
        "true",
        "false",
        "none",
    }
)

_PY_CONTROL_NODE_TYPES: frozenset[str] = frozenset(
    {
        "if_statement",
        "for_statement",
        "while_statement",
        "try_statement",
        "return_statement",
        "function_definition",
        "async_function_definition",
        "class_definition",
        "with_statement",
        "raise_statement",
    }
)


class TypicalityFeatures(NamedTuple):
    """Four AST-derived features used by the typicality predicate."""

    literal_leaf_ratio: float
    control_node_density: float
    ast_type_entropy: float
    unique_token_ratio: float


_NEUTRAL = TypicalityFeatures(0.0, 0.0, 0.0, 0.0)


def _walk_all_nodes(root: Node, atomic_types: frozenset[str] = frozenset()) -> Iterator[Node]:
    """Depth-first iterator over every node in the tree (named + anonymous).

    When ``atomic_types`` is provided, does not descend into nodes whose type
    is in that set — their children are grammar-internal detail (e.g. tree-sitter
    Python wraps string literals in string_start/string_content/string_end).
    """
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        if node.type not in atomic_types:
            stack.extend(reversed(node.children))


def _is_leaf_equivalent(node: Node) -> bool:
    """True for named nodes treated as atomic leaves for ratio computation.

    Covers genuinely childless named nodes *and* string nodes whose children
    are purely structural (string_start / string_content / string_end) — the
    tree-sitter Python grammar wraps string literals in child nodes that are
    implementation detail, not semantic children.
    """
    if not node.is_named:
        return False
    if not node.children:
        return True
    # string / concatenated_string are atomic literals even with children
    if node.type in _PY_LITERAL_NODE_TYPES:
        return True
    return False


def _compute_python(source: str) -> TypicalityFeatures:
    if not source.strip():
        return _NEUTRAL
    try:
        parser = TsParser(_PY_LANGUAGE)
        tree = parser.parse(source.encode("utf-8"))
    except Exception:
        return _NEUTRAL
    # tree-sitter always returns a module root; use has_error to detect parse failures
    if tree.root_node.has_error and all(
        c.type == "ERROR" for c in tree.root_node.children if c.is_named
    ):
        return _NEUTRAL

    leaves_total = 0
    leaves_literal = 0
    node_type_counts: Counter[str] = Counter()
    control_nodes = 0
    token_counts: Counter[str] = Counter()

    for node in _walk_all_nodes(tree.root_node, _PY_LITERAL_NODE_TYPES):
        if node.is_named:
            node_type_counts[node.type] += 1
            if node.type in _PY_CONTROL_NODE_TYPES:
                control_nodes += 1
        # Leaves are nodes with no children (or atomic literals like strings);
        # count only named ones to exclude punctuation tokens like "(" ")" "," etc.
        if _is_leaf_equivalent(node):
            leaves_total += 1
            if node.type in _PY_LITERAL_NODE_TYPES:
                leaves_literal += 1
            token_text = node.text.decode("utf-8", errors="replace") if node.text else ""
            token_counts[token_text] += 1

    total_lines = max(source.count("\n") + 1, 1)
    literal_leaf_ratio = leaves_literal / leaves_total if leaves_total else 0.0
    control_node_density = (control_nodes / total_lines) * 100.0
    ast_type_entropy = _entropy(node_type_counts)
    total_tokens = sum(token_counts.values())
    unique_token_ratio = len(token_counts) / total_tokens if total_tokens else 0.0

    return TypicalityFeatures(
        literal_leaf_ratio=literal_leaf_ratio,
        control_node_density=control_node_density,
        ast_type_entropy=ast_type_entropy,
        unique_token_ratio=unique_token_ratio,
    )


def _entropy(counts: Counter[str]) -> float:
    total = sum(counts.values())
    if total == 0:
        return 0.0
    h = 0.0
    for c in counts.values():
        p = c / total
        if p > 0:
            h -= p * math.log2(p)
    return h


def compute_features(source: str, language: Language_) -> TypicalityFeatures:
    """Compute 4 structural features for ``source`` in ``language``.

    Returns ``TypicalityFeatures(0, 0, 0, 0)`` on parse error or empty input —
    a deliberately "near-typical" default so that downstream Mahalanobis
    distance will not flag unparseable fragments as atypical.
    """
    if language == "python":
        return _compute_python(source)
    raise NotImplementedError(f"TypeScript support lands in Task 3 (got language={language})")
