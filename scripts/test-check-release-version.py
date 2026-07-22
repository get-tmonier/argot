#!/usr/bin/env python3
"""Fixture tests for scripts/check-release-version.py."""

import json
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
CHECK = ROOT / "scripts/check-release-version.py"


def run(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(CHECK), "--root", str(root), *args],
        capture_output=True,
        text=True,
        check=False,
    )


with tempfile.TemporaryDirectory() as directory:
    fixture = Path(directory) / "release-tree"
    shutil.copytree(ROOT, fixture, ignore=shutil.ignore_patterns(".git", "target", "node_modules"))

    consistent = run(fixture, "--tag", "v0.2.97")
    assert consistent.returncode == 0, consistent.stderr

    plugin = fixture / ".claude-plugin/plugin.json"
    document = json.loads(plugin.read_text())
    document["version"] = "0.0.0"
    plugin.write_text(json.dumps(document))
    mismatch = run(fixture, "--tag", "v0.2.97")
    assert mismatch.returncode == 1, mismatch.stderr
    assert "Claude plugin is 0.0.0" in mismatch.stderr
