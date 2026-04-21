from __future__ import annotations

import ast

_LineNode = ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef
_ScopeNode = ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef | ast.Module
_LINE_TYPES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)
_SCOPE_TYPES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)
_CHAR_BUDGET = 2000
_HALF_BUDGET = _CHAR_BUDGET // 2


def build_context(
    source: str,
    hunk_start_line: int,
    hunk_end_line: int,
    mode: str,
) -> list[dict[str, str]]:
    if mode == "baseline":
        return _baseline(source, hunk_start_line)

    lines = source.splitlines()
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return _baseline(source, hunk_start_line)

    parent_map: dict[int, ast.AST] = {}
    for node in ast.walk(tree):
        for child in ast.iter_child_nodes(node):
            parent_map[id(child)] = node

    if mode == "parent_only":
        return _parent_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)

    if mode == "file_only":
        result_lines = _splice_hunk(lines, hunk_start_line, hunk_end_line)
        return _truncate(_to_dicts(result_lines), hunk_start_line, lines)

    if mode == "siblings_only":
        return _siblings_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)

    if mode == "combined":
        po = _parent_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)
        fo = _truncate(
            _to_dicts(_splice_hunk(lines, hunk_start_line, hunk_end_line)), hunk_start_line, lines
        )
        so = _siblings_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)
        sep: dict[str, str] = {"text": "---"}
        return po + [sep] + fo + [sep] + so

    raise ValueError(f"Unknown mode: {mode!r}")


def _baseline(source: str, hunk_start_line: int) -> list[dict[str, str]]:
    lines = source.splitlines()
    end = hunk_start_line - 1
    start = max(0, end - 20)
    return [{"text": line} for line in lines[start:end]]


def _splice_hunk(lines: list[str], hunk_start_line: int, hunk_end_line: int) -> list[str]:
    before = lines[: hunk_start_line - 1]
    after = lines[hunk_end_line:]
    return before + [""] + after


def _to_dicts(lines: list[str]) -> list[dict[str, str]]:
    return [{"text": line} for line in lines]


def _truncate(
    dicts: list[dict[str, str]],
    hunk_start_line: int,
    original_lines: list[str],
) -> list[dict[str, str]]:
    total_chars = sum(len(d["text"]) for d in dicts)
    if total_chars <= _CHAR_BUDGET:
        return dicts

    hunk_idx = hunk_start_line - 1
    chars_before = 0
    pivot = 0
    for i, d in enumerate(dicts):
        if i >= hunk_idx:
            pivot = i
            break
        chars_before += len(d["text"])
    else:
        pivot = len(dicts)

    before_dicts: list[dict[str, str]] = []
    after_dicts: list[dict[str, str]] = []
    used_before = 0
    for d in reversed(dicts[:pivot]):
        n = len(d["text"])
        if used_before + n > _HALF_BUDGET:
            break
        before_dicts.insert(0, d)
        used_before += n

    used_after = 0
    for d in dicts[pivot:]:
        n = len(d["text"])
        if used_after + n > _HALF_BUDGET:
            break
        after_dicts.append(d)
        used_after += n

    return before_dicts + after_dicts


def _node_span(node: _ScopeNode, total_lines: int) -> tuple[int, int]:
    if isinstance(node, ast.Module):
        return 1, total_lines
    end = node.end_lineno if node.end_lineno is not None else total_lines
    return node.lineno, end


def _find_enclosing(
    tree: ast.AST,
    hunk_start_line: int,
    hunk_end_line: int,
    total_lines: int,
) -> _ScopeNode | None:
    best: _ScopeNode | None = None
    best_span = float("inf")
    for node in ast.walk(tree):
        if not isinstance(node, _SCOPE_TYPES):
            continue
        node_start, node_end = _node_span(node, total_lines)
        if node_start <= hunk_start_line and node_end >= hunk_end_line:
            span = node_end - node_start
            if span < best_span:
                best_span = span
                best = node
    return best


def _parent_only(
    lines: list[str],
    hunk_start_line: int,
    hunk_end_line: int,
    tree: ast.AST,
    parent_map: dict[int, ast.AST],
    source: str,
) -> list[dict[str, str]]:
    node = _find_enclosing(tree, hunk_start_line, hunk_end_line, len(lines))
    if node is None:
        return _baseline(source, hunk_start_line)

    node_start, node_end = _node_span(node, len(lines))
    node_lines = lines[node_start - 1 : node_end]
    local_hunk_start = hunk_start_line - node_start
    local_hunk_end = hunk_end_line - node_start + 1
    spliced = node_lines[:local_hunk_start] + [""] + node_lines[local_hunk_end:]
    return _truncate(_to_dicts(spliced), hunk_start_line - node_start + 1, node_lines)


def _siblings_only(
    lines: list[str],
    hunk_start_line: int,
    hunk_end_line: int,
    tree: ast.AST,
    parent_map: dict[int, ast.AST],
    source: str,
) -> list[dict[str, str]]:
    enclosing = _find_enclosing(tree, hunk_start_line, hunk_end_line, len(lines))
    if enclosing is None:
        return _parent_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)

    direct_parent = parent_map.get(id(enclosing))
    if direct_parent is None:
        return _parent_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)

    line_siblings: list[_LineNode] = [
        child
        for child in ast.iter_child_nodes(direct_parent)
        if child is not enclosing and isinstance(child, _LINE_TYPES)
    ]
    if not line_siblings:
        return _parent_only(lines, hunk_start_line, hunk_end_line, tree, parent_map, source)

    sibling_lines: list[str] = []
    for sib in line_siblings:
        sibling_lines.extend(lines[sib.lineno - 1 : sib.end_lineno])

    return _truncate(_to_dicts(sibling_lines), 1, sibling_lines)
