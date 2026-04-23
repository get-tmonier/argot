from __future__ import annotations

import subprocess
from pathlib import Path


def ensure_clone(data_dir: Path, corpus: str, url: str) -> Path:
    """Clone the repo under data_dir/<corpus>/.repo, or fetch if it already exists.

    Returns the repo directory.
    """
    repo_dir = data_dir / corpus / ".repo"
    if (repo_dir / ".git").exists():
        subprocess.run(
            ["git", "-C", str(repo_dir), "fetch", "--quiet"],
            check=True,
        )
    else:
        repo_dir.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            ["git", "clone", "--quiet", url, str(repo_dir)],
            check=True,
        )
    return repo_dir


def ensure_sha_checked_out(repo_dir: Path, sha: str) -> None:
    """Detached-checkout the given SHA in repo_dir."""
    subprocess.run(
        ["git", "-C", str(repo_dir), "checkout", "--quiet", "--detach", sha],
        check=True,
    )
