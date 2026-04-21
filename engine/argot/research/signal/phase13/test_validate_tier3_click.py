"""Tests for Tier 3 click validation."""
from __future__ import annotations
import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
CONTROLS_DIR = FIXTURE_DIR / "controls"
BREAKS_DIR = FIXTURE_DIR / "breaks"
MANIFEST_PATH = FIXTURE_DIR / "manifest.json"


def test_manifest_lists_all_fixtures() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    control_entries = [f for f in manifest["fixtures"] if not f["is_break"]]
    break_entries = [f for f in manifest["fixtures"] if f["is_break"]]
    assert len(control_entries) == 10, f"expected 10 controls, got {len(control_entries)}"
    assert len(break_entries) == 8, f"expected 8 breaks, got {len(break_entries)}"


def test_all_fixture_files_exist() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        path = FIXTURE_DIR / f["file"]
        assert path.exists(), f"fixture file missing: {path}"


def test_control_files_parse_as_python() -> None:
    import ast
    for path in CONTROLS_DIR.glob("*.py"):
        ast.parse(path.read_text())


def test_break_files_have_click_import() -> None:
    """Breaks must look click-ish at the file level, not just the hunk."""
    for path in BREAKS_DIR.glob("*.py"):
        assert "import click" in path.read_text(), f"{path.name} missing click import"


def test_break_hunk_ranges_are_valid() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text())
    for f in manifest["fixtures"]:
        if not f["is_break"]:
            continue
        path = FIXTURE_DIR / f["file"]
        line_count = len(path.read_text().splitlines())
        assert 1 <= f["hunk_start_line"] <= f["hunk_end_line"] <= line_count, \
            f"invalid hunk range in {f['name']}: {f['hunk_start_line']}-{f['hunk_end_line']} for {line_count}-line file"


def test_break_files_parse_as_python() -> None:
    import ast
    for path in BREAKS_DIR.glob("*.py"):
        ast.parse(path.read_text())
