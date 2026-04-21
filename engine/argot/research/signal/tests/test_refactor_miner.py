"""Tests for mine_refactor_pairs — uses real temp git repos for subprocess accuracy."""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from argot.research.signal.scorers.refactor_miner import mine_refactor_pairs

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _init_repo(path: Path) -> None:
    """Initialise an empty git repo with a dummy initial commit."""
    subprocess.run(
        ["git", "init", "--initial-branch=main"], cwd=path, check=True, capture_output=True
    )
    subprocess.run(
        ["git", "config", "user.email", "test@test.com"], cwd=path, check=True, capture_output=True
    )
    subprocess.run(
        ["git", "config", "user.name", "Test"], cwd=path, check=True, capture_output=True
    )


def _commit(path: Path, filename: str, content: str, message: str) -> None:
    """Write a file and commit it."""
    (path / filename).write_text(content)
    subprocess.run(["git", "add", filename], cwd=path, check=True, capture_output=True)
    subprocess.run(["git", "commit", "-m", message], cwd=path, check=True, capture_output=True)


def _make_initial_commit(path: Path) -> None:
    """Create a seed commit so HEAD exists."""
    _commit(path, "seed.py", "# seed\n", "init")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_mine_refactor_pairs_message_match() -> None:
    """A commit with 'refactor' in the message whose diff has before+after lines is mined."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _init_repo(repo)
        _make_initial_commit(repo)

        # Write version 1 of the file
        _commit(
            repo,
            "mod.py",
            "def foo():\n    x = []\n    for i in range(10):\n        x.append(i)\n    return x\n",
            "add initial implementation",
        )

        # Write version 2 — simulates a refactor
        _commit(
            repo,
            "mod.py",
            "def foo():\n    return list(range(10))\n",
            "refactor: use list comprehension instead of loop",
        )

        pairs = mine_refactor_pairs(repo)

    assert len(pairs) >= 1, f"Expected at least 1 pair, got {pairs}"
    before, after = pairs[0]
    assert before.strip()
    assert after.strip()


def test_mine_refactor_pairs_no_match_excluded() -> None:
    """A commit with a non-refactor message and low churn is not included."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _init_repo(repo)
        _make_initial_commit(repo)

        _commit(repo, "utils.py", "x = 1\n", "add utils")
        _commit(repo, "utils.py", "x = 2\n", "bump constant")

        pairs = mine_refactor_pairs(repo)

    # The second commit changes 1 line → 1 line, below min_churn_lines=3 default.
    # No refactor-style message either.
    assert pairs == [], f"Expected no pairs, got {pairs}"


def test_mine_refactor_pairs_churn_filter() -> None:
    """A commit with >= min_churn_lines removed AND added in one hunk is included
    even when its message has no refactor keyword."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _init_repo(repo)
        _make_initial_commit(repo)

        old_lines = "\n".join(f"line_{i} = {i}" for i in range(8))
        _commit(repo, "data.py", old_lines + "\n", "add data")

        new_lines = "\n".join(f"value_{i} = {i * 2}" for i in range(8))
        _commit(repo, "data.py", new_lines + "\n", "update variable names")

        pairs = mine_refactor_pairs(repo, min_churn_lines=3)

    assert len(pairs) >= 1, f"Expected at least 1 pair via churn filter, got {pairs}"


def test_mine_refactor_pairs_empty_repo() -> None:
    """An empty (no commits) git repo returns empty list without crashing."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _init_repo(repo)

        # No commits — git log will succeed but return nothing
        pairs = mine_refactor_pairs(repo)

    assert pairs == []


def test_mine_refactor_pairs_not_a_repo_returns_empty() -> None:
    """A non-git directory returns empty list (subprocess error caught)."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        # Not a git repo — mine_refactor_pairs should handle CalledProcessError
        pairs = mine_refactor_pairs(repo)

    assert pairs == []
