# engine/argot/research/signal/phase14/adapters/test_python_adapter.py
"""Unit tests for PythonAdapter — Fix A (enumerate_sampleable_ranges) and Fix B (header scope)."""

from __future__ import annotations

import textwrap

import pytest

from argot.research.signal.phase14.adapters.python_adapter import PythonAdapter


@pytest.fixture(scope="module")
def adapter() -> PythonAdapter:
    return PythonAdapter()


# ---------------------------------------------------------------------------
# Fix A: enumerate_sampleable_ranges
# ---------------------------------------------------------------------------


def test_enumerate_sampleable_ranges_python_functions_and_classes(adapter: PythonAdapter) -> None:
    src = textwrap.dedent("""\
        def top_func():
            x = 1
            y = 2
            return x + y

        class MyClass:
            def method(self) -> int:
                return 0

        async def async_fn() -> None:
            pass
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    # Expect 3 top-level units: top_func, MyClass, async_fn
    assert len(ranges) == 3
    starts = [r[0] for r in ranges]
    assert all(isinstance(s, int) and s >= 1 for s in starts)
    assert all(e >= s for s, e in ranges)


def test_enumerate_sampleable_ranges_python_only_top_level(adapter: PythonAdapter) -> None:
    """Nested functions are NOT returned — only module-level units."""
    src = textwrap.dedent("""\
        def outer():
            def inner():
                return 1
            return inner()
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1
    assert ranges[0][0] == 1  # outer starts at line 1


def test_enumerate_sampleable_ranges_python_parse_error_returns_empty(
    adapter: PythonAdapter,
) -> None:
    assert adapter.enumerate_sampleable_ranges("def (broken syntax") == []


def test_enumerate_sampleable_ranges_python_1indexed(adapter: PythonAdapter) -> None:
    """Lines are 1-indexed: first function starts at line 1."""
    src = "def f():\n    return 1\n"
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1
    assert ranges[0][0] == 1
    assert ranges[0][1] == 2


# ---------------------------------------------------------------------------
# Fix B: is_auto_generated restricted to header region (HEADER_LINE_LIMIT=20)
# ---------------------------------------------------------------------------


def test_autogen_header_comment_still_flagged(adapter: PythonAdapter) -> None:
    """Marker in line 1 (header) must still flag."""
    src = "# auto-generated — do not edit\n\ndef foo():\n    pass\n"
    assert adapter.is_auto_generated(src) is True


def test_autogen_body_comment_mentioning_generated_not_flagged(adapter: PythonAdapter) -> None:
    """Comment beyond line 20 mentioning 'generated' must NOT flag."""
    # 20 blank lines put the comment at line 21 (1-indexed)
    src = "\n" * 20 + "# this function generates X\n\ndef foo():\n    pass\n"
    assert adapter.is_auto_generated(src) is False


def test_autogen_marker_at_line_20_still_flagged(adapter: PythonAdapter) -> None:
    """Marker exactly at line 20 (1-indexed) must still flag (boundary inclusive)."""
    # 19 blank lines → comment is on line 20 (0-indexed: 19 < 20) → flagged
    src = "\n" * 19 + "# auto-generated — do not edit\n"
    assert adapter.is_auto_generated(src) is True


def test_autogen_marker_at_line_21_not_flagged(adapter: PythonAdapter) -> None:
    """Marker exactly at line 21 (1-indexed) must NOT flag."""
    # 20 blank lines → comment is on line 21 (0-indexed: 20 >= 20) → not flagged
    src = "\n" * 20 + "# auto-generated — do not edit\n"
    assert adapter.is_auto_generated(src) is False


# ---------------------------------------------------------------------------
# Fix C: value-type-aware is_data_dominant
# ---------------------------------------------------------------------------


def test_data_dominant_list_of_strings_python(adapter: PythonAdapter) -> None:
    """Regression: list of string literals must still be flagged as data-dominant."""
    names = [f'"name_{i}"' for i in range(60)]
    src = "CITIES = [\n    " + ",\n    ".join(names) + ",\n]\n"
    assert adapter.is_data_dominant(src) is True


def test_data_dominant_dict_of_methods_python_not_flagged(adapter: PythonAdapter) -> None:
    """Dict whose values are callables must NOT be flagged as data-dominant."""
    src = textwrap.dedent("""\
        import os

        HANDLERS = {
            "get": lambda req: req,
            "post": lambda req: req,
            "put": lambda req: req,
            "delete": lambda req: req,
            "patch": lambda req: req,
            "head": lambda req: req,
            "options": lambda req: req,
        }

        def dispatch(method: str, req: object) -> object:
            return HANDLERS[method](req)
    """)
    assert adapter.is_data_dominant(src) is False
