#!/usr/bin/env python3
"""Fixture tests for scripts/check-release-version.py."""

import json
import re
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
    cargo = (fixture / "Cargo.toml").read_text()
    version = re.search(r'^version = "([^"]+)"', cargo, re.MULTILINE)
    assert version, "fixture Cargo.toml must declare a workspace version"
    tag = f"v{version.group(1)}"

    consistent = run(fixture, "--tag", tag)
    assert consistent.returncode == 0, consistent.stderr

    plugin = fixture / ".claude-plugin/plugin.json"
    document = json.loads(plugin.read_text())
    document["version"] = "0.0.0"
    plugin.write_text(json.dumps(document))
    mismatch = run(fixture, "--tag", tag)
    assert mismatch.returncode == 1, mismatch.stderr
    assert "Claude plugin is 0.0.0" in mismatch.stderr
