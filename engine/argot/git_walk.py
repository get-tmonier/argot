from __future__ import annotations

from collections.abc import Iterator
from pathlib import Path

import pygit2

SUPPORTED_EXTENSIONS = frozenset({".ts", ".tsx", ".js", ".jsx", ".py"})


def _extension(path: str) -> str:
    return Path(path).suffix.lower()


def walk_repo(repo_path: str) -> Iterator[tuple[pygit2.Commit, str, bytes, list[pygit2.DiffHunk]]]:
    """Yield (commit, file_path, post_blob_content, hunks) for each changed supported file.

    Skips merge commits. Yields nothing if the repo has no commits.
    """
    repo = pygit2.Repository(repo_path)
    if repo.is_empty:
        return

    try:
        start_oid = repo.head.target
    except pygit2.GitError:
        branch_refs = [r for r in repo.references if r.startswith("refs/heads/")]
        if not branch_refs:
            return
        start_oid = repo.references[branch_refs[0]].target

    for commit in repo.walk(start_oid, pygit2.enums.SortMode.TOPOLOGICAL):
        if len(commit.parents) != 1:
            # Skip merge commits and root commits
            continue

        parent = commit.parents[0]
        diff = parent.tree.diff_to_tree(commit.tree)
        diff.find_similar()

        for patch in diff:
            if patch is None:
                continue
            file_path = patch.delta.new_file.path
            if _extension(file_path) not in SUPPORTED_EXTENSIONS:
                continue

            hunks = list(patch.hunks)
            if not hunks:
                continue

            try:
                obj = commit.tree / file_path
                if not isinstance(obj, pygit2.Blob):
                    continue
                post_blob_content = obj.data
            except KeyError:
                # File deleted in this commit
                continue

            yield commit, file_path, post_blob_content, hunks


def walk_commits(
    repo_path: str, shas: set[str]
) -> Iterator[tuple[pygit2.Commit, str, bytes, list[pygit2.DiffHunk]]]:
    """Yield (commit, file_path, post_blob_content, hunks) for commits in shas."""
    repo = pygit2.Repository(repo_path)
    if repo.is_empty:
        return

    try:
        start_oid = repo.head.target
    except pygit2.GitError:
        branch_refs = [r for r in repo.references if r.startswith("refs/heads/")]
        if not branch_refs:
            return
        start_oid = repo.references[branch_refs[0]].target

    for commit in repo.walk(start_oid, pygit2.enums.SortMode.TOPOLOGICAL):
        if str(commit.id) not in shas:
            continue
        if len(commit.parents) != 1:
            continue

        parent = commit.parents[0]
        diff = parent.tree.diff_to_tree(commit.tree)
        diff.find_similar()

        for patch in diff:
            if patch is None:
                continue
            file_path = patch.delta.new_file.path
            if _extension(file_path) not in SUPPORTED_EXTENSIONS:
                continue

            hunks = list(patch.hunks)
            if not hunks:
                continue

            try:
                obj = commit.tree / file_path
                if not isinstance(obj, pygit2.Blob):
                    continue
                post_blob_content = obj.data
            except KeyError:
                continue

            yield commit, file_path, post_blob_content, hunks
