"""Smoke tests for the Phase 12 S4 bakeoff CLI — no model loading."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
from typing import Any
from unittest.mock import patch

# ---------------------------------------------------------------------------
# Fake scorer for testing
# ---------------------------------------------------------------------------


class _FakeScorer:
    """Deterministic scorer: score = index of fixture in the list (0-based)."""

    name = "fake_scorer"

    def __init__(self, offset: float = 0.0) -> None:
        self._offset = offset

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        pass

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        return [float(i) + self._offset for i in range(len(fixtures))]


# ---------------------------------------------------------------------------
# Shared fixture data
# ---------------------------------------------------------------------------


def _make_fixture_results(n_break: int = 2, n_ctrl: int = 2) -> list[dict[str, Any]]:
    """Build a minimal list of fixture result dicts as produced by _run_bakeoff."""
    results: list[dict[str, Any]] = []
    for i in range(n_break):
        results.append(
            {
                "name": f"break_{i}",
                "scope": "scope_a",
                "is_break": True,
                "category": "naming",
                "set": "v1",
                "scores": {"fake_scorer": float(10 + i)},
            }
        )
    for i in range(n_ctrl):
        results.append(
            {
                "name": f"ctrl_{i}",
                "scope": "scope_a",
                "is_break": False,
                "category": "naming",
                "set": "v1",
                "scores": {"fake_scorer": float(i)},
            }
        )
    return results


# ---------------------------------------------------------------------------
# Tests for _run_bakeoff
# ---------------------------------------------------------------------------


def test_run_bakeoff_returns_expected_keys() -> None:
    """_run_bakeoff should return a dict with all required keys."""
    from argot.research.signal.base import REGISTRY
    from argot.research.signal.cli.bakeoff import _run_bakeoff

    # Register fake scorer temporarily
    REGISTRY["_test_fake"] = lambda: _FakeScorer()

    try:
        with tempfile.TemporaryDirectory() as tmpdir:
            entry_dir = Path(tmpdir)

            # Write minimal corpus
            corpus_path = entry_dir / "corpus_baseline.jsonl"
            corpus_path.write_text(
                json.dumps({"hunk_tokens": [{"text": "def"}, {"text": "foo"}]}) + "\n"
            )

            # Write manifest
            manifest = {
                "fixtures": [
                    {
                        "name": "fix_break",
                        "scope": "scope_a",
                        "file": "fix.py",
                        "hunk_start_line": 1,
                        "hunk_end_line": 2,
                        "is_break": True,
                        "rationale": "test",
                        "category": "naming",
                        "set": "v1",
                    },
                    {
                        "name": "fix_ctrl",
                        "scope": "scope_a",
                        "file": "fix.py",
                        "hunk_start_line": 1,
                        "hunk_end_line": 2,
                        "is_break": False,
                        "rationale": "test",
                        "category": "naming",
                        "set": "v1",
                    },
                ]
            }
            (entry_dir / "manifest.json").write_text(json.dumps(manifest))

            # Write scopes
            scopes = {"scopes": [{"name": "scope_a", "path_prefix": "", "paradigm": "oop"}]}
            (entry_dir / "scopes.json").write_text(json.dumps(scopes))

            # Write a minimal fixture Python file
            (entry_dir / "fix.py").write_text("x = 1\ny = 2\n")

            # Patch load_corpus and fixture_to_record to avoid filesystem details
            corpus_record = {"hunk_tokens": [{"text": "def"}, {"text": "foo"}]}
            fixture_record = {
                "hunk_tokens": [{"text": "def"}],
                "ctx_before_tokens": [],
            }

            with (
                patch(
                    "argot.research.signal.cli.bakeoff.load_corpus",
                    return_value=[corpus_record],
                ),
                patch(
                    "argot.research.signal.cli.bakeoff.fixture_to_record",
                    return_value=fixture_record,
                ),
            ):
                result = _run_bakeoff(entry_dir, ["_test_fake"], context_mode="baseline")

        assert "scorer_names" in result
        assert "fixtures" in result
        assert "scorer_aucs" in result
        assert result["scorer_names"] == ["_test_fake"]
        assert len(result["fixtures"]) == 2
        assert "_test_fake" in result["scorer_aucs"]
        assert "overall" in result["scorer_aucs"]["_test_fake"]
        assert "by_category" in result["scorer_aucs"]["_test_fake"]
    finally:
        REGISTRY.pop("_test_fake", None)


def test_run_bakeoff_auc_ordering() -> None:
    """A scorer that ranks breaks above controls should yield AUC > 0.5."""
    from argot.research.signal.base import REGISTRY
    from argot.research.signal.cli.bakeoff import _run_bakeoff

    # Scorer that returns high scores for breaks (is_break=True come first in fixture list)
    class _PerfectScorer:
        name = "_test_perfect"

        def fit(self, corpus: list[dict[str, Any]]) -> None:
            pass

        def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
            # Return 1.0 for breaks, 0.0 for controls based on fixture index
            # (breaks are first in our test manifest)
            return [1.0, 0.0]

    REGISTRY["_test_perfect"] = _PerfectScorer

    try:
        with tempfile.TemporaryDirectory() as tmpdir:
            entry_dir = Path(tmpdir)
            manifest = {
                "fixtures": [
                    {
                        "name": "brk",
                        "scope": "s",
                        "file": "f.py",
                        "hunk_start_line": 1,
                        "hunk_end_line": 1,
                        "is_break": True,
                        "rationale": "r",
                        "category": "x",
                        "set": "v1",
                    },
                    {
                        "name": "ctl",
                        "scope": "s",
                        "file": "f.py",
                        "hunk_start_line": 1,
                        "hunk_end_line": 1,
                        "is_break": False,
                        "rationale": "r",
                        "category": "x",
                        "set": "v1",
                    },
                ]
            }
            (entry_dir / "manifest.json").write_text(json.dumps(manifest))
            (entry_dir / "scopes.json").write_text(
                json.dumps({"scopes": [{"name": "s", "path_prefix": "", "paradigm": "oop"}]})
            )
            (entry_dir / "f.py").write_text("x = 1\n")

            corpus_record = {"hunk_tokens": [{"text": "x"}]}
            fixture_record = {"hunk_tokens": [{"text": "x"}], "ctx_before_tokens": []}

            with (
                patch(
                    "argot.research.signal.cli.bakeoff.load_corpus",
                    return_value=[corpus_record],
                ),
                patch(
                    "argot.research.signal.cli.bakeoff.fixture_to_record",
                    return_value=fixture_record,
                ),
            ):
                result = _run_bakeoff(entry_dir, ["_test_perfect"], context_mode="baseline")

        auc = result["scorer_aucs"]["_test_perfect"]["overall"]
        assert auc > 0.5, f"Expected AUC > 0.5 for perfect scorer, got {auc}"
    finally:
        REGISTRY.pop("_test_perfect", None)


# ---------------------------------------------------------------------------
# Tests for _write_report
# ---------------------------------------------------------------------------


def test_write_report_creates_files() -> None:
    """_write_report should create both the .md and .json files."""
    from argot.research.signal.cli.bakeoff import _write_report

    fixture_results = _make_fixture_results(n_break=2, n_ctrl=2)
    scorer_aucs = {
        "fake_scorer": {
            "overall": 0.75,
            "by_category": {"naming": 0.75},
        }
    }
    result: dict[str, Any] = {
        "scorer_names": ["fake_scorer"],
        "fixtures": fixture_results,
        "scorer_aucs": scorer_aucs,
        "scorer_ci": None,
        "context_mode": "file_only",
        "entry": "fastapi",
    }

    with tempfile.TemporaryDirectory() as tmpdir:
        out_dir = Path(tmpdir)
        _write_report(result, out_dir)

        # Check .md file exists and has content
        md_files = list(out_dir.glob("b_mlm_and_existing_*.md"))
        assert len(md_files) == 1, f"Expected 1 .md file, got {md_files}"
        md_text = md_files[0].read_text()
        assert "fake_scorer" in md_text
        assert "0.7500" in md_text

        # Check .json file exists and is valid JSON
        json_files = list(out_dir.glob("b_scores_*.json"))
        assert len(json_files) == 1, f"Expected 1 .json file, got {json_files}"
        scores_data = json.loads(json_files[0].read_text())
        assert scores_data["entry"] == "fastapi"
        assert "scorers" in scores_data
        assert "fixtures" in scores_data
        assert "scorer_aucs" in scores_data


def test_write_report_json_schema() -> None:
    """The JSON scores file should match the expected schema."""
    from argot.research.signal.cli.bakeoff import _write_report

    fixture_results = _make_fixture_results(n_break=1, n_ctrl=1)
    scorer_aucs = {"fake_scorer": {"overall": 0.5, "by_category": {"naming": 0.5}}}
    result: dict[str, Any] = {
        "scorer_names": ["fake_scorer"],
        "fixtures": fixture_results,
        "scorer_aucs": scorer_aucs,
        "scorer_ci": {"fake_scorer": {"delta": 0.1, "ci_lo": -0.05, "ci_hi": 0.25}},
        "context_mode": "baseline",
        "entry": "test_entry",
    }

    with tempfile.TemporaryDirectory() as tmpdir:
        out_dir = Path(tmpdir)
        _write_report(result, out_dir)

        json_files = list(out_dir.glob("b_scores_*.json"))
        assert json_files, "No JSON file was written"
        data = json.loads(json_files[0].read_text())

        # Top-level keys
        assert "entry" in data
        assert "context_mode" in data
        assert "date" in data
        assert "scorers" in data
        assert "fixtures" in data
        assert "scorer_aucs" in data

        # Fixture-level schema
        for f in data["fixtures"]:
            assert "name" in f
            assert "scope" in f
            assert "is_break" in f
            assert "category" in f
            assert "set" in f
            assert "scores" in f

        # Scorer AUC schema
        for _name, auc_data in data["scorer_aucs"].items():
            assert "overall" in auc_data
            assert "by_category" in auc_data


def test_write_report_markdown_structure() -> None:
    """Markdown report should contain required section headers."""
    from argot.research.signal.cli.bakeoff import _write_report

    fixture_results = _make_fixture_results()
    scorer_aucs = {"fake_scorer": {"overall": 0.8, "by_category": {"naming": 0.8}}}
    result: dict[str, Any] = {
        "scorer_names": ["fake_scorer"],
        "fixtures": fixture_results,
        "scorer_aucs": scorer_aucs,
        "scorer_ci": None,
        "context_mode": "file_only",
        "entry": "fastapi",
    }

    with tempfile.TemporaryDirectory() as tmpdir:
        out_dir = Path(tmpdir)
        _write_report(result, out_dir)

        md_files = list(out_dir.glob("b_mlm_and_existing_*.md"))
        md_text = md_files[0].read_text()

        assert "## Summary: Overall AUC" in md_text
        assert "## Per-Category AUC" in md_text
        assert "## Per-Fixture Scores" in md_text
        assert "Phase 12 S4" in md_text
