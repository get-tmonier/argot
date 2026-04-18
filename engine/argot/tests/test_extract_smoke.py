from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent.parent.parent


def test_smoke_extract_argot_repo(tmp_path: Path) -> None:
    """Run extract on the argot repo itself and verify ≥1 records (CI has limited history)."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.extract",
            str(REPO_ROOT),
            "--out",
            str(out),
        ],
        capture_output=True,
        text=True,
    )
    # exit 0 means records found, exit 2 means no history (OK in shallow clone)
    assert result.returncode in (0, 2), f"stderr: {result.stderr}"

    if result.returncode == 0:
        assert out.exists(), "Output file should exist"
        lines = out.read_text().strip().splitlines()
        assert len(lines) >= 1, f"Expected ≥1 records, got {len(lines)}"
        for line in lines[:10]:
            record = json.loads(line)
            assert "commit_sha" in record
            assert "file_path" in record
            assert "language" in record
            assert record["hunk_start_line"] <= record["hunk_end_line"]


def test_smoke_extract_with_repo_name(tmp_path: Path) -> None:
    """--repo-name stamps _repo on every output record."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.extract",
            str(REPO_ROOT),
            "--out",
            str(out),
            "--repo-name",
            "argot",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 2), f"stderr: {result.stderr}"
    if result.returncode == 0:
        lines = out.read_text().strip().splitlines()
        assert len(lines) >= 1
        for line in lines:
            record = json.loads(line)
            assert record.get("_repo") == "argot"


def test_smoke_extract_without_repo_name_omits_tag(tmp_path: Path) -> None:
    """Absence of --repo-name leaves records un-tagged (backwards-compat)."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [sys.executable, "-m", "argot.extract", str(REPO_ROOT), "--out", str(out)],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 2)
    if result.returncode == 0:
        lines = out.read_text().strip().splitlines()
        for line in lines:
            record = json.loads(line)
            assert "_repo" not in record
