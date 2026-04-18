from __future__ import annotations

from typing import Any

import pytest

from argot.validate import stratify_scores, top_dir

# --- top_dir ---


def test_top_dir_nested_path() -> None:
    assert top_dir("cli/src/modules/foo.ts") == "cli"


def test_top_dir_single_component() -> None:
    assert top_dir("foo.ts") == "foo.ts"


def test_top_dir_empty() -> None:
    assert top_dir("") == ""


# --- stratify_scores ---


def _rec(*, file_path: str, language: str) -> dict[str, Any]:
    return {"file_path": file_path, "language": language}


def test_stratify_none_returns_single_group() -> None:
    records = [_rec(file_path="cli/a.ts", language="typescript")]
    scores = [0.1]
    out = stratify_scores(records, scores, by="none")
    assert out == {"all": [0.1]}


def test_stratify_by_language_groups_correctly() -> None:
    records = [
        _rec(file_path="cli/a.ts", language="typescript"),
        _rec(file_path="engine/b.py", language="python"),
        _rec(file_path="cli/c.tsx", language="typescript"),
    ]
    scores = [0.1, 0.5, 0.3]
    out = stratify_scores(records, scores, by="language")
    assert out == {"typescript": [0.1, 0.3], "python": [0.5]}


def test_stratify_by_top_dir_groups_correctly() -> None:
    records = [
        _rec(file_path="cli/a.ts", language="typescript"),
        _rec(file_path="engine/b.py", language="python"),
        _rec(file_path="cli/sub/c.tsx", language="typescript"),
    ]
    scores = [0.1, 0.5, 0.3]
    out = stratify_scores(records, scores, by="top-dir")
    assert out == {"cli": [0.1, 0.3], "engine": [0.5]}


def test_stratify_by_top_dir_handles_empty_path() -> None:
    records = [_rec(file_path="", language="typescript")]
    scores = [0.4]
    out = stratify_scores(records, scores, by="top-dir")
    assert out == {"unknown": [0.4]}


def test_stratify_mismatched_lengths_raises() -> None:
    records = [_rec(file_path="cli/a.ts", language="typescript")]
    scores = [0.1, 0.2]
    with pytest.raises(ValueError):
        stratify_scores(records, scores, by="language")
