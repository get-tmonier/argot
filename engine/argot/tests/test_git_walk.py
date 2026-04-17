from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pygit2
import pytest

from argot.git_walk import walk_repo


def _make_repo_with_commit(tmp_path: Path, filename: str, content: str) -> pygit2.Repository:
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    file_path = tmp_path / filename
    file_path.write_text(content)
    repo.index.add(filename)
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "initial commit", tree, [])

    # Make a second commit so we have a parent
    file_path.write_text(content + "\n# change\n")
    repo.index.add(filename)
    repo.index.write()
    tree2 = repo.index.write_tree()
    repo.create_commit(
        "refs/heads/main", sig, sig, "second commit", tree2, [repo.head.target]
    )
    return repo


def test_walk_repo_yields_ts_file(tmp_path: Path) -> None:
    _make_repo_with_commit(tmp_path, "index.ts", "const x = 1;\n")
    results = list(walk_repo(str(tmp_path)))
    assert len(results) >= 1
    _, file_path, blob, hunks = results[0]
    assert file_path == "index.ts"
    assert len(hunks) >= 1


def test_walk_repo_skips_unsupported_extension(tmp_path: Path) -> None:
    _make_repo_with_commit(tmp_path, "data.json", '{"key": "value"}\n')
    results = list(walk_repo(str(tmp_path)))
    assert len(results) == 0


def test_walk_repo_empty_repo(tmp_path: Path) -> None:
    pygit2.init_repository(str(tmp_path))
    results = list(walk_repo(str(tmp_path)))
    assert len(results) == 0
