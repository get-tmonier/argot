"""Typicality filter — AST-derived structural predicate.

Production-resident module (``engine/argot/scoring/filters/``).  Drops
structurally atypical hunks (data tables, generated boilerplate) from both
the calibration pool and inference-time control scoring.

Parses source directly with ``tree_sitter_python`` and
``tree_sitter_typescript``.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterator
from typing import Literal, NamedTuple

import tree_sitter_python as tspython
import tree_sitter_typescript as tstypescript
from tree_sitter import Language, Node
from tree_sitter import Parser as TsParser

Language_ = Literal["python", "typescript"]

_PY_LANGUAGE = Language(tspython.language())
_TS_LANGUAGE = Language(tstypescript.language_typescript())

# Module-level parsers — instantiated once and reused across all calls.
# Creating a TsParser per hunk causes linear memory growth (~30 GB on 150k+ hunks)
# because native byte buffers don't always get reclaimed between calls.
_PY_PARSER = TsParser(_PY_LANGUAGE)
_TS_PARSER = TsParser(_TS_LANGUAGE)

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

_TS_LITERAL_NODE_TYPES: frozenset[str] = frozenset(
    {
        "string",
        "template_string",
        "number",
        "true",
        "false",
        "null",
        "undefined",
        "regex",
    }
)

_TS_CONTROL_NODE_TYPES: frozenset[str] = frozenset(
    {
        "if_statement",
        "for_statement",
        "for_in_statement",
        "for_of_statement",
        "while_statement",
        "do_statement",
        "try_statement",
        "return_statement",
        "function_declaration",
        "function_expression",
        "arrow_function",
        "class_declaration",
        "method_definition",
        "throw_statement",
        "switch_statement",
    }
)

# Absolute cutoffs for the structural predicate.
# literal_leaf_ratio > 0.80: 4/5 AST leaves are literals — data-dominant by definition.
# named_leaf_count >= 5: size gate to avoid flagging tiny 1-2 entry constant definitions
#   (~2 named leaves). Lowered from 10 to catch 6-9 leaf small-fragment data hunks
#   (faker-js locale files). Break fixtures have low ratio regardless of size.
_LITERAL_RATIO_CUTOFF = 0.80
_NAMED_LEAF_COUNT_GATE = 5

# File-level fallback thresholds — applied when a hunk is typical at hunk-level
# but the containing file is data-dominant overall.
_FILE_LEVEL_MIN_LEAVES: int = 100
_FILE_LEVEL_MIN_RATIO: float = 0.80


class TypicalityFeatures(NamedTuple):
    """Five AST-derived features. All five are recorded for audit/debug;
    only ``literal_leaf_ratio`` and ``named_leaf_count`` gate the verdict."""

    literal_leaf_ratio: float
    control_node_density: float
    ast_type_entropy: float
    unique_token_ratio: float
    named_leaf_count: int


_NEUTRAL = TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


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


def _is_leaf_equivalent_generic(node: Node, literal_types: frozenset[str]) -> bool:
    """True for named nodes treated as atomic leaves for ratio computation."""
    if not node.is_named:
        return False
    if not node.children:
        return True
    return node.type in literal_types


def _compute_generic(
    source: str,
    parser: TsParser,
    literal_types: frozenset[str],
    control_types: frozenset[str],
) -> TypicalityFeatures:
    if not source.strip():
        return _NEUTRAL
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:
        return _NEUTRAL
    # No early bail on ERROR-root trees: tree-sitter preserves typed literal
    # nodes (string, number…) as children of ERROR subtrees. Mid-array fragments
    # lack their syntactic container so the whole tree is ERROR, but the leaf
    # literals are still countable. Genuinely unparseable content (e.g. `def ((((`)
    # produces no named leaves → named_leaf_count=0 → _NEUTRAL downstream.

    named_leaf_count = 0
    leaves_literal = 0
    node_type_counts: Counter[str] = Counter()
    control_nodes = 0
    token_counts: Counter[str] = Counter()

    for node in _walk_all_nodes(tree.root_node, literal_types):
        if node.is_named:
            node_type_counts[node.type] += 1
            if node.type in control_types:
                control_nodes += 1
        if _is_leaf_equivalent_generic(node, literal_types):
            named_leaf_count += 1
            if node.type in literal_types:
                leaves_literal += 1
            token_text = node.text.decode("utf-8", errors="replace") if node.text else ""
            token_counts[token_text] += 1

    del tree

    if named_leaf_count == 0:
        return _NEUTRAL

    total_lines = max(source.count("\n") + 1, 1)
    literal_leaf_ratio = leaves_literal / named_leaf_count
    control_node_density = (control_nodes / total_lines) * 100.0
    ast_type_entropy = _entropy(node_type_counts)
    total_tokens = sum(token_counts.values())
    unique_token_ratio = len(token_counts) / total_tokens if total_tokens else 0.0

    return TypicalityFeatures(
        literal_leaf_ratio=literal_leaf_ratio,
        control_node_density=control_node_density,
        ast_type_entropy=ast_type_entropy,
        unique_token_ratio=unique_token_ratio,
        named_leaf_count=named_leaf_count,
    )


def _compute_python(source: str) -> TypicalityFeatures:
    return _compute_generic(source, _PY_PARSER, _PY_LITERAL_NODE_TYPES, _PY_CONTROL_NODE_TYPES)


def _compute_typescript(source: str) -> TypicalityFeatures:
    return _compute_generic(source, _TS_PARSER, _TS_LITERAL_NODE_TYPES, _TS_CONTROL_NODE_TYPES)


def compute_features(source: str, language: Language_) -> TypicalityFeatures:
    """Compute 5 structural features for ``source`` in ``language``.

    Returns ``_NEUTRAL`` on parse error or empty input — a deliberately
    "near-typical" default so that downstream inference will not flag
    unparseable fragments as atypical.
    """
    if language == "python":
        return _compute_python(source)
    if language == "typescript":
        return _compute_typescript(source)
    raise ValueError(f"unsupported language: {language}")


class TypicalityModel:
    """Stateless typicality predicate — absolute thresholds + file-level fallback.

    Applied symmetrically at calibration (drop atypical hunks from the
    sampling pool) and inference (short-circuit atypical hunks before BPE
    scoring).  See ``docs/research/05-calibration-hygiene.md`` for the
    design rationale.
    """

    def __init__(self, language: Language_) -> None:
        self.language: Language_ = language

    def is_atypical(self, hunk: str) -> tuple[bool, TypicalityFeatures]:
        """Hunk-level check: returns ``(True, features)`` if structurally data-dominant."""
        features = compute_features(hunk, self.language)
        if features == _NEUTRAL:
            return False, features
        is_atyp = (
            features.named_leaf_count >= _NAMED_LEAF_COUNT_GATE
            and features.literal_leaf_ratio > _LITERAL_RATIO_CUTOFF
        )
        return is_atyp, features

    def is_atypical_file(self, file_source: str) -> tuple[bool, TypicalityFeatures]:
        """File-level check: stricter thresholds, used when hunk-level doesn't fire."""
        features = compute_features(file_source, self.language)
        if features == _NEUTRAL:
            return False, features
        is_atyp = (
            features.named_leaf_count >= _FILE_LEVEL_MIN_LEAVES
            and features.literal_leaf_ratio > _FILE_LEVEL_MIN_RATIO
        )
        return is_atyp, features


def language_for_adapter(adapter: object) -> Language_:
    """Derive a language string from a LanguageAdapter instance via file extensions."""
    exts = getattr(adapter, "file_extensions", frozenset())
    if ".py" in exts:
        return "python"
    if ".ts" in exts or ".tsx" in exts:
        return "typescript"
    raise ValueError(f"cannot infer language from adapter extensions: {exts!r}")
