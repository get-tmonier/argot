#!/usr/bin/env python3
"""Smoke-test the assets published together by the Claude Code plugin."""

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MCP_REGISTRY_DESCRIPTION_LIMIT = 100
SKILLS = (
    "argot-setup",
    "argot-refresh",
    "argot-check",
    "argot-review-pr",
    "argot-setup-ci",
    "argot-write-rule",
    "argot-suggest-rules",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text())


def workspace_version() -> str:
    cargo = (ROOT / "Cargo.toml").read_text()
    match = re.search(r'^version = "([^"]+)"', cargo, re.MULTILINE)
    assert match, "Cargo.toml must declare the workspace version"
    return match.group(1)


def skill_name(path: Path) -> str:
    content = path.read_text()
    parts = content.split("---", 2)
    assert len(parts) == 3 and not parts[0].strip(), f"{path} has no YAML front matter"
    name = re.search(r"^name:\s*(\S+)\s*$", parts[1], re.MULTILINE)
    assert name, f"{path} front matter has no name"
    return name.group(1)


def assert_versions(plugin: dict) -> None:
    expected = workspace_version()
    server = load_json(ROOT / "server.json")
    release = load_json(ROOT / "landing/src/data/release.json")

    assert plugin["version"] == expected
    assert server["version"] == expected
    description = server["description"]
    assert 0 < len(description) <= MCP_REGISTRY_DESCRIPTION_LIMIT, (
        "server.json description must be non-empty and at most "
        f"{MCP_REGISTRY_DESCRIPTION_LIMIT} characters for the MCP registry; "
        f"got {len(description)}"
    )
    assert len(server["packages"]) == 1
    assert server["packages"][0]["version"] == expected
    assert release["version"] == expected


def assert_skills() -> None:
    skill_directories = {path.name for path in (ROOT / "skills").iterdir() if path.is_dir()}
    assert skill_directories == set(SKILLS)

    readme = (ROOT / "skills/README.md").read_text()
    assert "Seven skills" in readme
    for skill in SKILLS:
        assert skill_name(ROOT / "skills" / skill / "SKILL.md") == skill
        assert f"`{skill}`" in readme
        assert f"/argot:{skill}" in readme


def assert_plugin_layout(plugin: dict) -> None:
    marketplace = load_json(ROOT / ".claude-plugin/marketplace.json")
    assert plugin["name"] == "argot"
    assert len(marketplace["plugins"]) == 1
    assert marketplace["plugins"][0]["name"] == plugin["name"]
    assert marketplace["plugins"][0]["source"] == "./"

    assert plugin["mcpServers"] == {
        "argot": {"command": "argot", "args": ["mcp", "--repo", "."]}
    }

    hook_paths = list((ROOT / "hooks").glob("**/hooks.json"))
    assert hook_paths == [ROOT / "hooks/hooks.json"]
    hooks = load_json(hook_paths[0])
    assert set(hooks["hooks"]) == {"PreToolUse"}
    pre_tool_use = hooks["hooks"]["PreToolUse"]
    assert len(pre_tool_use) == 1
    assert pre_tool_use[0]["matcher"] == "Write|Edit|MultiEdit"
    declarations = pre_tool_use[0]["hooks"]
    assert len(declarations) == 1
    assert declarations[0]["type"] == "command"
    declaration = declarations[0]
    assert declaration["timeout"] == 5
    assert "argot hook --repo" in declaration["command"]
    assert "argot check" not in declaration["command"]
    assert_hook_fails_open_silently(declaration["command"])

    # The package ships one pre-write ask, not a second copy or a lifecycle hook.
    assert "hooks" not in plugin
    assert "Stop" not in json.dumps(hooks)


def assert_hook_fails_open_silently(command: str) -> None:
    """The manifest wrapper must not turn a hook failure into coding friction."""
    with tempfile.TemporaryDirectory() as temp_dir:
        temp = Path(temp_dir)
        project = temp / "project"
        (project / ".argot").mkdir(parents=True)
        (project / ".argot/scorer-config.json").write_text("not valid json")

        bin_dir = temp / "bin"
        bin_dir.mkdir()
        argot = bin_dir / "argot"
        argot.write_text("#!/bin/sh\nexit 1\n")
        argot.chmod(0o755)

        result = subprocess.run(
            ["sh", "-c", command],
            capture_output=True,
            text=True,
            check=False,
            env={
                **os.environ,
                "CLAUDE_PROJECT_DIR": str(project),
                "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
            },
        )
        assert result.returncode == 0
        assert result.stdout == ""
        assert result.stderr == ""


def assert_mcp_starts(plugin: dict) -> None:
    binary = os.environ.get("ARGOT_BIN")
    assert binary, "set ARGOT_BIN to the checkout-built argot binary"

    version = subprocess.run(
        [binary, "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    assert version == f"argot {plugin['version']}"

    command = plugin["mcpServers"]["argot"]
    mcp = subprocess.run(
        [binary, *command["args"][:-1], str(ROOT)],
        input='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n',
        capture_output=True,
        text=True,
        timeout=10,
        check=True,
    )
    responses = [json.loads(line) for line in mcp.stdout.splitlines() if line]
    response = next(response for response in responses if response.get("id") == 1)
    assert "result" in response


def main() -> None:
    plugin = load_json(ROOT / ".claude-plugin/plugin.json")
    assert_versions(plugin)
    assert_skills()
    assert_plugin_layout(plugin)
    assert_mcp_starts(plugin)


if __name__ == "__main__":
    main()
