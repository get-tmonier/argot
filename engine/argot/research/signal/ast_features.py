"""AST structural feature extraction — fully automatic, corpus-derived.

Walks every AST node and emits (NodeClassName, dotted_name) for any field
that resolves to a dotted identifier. No pre-defined categories; any node
type with an identifier field contributes automatically.

Returns dict[node_class_name -> list[dotted_name_str]].
"""

from __future__ import annotations

import ast


def _dotted_name(node: ast.expr) -> str | None:
    """Return dotted string for a Name/Attribute chain, or None if ambiguous."""
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        base = _dotted_name(node.value)
        if base is None:
            return None
        return f"{base}.{node.attr}"
    return None


_SCOPE_TYPES = (
    ast.Module,
    ast.FunctionDef,
    ast.AsyncFunctionDef,
    ast.ClassDef,
    ast.If,
    ast.For,
    ast.While,
    ast.Try,
)


def _nearest_scope(node: ast.AST, parent_map: dict[int, ast.AST]) -> str:
    """Walk parent_map upward and return the type name of the nearest scope-ish node."""
    current = parent_map.get(id(node))
    while current is not None:
        if isinstance(current, _SCOPE_TYPES):
            return type(current).__name__
        current = parent_map.get(id(current))
    return "Module"


_COOC_SCOPE_TYPES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)
_COOC_K = 20


def _collect_scope_features(
    scope_node: ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef,
    parent_context: bool,
    parent_map: dict[int, ast.AST],
) -> list[tuple[str, str]]:
    """Collect up to K (category, value) feature pairs from a scope body."""
    features: list[tuple[str, str]] = []
    for node in ast.walk(scope_node):
        if len(features) >= _COOC_K:
            break
        node_class = type(node).__name__
        if parent_context:
            scope = _nearest_scope(node, parent_map)
            category = f"{scope}::{node_class}"
        else:
            category = node_class
        for _, field_value in ast.iter_fields(node):
            if len(features) >= _COOC_K:
                break
            candidates = field_value if isinstance(field_value, list) else [field_value]
            for candidate in candidates:
                if len(features) >= _COOC_K:
                    break
                if isinstance(candidate, ast.expr):
                    name = _dotted_name(candidate)
                    if name is not None:
                        features.append((category, name))
    return features


def extract_features(
    source: str, *, parent_context: bool = False, cooccurrence: bool = False
) -> dict[str, list[str]]:
    """Parse *source* and return structural features keyed by AST node class name.

    When *parent_context* is True, each feature category is prefixed with the
    nearest enclosing scope type (e.g. ``"AsyncFunctionDef::Raise"``).

    When *cooccurrence* is True, for each function/class body emit cross-category
    feature pairs under the ``"cooccur"`` key. Only pairs with different category
    keys are emitted; at most K=20 features per scope are considered.

    Returns an empty dict on SyntaxError.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return {}

    parent_map: dict[int, ast.AST] = {}
    if parent_context or cooccurrence:
        for node in ast.walk(tree):
            for child in ast.iter_child_nodes(node):
                parent_map[id(child)] = node

    result: dict[str, list[str]] = {}
    for node in ast.walk(tree):
        node_class = type(node).__name__
        if parent_context:
            scope = _nearest_scope(node, parent_map)
            category = f"{scope}::{node_class}"
            # Ensure every node creates a category entry so structural presence
            # is always recorded, even when no dotted-name fields resolve.
            result.setdefault(category, [])
        else:
            category = node_class
        for _, field_value in ast.iter_fields(node):
            candidates = field_value if isinstance(field_value, list) else [field_value]
            for candidate in candidates:
                if isinstance(candidate, ast.expr):
                    name = _dotted_name(candidate)
                    if name is not None:
                        result.setdefault(category, []).append(name)

    if cooccurrence:
        for node in ast.walk(tree):
            if isinstance(node, _COOC_SCOPE_TYPES):
                scope_feats = _collect_scope_features(node, parent_context, parent_map)
                for i in range(len(scope_feats)):
                    cat_a, val_a = scope_feats[i]
                    for j in range(i + 1, len(scope_feats)):
                        cat_b, val_b = scope_feats[j]
                        if cat_a == cat_b:
                            continue
                        pair = "|".join(sorted([f"{cat_a}:{val_a}", f"{cat_b}:{val_b}"]))
                        result.setdefault("cooccur", []).append(pair)

    return result
