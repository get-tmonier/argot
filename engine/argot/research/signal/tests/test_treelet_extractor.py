from __future__ import annotations

from argot.research.signal.treelet_extractor import extract_treelets

_SAMPLE = """
import os

def greet(name: str) -> str:
    return f"hello {name}"

class Foo:
    x: int = 42
"""


def test_basic_extraction_nonempty() -> None:
    result = extract_treelets(_SAMPLE)
    assert len(result) > 0


def test_no_identifier_strings_in_output() -> None:
    result = extract_treelets(_SAMPLE)
    identifiers = {"greet", "name", "str", "Foo", "x", "int", "os", "hello"}
    for treelet in result:
        for ident in identifiers:
            assert ident not in treelet, f"Identifier {ident!r} leaked into treelet {treelet!r}"


def test_depth2_treelets_present() -> None:
    result = extract_treelets(_SAMPLE)
    d2 = [t for t in result if t.startswith("d2:")]
    assert len(d2) > 0, "Expected depth-2 treelets but found none"


def test_syntax_error_returns_empty() -> None:
    result = extract_treelets("def (broken syntax !!!:")
    assert result == []


def test_depth1_format() -> None:
    result = extract_treelets(_SAMPLE)
    d1 = [t for t in result if t.startswith("d1:")]
    assert len(d1) > 0
    for t in d1:
        parts = t[3:].split(">")
        assert len(parts) == 2, f"Bad d1 format: {t!r}"


def test_depth2_format() -> None:
    result = extract_treelets(_SAMPLE)
    d2 = [t for t in result if t.startswith("d2:")]
    for t in d2:
        parts = t[3:].split(">")
        assert len(parts) == 3, f"Bad d2 format: {t!r}"
