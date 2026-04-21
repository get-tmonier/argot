"""Tests for Tier 3 methodology-controlled rerun on click."""
from __future__ import annotations

import ast
import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
CONTROLS_DIR = FIXTURE_DIR / "controls"
BREAKS_DIR = FIXTURE_DIR / "breaks"
MANIFEST_PATH = FIXTURE_DIR / "manifest_matched.json"

HELD_OUT = {"decorators.py", "types.py", "core.py"}
CONTROL_TARGET_LINES = 20
CONTROL_TOLERANCE = 8  # hunks must be within 20 ± 8 lines (12–28)


def test_manifest_counts() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    controls = [f for f in manifest["fixtures"] if not f["is_break"]]
    breaks = [f for f in manifest["fixtures"] if f["is_break"]]
    assert len(controls) == 10, f"expected 10 controls, got {len(controls)}"
    assert len(breaks) == 8, f"expected 8 breaks, got {len(breaks)}"


def test_controls_only_reference_held_out_files() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        basename = Path(f["file"]).name
        assert basename in HELD_OUT, (
            f"control {f['name']} references {basename}, not in held-out set {HELD_OUT}"
        )


def test_control_hunks_are_size_matched() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        hunk_size = f["hunk_end_line"] - f["hunk_start_line"] + 1
        low = CONTROL_TARGET_LINES - CONTROL_TOLERANCE
        high = CONTROL_TARGET_LINES + CONTROL_TOLERANCE
        assert low <= hunk_size <= high, (
            f"control {f['name']} hunk size {hunk_size} outside [{low}, {high}]"
        )


def test_control_hunk_ranges_valid() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        line_count = len(path.read_text().splitlines())
        assert 1 <= f["hunk_start_line"] <= f["hunk_end_line"] <= line_count, (
            f"invalid range in {f['name']}: "
            f"{f['hunk_start_line']}-{f['hunk_end_line']} for {line_count}-line file"
        )


def test_break_entries_reuse_existing_fixtures() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if not f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        assert path.exists(), f"break fixture missing: {path}"
        ast.parse(path.read_text())
