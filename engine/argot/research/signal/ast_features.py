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


def extract_features(source: str) -> dict[str, list[str]]:
    """Parse *source* and return structural features keyed by AST node class name.

    Returns an empty dict on SyntaxError.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return {}

    result: dict[str, list[str]] = {}
    for node in ast.walk(tree):
        category = type(node).__name__
        for _, field_value in ast.iter_fields(node):
            candidates = field_value if isinstance(field_value, list) else [field_value]
            for candidate in candidates:
                if isinstance(candidate, ast.expr):
                    name = _dotted_name(candidate)
                    if name is not None:
                        result.setdefault(category, []).append(name)
    return result
