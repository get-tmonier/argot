# engine/argot/research/signal/phase14/filters/data_dominant.py
"""Data-dominant file detector — tree-sitter structural heuristic, no I/O.

.. deprecated::
    Direct use of this module is deprecated.  Use
    ``argot.research.signal.phase14.adapters.PythonAdapter.is_data_dominant``
    instead.  This module is kept as a thin shim because external callers
    (validation scripts, experiments) may still import it directly.
"""

from __future__ import annotations

import tree_sitter_python as tspython
from tree_sitter import Language, Node
from tree_sitter import Parser as TsParser

_PY_LANGUAGE = Language(tspython.language())
_PARSER = TsParser(_PY_LANGUAGE)

# RHS node types that count as "static data literals"
_DATA_LITERAL_TYPES: frozenset[str] = frozenset({"list", "tuple", "dictionary", "set"})


def _get_rhs(node: Node) -> Node | None:
    """Return the right-hand-side child of an assignment node, or None."""
    # tree-sitter Python: assignment → left, '=', right
    # The right-hand side is the last named child when there are named children.
    # Walk children looking for a node that comes after '='
    found_eq = False
    for child in node.children:
        if found_eq:
            return child
        if child.type == "=":
            found_eq = True
    return None


def _collect_stmt_data_rows(stmts: list[Node], rows: set[int]) -> None:
    """Scan a list of sibling statement nodes and add data-literal spans to *rows*."""
    for stmt in stmts:
        target: Node | None = None

        if stmt.type == "assignment":
            target = stmt
        elif stmt.type == "expression_statement":
            for sub in stmt.children:
                if sub.type == "assignment":
                    target = sub
                    break

        if target is None:
            continue

        rhs = _get_rhs(target)
        if rhs is not None and rhs.type in _DATA_LITERAL_TYPES:
            rows.update(range(stmt.start_point[0], stmt.end_point[0] + 1))


def _extract_data_literal_lines(root: Node) -> frozenset[int]:
    """Return 0-indexed row numbers covered by data literal assignments.

    Scans both module-level statements and class-body statements so that
    locale-style provider files (e.g. ``class Provider: cities = (...)`` ) are
    correctly detected as data-dominant even when the data lives in a class body.
    """
    rows: set[int] = set()

    _collect_stmt_data_rows(root.children, rows)

    # Descend into top-level class definitions to catch class-body data tables.
    for child in root.children:
        if child.type == "class_definition":
            for sub in child.children:
                if sub.type == "block":
                    _collect_stmt_data_rows(sub.children, rows)

    return frozenset(rows)


def is_data_dominant(file_source: str, threshold: float = 0.65) -> bool:
    """Return True if the file is overwhelmingly composed of top-level data literal assignments.

    The heuristic parses the file with tree-sitter and walks module-level statements
    as well as class-body statements inside top-level class definitions.  Any
    ``assignment`` whose right-hand side is a ``list``, ``tuple``, ``dictionary``,
    or ``set`` literal contributes its line span to *data_literal_lines*.  The ratio::

        data_literal_lines / total_nonblank_lines

    is compared against *threshold* (default 0.65).  Files with a parse error or
    empty input return False.

    Args:
        file_source: Full file content as a string.
        threshold: Fraction above which the file is considered data-dominant.

    Returns:
        True if the data-literal ratio exceeds *threshold*, False otherwise.
    """
    if not file_source or not file_source.strip():
        return False

    try:
        tree = _PARSER.parse(file_source.encode("utf-8"))
    except Exception:
        return False

    if tree.root_node.has_error:
        # Tolerate minor errors — the heuristic degrades gracefully; but if the
        # tree is entirely an ERROR node, bail out.
        if tree.root_node.type == "ERROR":
            return False

    total_nonblank = sum(1 for line in file_source.splitlines() if line.strip())
    if total_nonblank == 0:
        return False

    data_rows = _extract_data_literal_lines(tree.root_node)
    ratio = len(data_rows) / total_nonblank
    return ratio > threshold
