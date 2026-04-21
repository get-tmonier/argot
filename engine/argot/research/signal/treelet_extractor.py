from __future__ import annotations

import ast


def extract_treelets(source: str) -> list[str]:
    """Parse *source* and return depth-1, depth-2, and depth-3 treelet strings.

    Each treelet encodes only AST node type names — no identifier strings.
    Returns [] on SyntaxError.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []

    treelets: list[str] = []
    for parent in ast.walk(tree):
        p_name = type(parent).__name__
        for child in ast.iter_child_nodes(parent):
            c_name = type(child).__name__
            treelets.append(f"d1:{p_name}>{c_name}")
            for grandchild in ast.iter_child_nodes(child):
                g_name = type(grandchild).__name__
                treelets.append(f"d2:{p_name}>{c_name}>{g_name}")
                for ggchild in ast.iter_child_nodes(grandchild):
                    gg_name = type(ggchild).__name__
                    treelets.append(f"d3:{p_name}>{c_name}>{g_name}>{gg_name}")

    return treelets
