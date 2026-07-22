#!/usr/bin/env python3

import json
import os
import re
import subprocess
from pathlib import Path


root = Path(__file__).resolve().parent.parent
plugin = json.loads((root / ".claude-plugin/plugin.json").read_text())
marketplace = json.loads((root / ".claude-plugin/marketplace.json").read_text())
cargo_version = re.search(r'^version = "([^"]+)"', (root / "Cargo.toml").read_text(), re.M)
skills = ["argot-setup", "argot-check", "argot-review-pr", "argot-setup-ci", "argot-write-rule", "argot-suggest-rules"]

assert cargo_version and plugin["version"] == cargo_version.group(1)
assert len(marketplace["plugins"]) == 1
assert marketplace["plugins"][0]["name"] == plugin["name"] == "argot"
assert marketplace["plugins"][0]["source"] == "./"
assert plugin["mcpServers"]["argot"] == {"command": "argot", "args": ["mcp", "--repo", "."]}
for skill in skills:
    content = (root / "skills" / skill / "SKILL.md").read_text()
    assert f"name: {skill}" in content
readme = (root / "skills/README.md").read_text()
assert "Six skills" in readme
for skill in skills:
    assert f"`{skill}`" in readme and f"/argot:{skill}" in readme

hooks = json.loads((root / "hooks/hooks.json").read_text())
assert len(hooks["hooks"]["PreToolUse"]) == 1
assert hooks["hooks"]["PreToolUse"][0]["matcher"] == "Write|Edit|MultiEdit"
assert "Stop" not in json.dumps(hooks)
pre_commit = (root / ".pre-commit-hooks.yaml").read_text()
assert "- id: argot-check\n" in pre_commit and "- id: argot-check-gate\n" in pre_commit
assert 'entry: "sh -c' in pre_commit and "entry: argot check --staged" in pre_commit

binary = os.environ.get("ARGOT_BIN")
assert binary, "set ARGOT_BIN to the checkout-built argot binary"
version = subprocess.run([binary, "--version"], capture_output=True, text=True, check=True).stdout.strip()
assert version == f"argot {plugin['version']}"
mcp = subprocess.run(
    [binary, "mcp", "--repo", str(root)],
    input='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n',
    capture_output=True, text=True, timeout=10, check=True,
)
assert '"id":1' in mcp.stdout and '"result"' in mcp.stdout
