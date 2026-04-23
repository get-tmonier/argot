"""Typicality filter — AST-derived structural predicate.

Used inside the benchmark sandbox to drop structurally atypical hunks
(data tables, generated boilerplate, assertion-heavy tests) from both
the calibration pool and inference-time control scoring.

No imports from ``engine/argot/scoring/``. Parses source directly with
``tree_sitter_python`` and ``tree_sitter_typescript``.
"""

from __future__ import annotations

from typing import Literal, NamedTuple

Language = Literal["python", "typescript"]


class TypicalityFeatures(NamedTuple):
    """Four AST-derived features used by the typicality predicate."""

    literal_leaf_ratio: float
    control_node_density: float
    ast_type_entropy: float
    unique_token_ratio: float


def compute_features(source: str, language: Language) -> TypicalityFeatures:
    """Compute 4 structural features for ``source`` in ``language``.

    Returns ``TypicalityFeatures(0, 0, 0, 0)`` on parse error or empty input —
    a deliberately "near-typical" default so that downstream Mahalanobis
    distance will not flag unparseable fragments as atypical.
    """
    raise NotImplementedError
