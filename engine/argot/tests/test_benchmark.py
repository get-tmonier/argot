from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.benchmark import (
    FixtureSpec,
    band_in_range,
    load_manifest,
    percentile_rank,
    run_benchmark,
    score_to_band,
)

# --- score_to_band ---


def test_score_to_band_ok_at_threshold() -> None:
    assert score_to_band(0.5, threshold=0.5) == "ok"


def test_score_to_band_unusual_just_above() -> None:
    assert score_to_band(0.51, threshold=0.5) == "unusual"


def test_score_to_band_unusual_upper_bound() -> None:
    assert score_to_band(0.8, threshold=0.5) == "unusual"


def test_score_to_band_suspicious() -> None:
    assert score_to_band(0.9, threshold=0.5) == "suspicious"


def test_score_to_band_foreign() -> None:
    assert score_to_band(1.5, threshold=0.5) == "foreign"


def test_score_to_band_respects_custom_threshold() -> None:
    assert score_to_band(0.9, threshold=1.0) == "ok"
    assert score_to_band(1.2, threshold=1.0) == "unusual"
    assert score_to_band(1.5, threshold=1.0) == "suspicious"


# --- band_in_range ---


def test_band_in_range_exact_match() -> None:
    assert band_in_range("ok", "ok", "ok")


def test_band_in_range_within_range() -> None:
    assert band_in_range("suspicious", "unusual", "foreign")


def test_band_in_range_below_min() -> None:
    assert not band_in_range("ok", "unusual", "foreign")


def test_band_in_range_above_max() -> None:
    assert not band_in_range("foreign", "ok", "unusual")


# --- load_manifest ---


def _write_manifest(path: Path, entries: list[dict[str, object]]) -> None:
    path.mkdir(parents=True, exist_ok=True)
    (path / "manifest.json").write_text(json.dumps({"fixtures": entries}))


def test_load_manifest_parses_entries(tmp_path: Path) -> None:
    _write_manifest(
        tmp_path,
        [
            {
                "name": "foo",
                "file": "foo.ts",
                "hunk_start_line": 10,
                "hunk_end_line": 15,
                "min_band": "ok",
                "max_band": "unusual",
                "rationale": "test",
            }
        ],
    )
    specs = load_manifest(tmp_path)
    assert len(specs) == 1
    assert specs[0].name == "foo"
    assert specs[0].hunk_start_line == 10
    assert specs[0].hunk_end_line == 15


def test_load_manifest_rejects_unknown_band(tmp_path: Path) -> None:
    _write_manifest(
        tmp_path,
        [
            {
                "name": "bad",
                "file": "bad.ts",
                "hunk_start_line": 1,
                "hunk_end_line": 2,
                "min_band": "banana",
                "max_band": "unusual",
                "rationale": "invalid",
            }
        ],
    )
    with pytest.raises(ValueError, match="min_band"):
        load_manifest(tmp_path)


# --- percentile_rank ---


def test_percentile_rank_empty_distribution() -> None:
    assert percentile_rank(0.5, []) == 0.0


def test_percentile_rank_middle() -> None:
    assert percentile_rank(0.5, [0.1, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.9]) == pytest.approx(50.0)


def test_percentile_rank_above_all() -> None:
    assert percentile_rank(10.0, [0.1, 0.2]) == 100.0


def test_percentile_rank_below_all() -> None:
    assert percentile_rank(0.0, [0.1, 0.2]) == 0.0


# --- run_benchmark ---


def _spec(name: str, min_band: str, max_band: str) -> FixtureSpec:
    return FixtureSpec(
        name=name,
        file=f"{name}.ts",
        hunk_start_line=1,
        hunk_end_line=2,
        min_band=min_band,
        max_band=max_band,
        rationale="",
    )


def test_run_benchmark_all_pass(tmp_path: Path) -> None:
    specs = [_spec("ctrl", "ok", "ok"), _spec("break", "suspicious", "foreign")]
    scores = {"ctrl": 0.2, "break": 0.95}

    def score_fn(spec: FixtureSpec, _dir: Path) -> float:
        return scores[spec.name]

    results = run_benchmark(specs, tmp_path, score_fn, threshold=0.5, distribution=[])
    assert [r.passed for r in results] == [True, True]
    assert results[0].predicted_band == "ok"
    assert results[1].predicted_band == "suspicious"


def test_run_benchmark_flags_failure(tmp_path: Path) -> None:
    specs = [_spec("break_weak", "suspicious", "foreign")]

    def score_fn(_spec: FixtureSpec, _dir: Path) -> float:
        return 0.6  # unusual, not suspicious

    results = run_benchmark(specs, tmp_path, score_fn, threshold=0.5, distribution=[])
    assert not results[0].passed
    assert results[0].predicted_band == "unusual"


def test_run_benchmark_skips_when_score_is_none(tmp_path: Path) -> None:
    specs = [_spec("skipped", "ok", "ok")]

    def score_fn(_spec: FixtureSpec, _dir: Path) -> None:
        return None

    results = run_benchmark(specs, tmp_path, score_fn, threshold=0.5, distribution=[])
    assert results == []


def test_run_benchmark_attaches_percentile(tmp_path: Path) -> None:
    specs = [_spec("ctrl", "ok", "ok")]

    def score_fn(_spec: FixtureSpec, _dir: Path) -> float:
        return 0.2

    results = run_benchmark(specs, tmp_path, score_fn, threshold=0.5, distribution=[0.1, 0.3, 0.5])
    assert results[0].percentile == pytest.approx(100 / 3)


# --- bundled fixtures sanity ---


def test_bundled_manifest_loads() -> None:
    """The default fixture manifest ships valid and points at real files."""
    from argot.benchmark import DEFAULT_FIXTURES_DIR

    specs = load_manifest(DEFAULT_FIXTURES_DIR)
    assert len(specs) >= 4
    for spec in specs:
        assert (DEFAULT_FIXTURES_DIR / spec.file).exists(), f"missing fixture {spec.file}"
        assert spec.hunk_start_line < spec.hunk_end_line
