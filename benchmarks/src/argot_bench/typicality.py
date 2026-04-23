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

import numpy as np
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


class TypicalityFeatures(NamedTuple):
    """Four AST-derived features used by the typicality predicate."""

    literal_leaf_ratio: float
    control_node_density: float
    ast_type_entropy: float
    unique_token_ratio: float


_NEUTRAL = TypicalityFeatures(0.0, 0.0, 0.0, 0.0)

# Minimum slack per feature for one-sided percentile bounds, so the
# threshold never collapses to a point when the pool is homogeneous.
# Order: literal_leaf_ratio, control_node_density, ast_type_entropy, unique_token_ratio.
_FALLBACK_MIN_SLACK: tuple[float, float, float, float] = (0.3, 1.0, 0.5, 0.2)


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
    if tree.root_node.has_error and all(c.type == "ERROR" for c in tree.root_node.children if c.is_named):
        del tree
        return _NEUTRAL

    leaves_total = 0
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
            leaves_total += 1
            if node.type in literal_types:
                leaves_literal += 1
            token_text = node.text.decode("utf-8", errors="replace") if node.text else ""
            token_counts[token_text] += 1

    del tree

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


def _compute_python(source: str) -> TypicalityFeatures:
    return _compute_generic(source, _PY_PARSER, _PY_LITERAL_NODE_TYPES, _PY_CONTROL_NODE_TYPES)


def _compute_typescript(source: str) -> TypicalityFeatures:
    return _compute_generic(source, _TS_PARSER, _TS_LITERAL_NODE_TYPES, _TS_CONTROL_NODE_TYPES)


def compute_features(source: str, language: Language_) -> TypicalityFeatures:
    """Compute 4 structural features for ``source`` in ``language``.

    Returns ``TypicalityFeatures(0, 0, 0, 0)`` on parse error or empty input —
    a deliberately "near-typical" default so that downstream inference
    will not flag unparseable fragments as atypical.
    """
    if language == "python":
        return _compute_python(source)
    if language == "typescript":
        return _compute_typescript(source)
    raise ValueError(f"unsupported language: {language}")


def _compute_fallback_bounds(vectors: np.ndarray) -> dict[str, tuple[float, float]]:
    """Per-feature one-sided p1/p99 bounds for the typicality predicate."""
    if vectors.shape[0] == 0:
        return {
            "literal_leaf_ratio": (-math.inf, math.inf),
            "control_node_density": (-math.inf, math.inf),
            "ast_type_entropy": (-math.inf, math.inf),
            "unique_token_ratio": (-math.inf, math.inf),
        }
    p1 = np.percentile(vectors, 1, axis=0)
    p99 = np.percentile(vectors, 99, axis=0)
    s = _FALLBACK_MIN_SLACK
    return {
        # Bounds stored in raw feature space (no log transform — no MCD).
        # high literal ratio = atypical (data-dense); lower bound unused
        "literal_leaf_ratio": (-math.inf, float(p99[0]) + s[0]),
        # low control density = atypical (boilerplate/data); upper bound unused
        "control_node_density": (max(0.0, float(p1[1]) - s[1]), math.inf),
        # low entropy = atypical (repetitive)
        "ast_type_entropy": (max(0.0, float(p1[2]) - s[2]), math.inf),
        # low unique-token ratio = atypical (repetitive tokens)
        "unique_token_ratio": (max(0.0, float(p1[3]) - s[3]), math.inf),
    }


def _predict_one_sided(
    features: TypicalityFeatures, bounds: dict[str, tuple[float, float]]
) -> bool:
    """Return True if any feature is outside its one-sided normal range."""
    if features.literal_leaf_ratio > bounds["literal_leaf_ratio"][1]:
        return True
    if features.control_node_density < bounds["control_node_density"][0]:
        return True
    if features.ast_type_entropy < bounds["ast_type_entropy"][0]:
        return True
    return features.unique_token_ratio < bounds["unique_token_ratio"][0]


class TypicalityModel:
    """One-sided percentile outlier model fit on a corpus candidate pool.

    Flags hunks where any feature falls outside its normal range in the
    direction that indicates structural atypicality:

    - ``literal_leaf_ratio > p99`` — data-heavy (long literal tables)
    - ``control_node_density < p1`` — boilerplate / pure data
    - ``ast_type_entropy < p1`` — repetitive structure
    - ``unique_token_ratio < p1`` — repetitive tokens

    OR over the four features. Applied symmetrically at calibration
    (pool hygiene) and inference (control scoring short-circuit).
    """

    def __init__(self, language: Language_) -> None:
        self.language: Language_ = language
        self._fitted: bool = False
        self._fallback_bounds: dict[str, tuple[float, float]] | None = None

    def fit(self, pool: list[str]) -> None:
        """Fit the outlier model on ``pool`` — list of hunk source strings."""
        features_list = [compute_features(h, self.language) for h in pool]
        vectors = np.array(
            [list(f) for f in features_list if f != _NEUTRAL],
            dtype=np.float64,
        )
        self._fallback_bounds = _compute_fallback_bounds(vectors)
        self._fitted = True

    def is_atypical(self, hunk: str) -> tuple[bool, float, TypicalityFeatures]:
        """Decide whether ``hunk`` is structurally atypical.

        Returns ``(is_atypical, distance, features)``. ``distance`` is always
        0.0 — retained for a stable downstream JSON shape. For parse errors /
        empty input the features are the neutral zero tuple and
        ``is_atypical`` is False (never filter what we couldn't parse).
        """
        if not self._fitted:
            raise RuntimeError("TypicalityModel.fit() must be called first")

        features = compute_features(hunk, self.language)
        if features == _NEUTRAL:
            return False, 0.0, features

        assert self._fallback_bounds is not None
        return _predict_one_sided(features, self._fallback_bounds), 0.0, features
