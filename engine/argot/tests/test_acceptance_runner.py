from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.acceptance.runner import (
    EntryResult,
    ScopeResult,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
    run_entry,
)


def _write_corpus(path: Path, n: int = 40) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    base_ts = 1_700_000_000
    with path.open("w") as f:
        for i in range(n):
            f.write(
                json.dumps({
                    "_repo": "ky",
                    "author_date_iso": str(base_ts + i * 3600),
                    "file_path": f"source/file_{i % 3}.ts",
                    "language": "typescript",
                    "context_before": [{"text": f"ctx_{i % 7}"}],
                    "context_after": [],
                    "hunk_tokens": [{"text": f"hunk_{i % 11}"}],
                })
                + "\n"
            )


def _make_entry(tmp_path: Path) -> Path:
    entry = tmp_path / "ky"
    entry.mkdir()

    (entry / "scopes.json").write_text(
        json.dumps({
            "scopes": [
                {"name": "default", "path_prefix": "", "paradigm": "Fetch-based HTTP"}
            ]
        })
    )

    fixture_dir = entry / "fixtures" / "default"
    fixture_dir.mkdir(parents=True)
    (fixture_dir / "break.ts").write_text(
        "// fixture\nconst x = 1;\nconst y = 2;\nconst z = x + y;\n"
    )
    (fixture_dir / "control.ts").write_text(
        "// control\nconst a = 1;\nconst b = 2;\nconst c = a + b;\n"
    )

    (entry / "manifest.json").write_text(
        json.dumps({
            "fixtures": [
                {
                    "name": "break_example",
                    "scope": "default",
                    "file": "fixtures/default/break.ts",
                    "hunk_start_line": 2,
                    "hunk_end_line": 4,
                    "is_break": True,
                    "rationale": "Test break fixture",
                },
                {
                    "name": "control_example",
                    "scope": "default",
                    "file": "fixtures/default/control.ts",
                    "hunk_start_line": 2,
                    "hunk_end_line": 4,
                    "is_break": False,
                    "rationale": "Test control fixture",
                },
            ]
        })
    )

    _write_corpus(entry / "corpus.jsonl", n=40)
    return entry


def test_load_scopes(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    scopes = load_scopes(entry)
    assert len(scopes) == 1
    assert scopes[0].name == "default"
    assert scopes[0].path_prefix == ""
    assert scopes[0].paradigm == "Fetch-based HTTP"


def test_load_manifest(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    specs = load_manifest(entry)
    assert len(specs) == 2
    assert specs[0].name == "break_example"
    assert specs[0].is_break is True
    assert specs[1].is_break is False


def test_load_corpus(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    records = load_corpus(entry)
    assert len(records) == 40
    assert all("hunk_tokens" in r for r in records)


def test_fixture_to_record_returns_tokens(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    specs = load_manifest(entry)
    record = fixture_to_record(entry, specs[0])
    assert record["language"] == "typescript"
    assert len(record["hunk_tokens"]) >= 3
    assert all("text" in t for t in record["hunk_tokens"])


def test_run_entry_returns_entry_result(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    result = run_entry(entry, epochs=5)
    assert isinstance(result, EntryResult)
    assert result.entry == "ky"
    assert len(result.scope_results) == 1
    assert isinstance(result.scope_results[0], ScopeResult)
    assert result.scope_results[0].name == "default"
    assert len(result.fixture_scores) == 2


def test_run_entry_passed_reflects_gate(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    result = run_entry(entry, epochs=5)
    # passed must equal whether all scopes clear the gate
    expected = all(s.passed for s in result.scope_results)
    assert result.passed == expected


def test_run_entry_raises_on_too_few_records(tmp_path: Path) -> None:
    entry = _make_entry(tmp_path)
    _write_corpus(entry / "corpus.jsonl", n=5)
    with pytest.raises(RuntimeError, match="only.*records"):
        run_entry(entry, epochs=5)
