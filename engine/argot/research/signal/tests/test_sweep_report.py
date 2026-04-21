"""Tests for sweep._write_report with the v2 schema (AUC + delta_v2, per-category AUC)."""

from __future__ import annotations

import statistics
from pathlib import Path
from typing import Any

from argot.research.signal.sweep import _write_report

_NO_FIXTURE_SCORES: dict[tuple[str, int], list[dict[str, Any]]] = {}


def _make_raw_rows(
    config: str = "cfg_a",
    seeds: list[int] | None = None,
) -> list[tuple[str, int, float, float, dict[str, float], float, dict[str, float]]]:
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
        auc = 0.65 + i * 0.01
        auc_by_cat = {
            "routing": 0.70 + i * 0.01,
            "exception_handling": 0.60 + i * 0.01,
            "validation": 0.45 + i * 0.01,
        }
        rows.append((config, seed, delta_v2, delta_v1, cat_dict, auc, auc_by_cat))
    return rows


def test_write_report_creates_file(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    files = list(tmp_path.glob("sweep_fastapi_stage8_*.md"))
    assert len(files) == 1


def test_write_report_raw_table_has_auc_and_delta(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "auc" in content
    assert "delta_v2" in content


def test_write_report_summary_has_mean_auc(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "mean_auc" in content
    assert "mean_delta_v2" in content


def test_write_report_per_category_section_present(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows()
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "Per-category AUC" in content
    assert "routing" in content
    assert "exception_handling" in content
    assert "validation" in content


def test_write_report_category_mean_correct(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows(seeds=[0, 1, 2])
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    # routing AUC across seeds: 0.70, 0.71, 0.72 → mean = 0.71
    expected_routing_mean = statistics.mean([0.70, 0.71, 0.72])
    assert f"{expected_routing_mean:.4f}" in content


def test_write_report_mean_auc_correct(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows(seeds=[0, 1, 2])
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0, 1, 2], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    # AUC across seeds: 0.65, 0.66, 0.67 → mean = 0.66
    expected_mean_auc = statistics.mean([0.65, 0.66, 0.67])
    assert f"{expected_mean_auc:.4f}" in content


def test_write_report_single_seed_std_zero(tmp_path: Path) -> None:
    raw_rows = _make_raw_rows(seeds=[0])
    configs = [{"name": "cfg_a"}]
    _write_report(8, "fastapi", configs, [0], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "0.0000" in content  # std_auc is 0 for single seed


def test_write_report_multiple_configs(tmp_path: Path) -> None:
    rows_a = _make_raw_rows("cfg_a", [0])
    rows_b = _make_raw_rows("cfg_b", [0])
    raw_rows = rows_a + rows_b
    configs = [{"name": "cfg_a"}, {"name": "cfg_b"}]
    _write_report(8, "fastapi", configs, [0], raw_rows, _NO_FIXTURE_SCORES, tmp_path)
    content = next(tmp_path.glob("*.md")).read_text()
    assert "cfg_a" in content
    assert "cfg_b" in content
    assert "### cfg_a" in content
    assert "### cfg_b" in content
