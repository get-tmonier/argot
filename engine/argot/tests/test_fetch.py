from __future__ import annotations

import json
from pathlib import Path

from argot.fetch import RepoConfig, load_repos_config, merge_jsonl


def test_load_repos_config(tmp_path: Path) -> None:
    config_text = """
[[repos]]
name = "vite"
url = "https://github.com/vitejs/vite"
limit = 5000
label = "good"

[[repos]]
name = "zellij"
url = "https://github.com/zellij-org/zellij"
limit = 3000
label = "good"
"""
    config_file = tmp_path / "repos.toml"
    config_file.write_text(config_text)
    repos = load_repos_config(config_file)
    assert len(repos) == 2
    assert repos[0] == RepoConfig(
        name="vite", url="https://github.com/vitejs/vite", limit=5000, label="good"
    )
    assert repos[1].name == "zellij"


def test_merge_jsonl_files(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    b = tmp_path / "b.jsonl"
    out = tmp_path / "merged.jsonl"
    a.write_text('{"x": 1}\n{"x": 2}\n')
    b.write_text('{"x": 3}\n')
    merge_jsonl([(a, "repo-a"), (b, "repo-b")], out)
    lines = [json.loads(line) for line in out.read_text().splitlines()]
    assert len(lines) == 3
    assert lines[0]["_repo"] == "repo-a"
    assert lines[2]["_repo"] == "repo-b"
    assert lines[2]["x"] == 3
