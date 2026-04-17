from __future__ import annotations

import pytest

from argot.tokenize import tokenize, language_for_path


def test_tokenize_typescript() -> None:
    source = b"const x: number = 42;\n"
    tokens = tokenize(source, "typescript")
    assert len(tokens) > 0
    texts = [t.text for t in tokens]
    assert "42" in texts


def test_tokenize_python() -> None:
    source = b"def foo(x: int) -> int:\n    return x + 1\n"
    tokens = tokenize(source, "python")
    assert len(tokens) > 0
    texts = [t.text for t in tokens]
    assert "foo" in texts


def test_tokenize_javascript() -> None:
    source = b"const add = (a, b) => a + b;\n"
    tokens = tokenize(source, "javascript")
    assert len(tokens) > 0


def test_language_for_path_ts() -> None:
    assert language_for_path("src/index.ts") == "typescript"


def test_language_for_path_tsx() -> None:
    assert language_for_path("src/App.tsx") == "typescript"


def test_language_for_path_py() -> None:
    assert language_for_path("script.py") == "python"


def test_language_for_path_unsupported() -> None:
    assert language_for_path("README.md") is None
