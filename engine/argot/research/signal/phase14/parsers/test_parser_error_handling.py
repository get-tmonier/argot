from __future__ import annotations

from argot.research.signal.phase14.parsers import PythonTreeSitterParser


def _parser() -> PythonTreeSitterParser:
    return PythonTreeSitterParser()


def test_extract_imports_syntax_error_returns_empty() -> None:
    src = "def foo(\n    x: int\n"  # unclosed function definition
    assert _parser().extract_imports(src) == frozenset()


def test_prose_line_ranges_syntax_error_returns_empty() -> None:
    src = "def foo(\n    x: int\n"
    assert _parser().prose_line_ranges(src) == frozenset()


def test_extract_imports_empty_string() -> None:
    assert _parser().extract_imports("") == frozenset()


def test_prose_line_ranges_empty_string() -> None:
    assert _parser().prose_line_ranges("") == frozenset()
