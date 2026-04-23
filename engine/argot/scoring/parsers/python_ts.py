from __future__ import annotations

import tree_sitter_python as tspython
from tree_sitter import Language, Node
from tree_sitter import Parser as TsParser

PY_LANGUAGE = Language(tspython.language())

_IMPORT_QUERY = PY_LANGUAGE.query(
    "(import_statement name: (dotted_name) @imp) "
    "(import_from_statement module_name: (dotted_name) @imp)"
)

# `from __future__ import X` is a distinct node type — the module name is a keyword, not dotted_name
_FUTURE_IMPORT_QUERY = PY_LANGUAGE.query("(future_import_statement) @fi")

_STRING_QUERY = PY_LANGUAGE.query("(string) @s")
_COMMENT_QUERY = PY_LANGUAGE.query("(comment) @c")

_DOCSTRING_PARENT_TYPES = frozenset({"module", "class_definition", "function_definition"})

_BODY_TYPES = frozenset({"block", "module"})


def _is_docstring(node: Node) -> bool:
    parent = node.parent
    if parent is None:
        return False
    if parent.type in _DOCSTRING_PARENT_TYPES:
        body = next(
            (c for c in parent.children if c.type == "block"),
            parent if parent.type == "module" else None,
        )
        if body is None:
            return False
        stmts = [c for c in body.children if c.type == "expression_statement"]
        if not stmts:
            return False
        first_stmt = stmts[0]
        return bool(first_stmt.children and first_stmt.children[0].id == node.id)
    if parent.type == "expression_statement":
        grandparent = parent.parent
        if grandparent is None:
            return False
        if grandparent.type in _BODY_TYPES:
            stmts = [c for c in grandparent.children if c.type == "expression_statement"]
            if stmts and stmts[0].id == parent.id:
                return True
        if grandparent.type in _DOCSTRING_PARENT_TYPES:
            body = next(
                (c for c in grandparent.children if c.type == "block"),
                grandparent if grandparent.type == "module" else None,
            )
            if body is None:
                return False
            stmts = [c for c in body.children if c.type == "expression_statement"]
            if stmts and stmts[0].id == parent.id:
                return True
    return False


def _interpolation_rows(node: Node) -> frozenset[int]:
    rows: set[int] = set()
    for child in node.children:
        if child.type == "interpolation":
            rows.update(range(child.start_point[0] + 1, child.end_point[0] + 2))
        rows |= _interpolation_rows(child)
    return frozenset(rows)


class PythonTreeSitterParser:
    def __init__(self) -> None:
        self._parser = TsParser(PY_LANGUAGE)

    def extract_imports(self, src: str) -> frozenset[str]:
        try:
            tree = self._parser.parse(src.encode("utf-8"))
            if tree.root_node.has_error:
                return frozenset()
            captures: dict[str, list[Node]] = _IMPORT_QUERY.captures(tree.root_node)
            nodes = captures.get("imp", [])
            result: set[str] = {
                node.text.decode("utf-8").split(".")[0] for node in nodes if node.text
            }
            future_captures: dict[str, list[Node]] = _FUTURE_IMPORT_QUERY.captures(tree.root_node)
            if future_captures.get("fi"):
                result.add("__future__")
            return frozenset(result)
        except Exception:
            return frozenset()

    def prose_line_ranges(self, src: str) -> frozenset[int]:
        try:
            tree = self._parser.parse(src.encode("utf-8"))
            if tree.root_node.has_error:
                return frozenset()

            rows: set[int] = set()

            str_captures: dict[str, list[Node]] = _STRING_QUERY.captures(tree.root_node)
            for node in str_captures.get("s", []):
                start_row = node.start_point[0]
                end_row = node.end_point[0]
                is_multiline = end_row > start_row
                is_doc = _is_docstring(node)
                if not (is_multiline or is_doc):
                    continue
                line_range = set(range(start_row + 1, end_row + 2))
                interp_rows = _interpolation_rows(node)
                rows.update(line_range - interp_rows)

            comment_captures: dict[str, list[Node]] = _COMMENT_QUERY.captures(tree.root_node)
            for node in comment_captures.get("c", []):
                rows.add(node.start_point[0] + 1)

            return frozenset(rows)
        except Exception:
            return frozenset()
