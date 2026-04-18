from __future__ import annotations

from pathlib import Path
from typing import cast

import pygit2

from argot.check import _FILE_COL_WIDTH, _resolve_shas, _trunc, _workdir_patches


def _make_repo(tmp_path: Path, files: dict[str, str]) -> pygit2.Repository:
    """Create a repo with a single commit containing the given files."""
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    for name, content in files.items():
        (tmp_path / name).write_text(content)
        repo.index.add(name)
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "init", tree, [])
    repo.set_head("refs/heads/main")
    return repo


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


def test_workdir_patches_detects_modification(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").write_text("x = 1\ny = 2\n")

    patches = list(_workdir_patches(str(tmp_path)))
    assert len(patches) == 1
    file_path, content, hunks = patches[0]
    assert file_path == "main.py"
    assert b"y = 2" in content
    assert len(hunks) > 0


def test_workdir_patches_ignores_unsupported_extension(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"config.json": "{}\n"})
    (tmp_path / "config.json").write_text('{"key": "value"}\n')

    patches = list(_workdir_patches(str(tmp_path)))
    assert patches == []


def test_workdir_patches_ignores_deleted_files(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").unlink()

    patches = list(_workdir_patches(str(tmp_path)))
    assert patches == []


def test_workdir_patches_no_changes(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})

    patches = list(_workdir_patches(str(tmp_path)))
    assert patches == []


# --- _trunc ---


def test_trunc_short_path() -> None:
    assert _trunc("src/main.py") == "src/main.py"


def test_trunc_exact_length() -> None:
    path = "a" * _FILE_COL_WIDTH
    assert _trunc(path) == path


def test_trunc_long_path_length() -> None:
    path = "x/" * 40 + "important.py"
    result = _trunc(path)
    assert len(result) == _FILE_COL_WIDTH


def test_trunc_long_path_prefix() -> None:
    path = "x/" * 40 + "important.py"
    assert _trunc(path).startswith("...")


def test_trunc_long_path_preserves_suffix() -> None:
    # The filename at the end must survive truncation
    path = "very/long/nested/path/" * 5 + "important.py"
    assert _trunc(path).endswith("important.py")
