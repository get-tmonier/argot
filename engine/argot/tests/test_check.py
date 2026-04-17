from __future__ import annotations

from pathlib import Path
from typing import cast

import pygit2

from argot.check import _resolve_shas


def _make_two_commit_repo(tmp_path: Path) -> pygit2.Repository:
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    f = tmp_path / "main.py"
    f.write_text("x = 1\n")
    repo.index.add("main.py")
    repo.index.write()
    tree1 = repo.index.write_tree()
    c1 = repo.create_commit("refs/heads/main", sig, sig, "first", tree1, [])
    f.write_text("x = 2\n")
    repo.index.add("main.py")
    repo.index.write()
    tree2 = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "second", tree2, [c1])
    return repo


def test_resolve_shas_range(tmp_path: Path) -> None:
    repo = _make_two_commit_repo(tmp_path)
    head_oid = repo.references["refs/heads/main"].target
    commit = cast(pygit2.Commit, repo.get(head_oid))
    parent_oid = commit.parents[0].id

    shas = _resolve_shas(repo, f"{parent_oid}..refs/heads/main")
    assert str(head_oid) in shas
    assert str(parent_oid) not in shas


def test_resolve_shas_bare_ref(tmp_path: Path) -> None:
    repo = _make_two_commit_repo(tmp_path)
    head_oid = str(repo.references["refs/heads/main"].target)
    shas = _resolve_shas(repo, "refs/heads/main")
    assert head_oid in shas
