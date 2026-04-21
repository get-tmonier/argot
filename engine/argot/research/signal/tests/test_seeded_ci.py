"""Tests for argot.research.signal.cli.seeded_ci."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from argot.research.signal.cli.seeded_ci import _write_report, run_seeded_ci

# ---------------------------------------------------------------------------
# _write_report tests (fast, no I/O besides tmp_path)
# ---------------------------------------------------------------------------


def _make_result(
    *,
    delta: float = 0.05,
    ci_lo: float = 0.01,
    ci_hi: float = 0.09,
) -> dict[str, object]:
    return {
        "seed_aucs": {
            "baseline": [0.70, 0.71, 0.69, 0.72, 0.70],
            "file_only": [0.75, 0.76, 0.74, 0.77, 0.75],
        },
        "seeds": [0, 1, 2, 3, 4],
        "baseline_scores_s0": ([1.0, 0.9], [0.5, 0.6]),
        "file_only_scores_s0": ([1.2, 1.1], [0.5, 0.6]),
        "delta": delta,
        "ci_lo": ci_lo,
        "ci_hi": ci_hi,
    }


def test_write_report_creates_file(tmp_path: Path) -> None:
    result = _make_result()
    _write_report(result, tmp_path)
    files = list(tmp_path.glob("a_seeded_ci_*.md"))
    assert len(files) == 1


def test_write_report_verdict_real(tmp_path: Path) -> None:
    """CI lower bound > 0 → promotion statistically real."""
    result = _make_result(ci_lo=0.01, ci_hi=0.09, delta=0.05)
    _write_report(result, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "Phase 11 promotion statistically real" in content
    assert "WARN" not in content


def test_write_report_verdict_warn(tmp_path: Path) -> None:
    """CI lower bound <= 0 → WARN: CI crosses zero."""
    result = _make_result(ci_lo=-0.02, ci_hi=0.08, delta=0.03)
    _write_report(result, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "WARN: CI crosses zero" in content


def test_write_report_contains_per_seed_table(tmp_path: Path) -> None:
    result = _make_result()
    _write_report(result, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    # Table headers
    assert "AUC(baseline)" in content
    assert "AUC(file_only)" in content
    # All 5 seeds appear
    for seed in range(5):
        assert f"| {seed} |" in content


def test_write_report_contains_mean_std_table(tmp_path: Path) -> None:
    result = _make_result()
    _write_report(result, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "mean_AUC" in content
    assert "std_AUC" in content
    assert "baseline" in content
    assert "file_only" in content


def test_write_report_contains_delta_and_ci(tmp_path: Path) -> None:
    result = _make_result(delta=0.05, ci_lo=0.01, ci_hi=0.09)
    _write_report(result, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "0.0500" in content or "+0.0500" in content
    assert "0.0100" in content or "+0.0100" in content
    assert "0.0900" in content or "+0.0900" in content


# ---------------------------------------------------------------------------
# run_seeded_ci integration smoke (mocked scorer and corpus)
# ---------------------------------------------------------------------------


@pytest.fixture()
def fake_catalog(tmp_path: Path) -> Path:
    """Create a minimal fake catalog for run_seeded_ci."""
    entry_dir = tmp_path / "myentry"
    entry_dir.mkdir()

    # scopes.json
    (entry_dir / "scopes.json").write_text(
        '{"scopes": [{"name": "default", "path_prefix": "src/", "paradigm": "mylib"}]}'
    )

    # manifest.json — one break, one control
    (entry_dir / "manifest.json").write_text(
        """{
  "fixtures": [
    {
      "name": "break_foo",
      "scope": "default",
      "file": "fixtures/default/break_foo.py",
      "hunk_start_line": 1,
      "hunk_end_line": 3,
      "is_break": true,
      "category": "routing",
      "set": "v1",
      "rationale": "test break"
    },
    {
      "name": "ctrl_foo",
      "scope": "default",
      "file": "fixtures/default/ctrl_foo.py",
      "hunk_start_line": 1,
      "hunk_end_line": 3,
      "is_break": false,
      "category": "routing",
      "set": "v1",
      "rationale": "test control"
    }
  ]
}"""
    )

    # Fixture stub files
    fixtures_dir = entry_dir / "fixtures" / "default"
    fixtures_dir.mkdir(parents=True)
    (fixtures_dir / "break_foo.py").write_text("x = 1\ny = 2\nz = 3\n")
    (fixtures_dir / "ctrl_foo.py").write_text("a = 1\nb = 2\nc = 3\n")

    # corpus.jsonl with a couple records scoped to src/
    records = [
        {
            "file_path": "src/main.py",
            "author_date_iso": str(i),
            "hunk_tokens": [{"text": f"tok{i}"}],
            "context_before": [],
            "language": "python",
        }
        for i in range(5)
    ]
    with (entry_dir / "corpus.jsonl").open("w") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")

    # variant corpus for file_only mode
    with (entry_dir / "corpus_file_only.jsonl").open("w") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")

    return tmp_path


def test_run_seeded_ci_smoke(fake_catalog: Path) -> None:
    """
    End-to-end smoke: run_seeded_ci with 2 seeds on the fake catalog.
    All scorer internals are mocked so this completes in milliseconds.
    """
    mock_scorer = MagicMock()
    # score() returns a list of floats: [break_score, ctrl_score]
    mock_scorer.score.side_effect = lambda records: [
        1.0 if i == 0 else 0.5 for i in range(len(records))
    ]

    with patch(
        "argot.research.signal.cli.seeded_ci.EnsembleJepaScorer",
        return_value=mock_scorer,
    ):
        result = run_seeded_ci(
            entry_name="myentry",
            seeds=[0, 1],
            catalog_dir=fake_catalog,
        )

    assert "seed_aucs" in result
    assert "baseline" in result["seed_aucs"]
    assert "file_only" in result["seed_aucs"]
    assert len(result["seed_aucs"]["baseline"]) == 2
    assert len(result["seed_aucs"]["file_only"]) == 2
    # AUC values should be in [0, 1]
    for mode in ("baseline", "file_only"):
        for auc in result["seed_aucs"][mode]:
            assert 0.0 <= auc <= 1.0
    # CI fields present
    assert "delta" in result
    assert "ci_lo" in result
    assert "ci_hi" in result
