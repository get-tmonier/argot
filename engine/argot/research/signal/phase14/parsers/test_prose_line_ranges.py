from __future__ import annotations

import textwrap

from argot.research.signal.phase14.parsers import PythonTreeSitterParser


def _parser() -> PythonTreeSitterParser:
    return PythonTreeSitterParser()


def test_module_docstring() -> None:
    src = textwrap.dedent('''\
        """Module docstring."""
        x = 1
    ''')
    # Line 1: module docstring — in prose
    assert 1 in _parser().prose_line_ranges(src)
    assert 2 not in _parser().prose_line_ranges(src)


def test_class_docstring() -> None:
    src = textwrap.dedent('''\
        class Foo:
            """Class docstring."""
            pass
    ''')
    # Line 2: class docstring — in prose
    result = _parser().prose_line_ranges(src)
    assert 2 in result
    assert 1 not in result
    assert 3 not in result


def test_function_docstring() -> None:
    src = textwrap.dedent('''\
        def bar():
            """Function docstring."""
            return 1
    ''')
    result = _parser().prose_line_ranges(src)
    assert 2 in result
    assert 3 not in result


def test_multiline_string_arg() -> None:
    # Multi-line string as an argument — not a docstring but multi-line → in prose
    src = textwrap.dedent('''\
        x = 1
        Doc("""
        some text
        """)
    ''')
    # Lines 2–4 are the multi-line string
    result = _parser().prose_line_ranges(src)
    assert 2 in result
    assert 3 in result
    assert 4 in result
    assert 1 not in result


def test_comments() -> None:
    src = textwrap.dedent('''\
        # top comment
        x = 1  # inline comment
        y = 2
    ''')
    result = _parser().prose_line_ranges(src)
    assert 1 in result
    assert 2 in result
    assert 3 not in result


def test_fstring_interpolation_excluded() -> None:
    # The interpolation line must NOT appear in prose ranges
    src = textwrap.dedent('''\
        def greet(name: str) -> str:
            """Greet someone.

            Returns a greeting.
            """
            return f"Hello {name}!"
    ''')
    result = _parser().prose_line_ranges(src)
    # Lines 2–5 are the docstring
    assert 2 in result
    assert 3 in result
    assert 4 in result
    assert 5 in result
    # Line 6: f-string with interpolation — single-line non-docstring, not in prose
    assert 6 not in result


def test_fstring_multiline_interpolation_lines_excluded() -> None:
    src = textwrap.dedent('''\
        msg = f"""
        Hello {
            name
        } world
        """
    ''')
    result = _parser().prose_line_ranges(src)
    # Lines 1 and 5 are literal parts of the multi-line f-string
    assert 1 in result
    assert 5 in result
    # Lines 2–4 are the interpolation block — must NOT appear
    assert 2 not in result
    assert 3 not in result
    assert 4 not in result


def test_hash_inside_string_not_a_comment() -> None:
    # A '#' inside a string literal must NOT produce a comment entry
    src = textwrap.dedent('''\
        x = "hello # world"
        y = 1
    ''')
    result = _parser().prose_line_ranges(src)
    # Single-line non-docstring string — NOT in prose
    assert 1 not in result
    assert 2 not in result


def test_raw_string_multiline() -> None:
    src = textwrap.dedent('''\
        pattern = r"""
        foo
        bar
        """
    ''')
    result = _parser().prose_line_ranges(src)
    assert 1 in result
    assert 2 in result
    assert 3 in result
    assert 4 in result


def test_byte_string_single_line_not_in_prose() -> None:
    src = textwrap.dedent('''\
        data = b"hello"
        x = 1
    ''')
    result = _parser().prose_line_ranges(src)
    assert 1 not in result
    assert 2 not in result


def test_single_line_non_docstring_string_excluded() -> None:
    src = textwrap.dedent('''\
        def foo():
            x = "not a docstring"
            return x
    ''')
    result = _parser().prose_line_ranges(src)
    assert 2 not in result


def test_multiline_docstring_all_lines_included() -> None:
    src = textwrap.dedent('''\
        def foo():
            """First line.

            Third line.
            """
            pass
    ''')
    result = _parser().prose_line_ranges(src)
    assert 2 in result
    assert 3 in result
    assert 4 in result
    assert 5 in result
    assert 6 not in result
