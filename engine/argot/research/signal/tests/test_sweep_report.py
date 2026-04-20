"""Tests for sweep._write_report with the v2 schema (delta_v1, delta_v2, per-category)."""
from __future__ import annotations

import statistics
from pathlib import Path

from argot.research.signal.sweep import _write_report


def _make_raw_rows(
    config: str = "cfg_a",
    seeds: list[int] | None = None,
) -> list[tuple[str, int, float, float, dict[str, float]]]:
    if seeds is None:
        seeds = [0, 1, 2]
    rows = []
    for i, seed in enumerate(seeds):
        delta_v2 = 0.25 + i * 0.01
        delta_v1 = 0.22 + i * 0.01
        cat_dict = {
            "routing": 0.30 + i * 0.01,
            "exception_handling": 0.20 + i * 0.01,
            "validation": -0.05 + i * 0.01,
        }
        rows.append((config, seed, delta_v2, delta_v1, cat_dict))
    return rows


def test_write_report_creates_file(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, tmp_path)
    files = list(tmp_path.glob("sweep_fastapi_stage8_*.md"))
    assert len(files) == 1


def test_write_report_raw_table_has_delta_v1_and_v2(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "delta_v2" in content
    assert "delta_v1" in content
    # Old single 'delta' column should not appear as a header
    assert "| delta |" not in content


def test_write_report_summary_has_mean_delta_v1_and_v2(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "mean_delta_v2" in content
    assert "mean_delta_v1" in content


def test_write_report_per_category_section_present(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "Per-category deltas" in content
    assert "routing" in content
    assert "exception_handling" in content
    assert "validation" in content


def test_write_report_category_mean_correct(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows(seeds=[0, 1, 2])
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    # routing deltas across seeds: 0.30, 0.31, 0.32 → mean = 0.31
    expected_routing_mean = statistics.mean([0.30, 0.31, 0.32])
    assert f"{expected_routing_mean:.4f}" in content


def test_write_report_gate_uses_delta_v2(tmp_path: Path) -> None:
    # delta_v2 >= 0.20 → gate ✓
    raw_rows = [("cfg_a", 0, 0.25, 0.15, {"routing": 0.25})]
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    # The gate for raw row with delta_v2=0.25 should be ✓
    assert "✓" in content


def test_write_report_gate_fails_when_delta_v2_below_threshold(tmp_path: Path) -> None:
    raw_rows = [("cfg_a", 0, 0.10, 0.22, {"routing": 0.10})]
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "✗" in content


def test_write_report_multiple_configs(tmp_path: Path) -> None:
    rows_a = _make_raw_rows("cfg_a", [0])
    rows_b = _make_raw_rows("cfg_b", [0])
    raw_rows = rows_a + rows_b
    configs = [{"name": "cfg_a"}, {"name": "cfg_b"}]
    _write_report(8, "fastapi", configs, [0], raw_rows, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "cfg_a" in content
    assert "cfg_b" in content
    # Per-category section should have subsections for each config
    assert "### cfg_a" in content
    assert "### cfg_b" in content
