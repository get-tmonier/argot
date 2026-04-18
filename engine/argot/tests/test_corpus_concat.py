from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.corpus import concat_datasets


def _write_jsonl(path: Path, records: list[dict]) -> None:  # type: ignore[type-arg]
    path.write_text("\n".join(json.dumps(r) for r in records) + "\n")


def test_concat_two_tagged_datasets_returns_counts(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    b = tmp_path / "b.jsonl"
    out = tmp_path / "combined.jsonl"
    _write_jsonl(a, [{"_repo": "alpha", "hunk_tokens": []} for _ in range(3)])
    _write_jsonl(b, [{"_repo": "beta", "hunk_tokens": []} for _ in range(2)])

    counts = concat_datasets([a, b], out)

    assert counts == {"alpha": 3, "beta": 2}
    out_lines = out.read_text().strip().splitlines()
    assert len(out_lines) == 5


def test_concat_rejects_record_missing_repo_tag(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    out = tmp_path / "combined.jsonl"
    _write_jsonl(a, [{"hunk_tokens": []}])  # no _repo tag

    with pytest.raises(ValueError, match="_repo"):
        concat_datasets([a], out)


def test_concat_skips_blank_lines(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    out = tmp_path / "combined.jsonl"
    a.write_text('{"_repo": "x", "hunk_tokens": []}\n\n\n')

    counts = concat_datasets([a], out)

    assert counts == {"x": 1}
