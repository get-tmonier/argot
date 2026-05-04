from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pygit2
import pytest

from argot.extract import main as extract_main


def _make_three_commit_repo(tmp_path: Path) -> tuple[pygit2.Repository, list[str]]:
    """Create a 3-commit repo, each adding a line to main.py. Returns (repo, [sha1, sha2, sha3])."""
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    f = tmp_path / "main.py"
    shas: list[str] = []

    f.write_text("x = 1\n")
    repo.index.add("main.py")
    repo.index.write()
    tree = repo.index.write_tree()
    c1 = repo.create_commit("refs/heads/main", sig, sig, "commit 1", tree, [])
    shas.append(str(c1))

    f.write_text("x = 1\ny = 2\n")
    repo.index.add("main.py")
    repo.index.write()
    tree = repo.index.write_tree()
    c2 = repo.create_commit("refs/heads/main", sig, sig, "commit 2", tree, [c1])
    shas.append(str(c2))

    f.write_text("x = 1\ny = 2\nz = 3\n")
    repo.index.add("main.py")
    repo.index.write()
    tree = repo.index.write_tree()
    c3 = repo.create_commit("refs/heads/main", sig, sig, "commit 3", tree, [c2])
    shas.append(str(c3))

    repo.set_head("refs/heads/main")
    return repo, shas


def test_extract_full_history(tmp_path: Path) -> None:
    """No ref arg: extracts from full history (all 2 non-root commits)."""
    repo_path = tmp_path / "repo"
    repo_path.mkdir()
    _, shas = _make_three_commit_repo(repo_path)

    out_path = tmp_path / "out.jsonl"
    with patch("sys.argv", ["argot-extract", str(repo_path), "--out", str(out_path)]):
        extract_main()  # exits 0 (returns normally) on success

    records = [json.loads(line) for line in out_path.read_text().splitlines()]
    assert len(records) >= 2  # commits 2 and 3 each change main.py
    all_shas = {r["commit_sha"] for r in records}
    assert all_shas.issubset(set(shas))


def test_extract_single_ref_filters_to_that_commit(tmp_path: Path) -> None:
    """Passing a single SHA extracts only records from that commit."""
    repo_path = tmp_path / "repo"
    repo_path.mkdir()
    _, shas = _make_three_commit_repo(repo_path)

    # Use sha2 (the second commit) as the bare ref.
    sha2 = shas[1]
    out_path = tmp_path / "out.jsonl"
    with patch("sys.argv", ["argot-extract", str(repo_path), sha2, "--out", str(out_path)]):
        extract_main()

    records = [json.loads(line) for line in out_path.read_text().splitlines()]
    commit_shas_in_output = {r["commit_sha"] for r in records}
    # Only sha2 should appear (bare ref = just that one commit).
    assert commit_shas_in_output == {sha2}


def test_extract_range_ref(tmp_path: Path) -> None:
    """A..B range extracts only commits in that range."""
    repo_path = tmp_path / "repo"
    repo_path.mkdir()
    _, shas = _make_three_commit_repo(repo_path)

    sha1, sha2, sha3 = shas
    # Range sha1..sha2 should include only sha2.
    out_path = tmp_path / "out.jsonl"
    ref = f"{sha1}..{sha2}"
    with patch("sys.argv", ["argot-extract", str(repo_path), ref, "--out", str(out_path)]):
        extract_main()

    records = [json.loads(line) for line in out_path.read_text().splitlines()]
    commit_shas_in_output = {r["commit_sha"] for r in records}
    assert sha2 in commit_shas_in_output
    assert sha3 not in commit_shas_in_output
    assert sha1 not in commit_shas_in_output


def test_extract_invalid_ref_exits_2(tmp_path: Path) -> None:
    """An unknown ref exits with code 2."""
    repo_path = tmp_path / "repo"
    repo_path.mkdir()
    _make_three_commit_repo(repo_path)

    out_path = tmp_path / "out.jsonl"
    argv = ["argot-extract", str(repo_path), "nonexistent-ref", "--out", str(out_path)]
    with patch("sys.argv", argv), pytest.raises(SystemExit) as exc_info:
        extract_main()
    # Should exit with 2 (error): unknown ref → empty SHA set → sys.exit(2).
    assert exc_info.value.code == 2
