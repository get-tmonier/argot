from __future__ import annotations

import re
import subprocess
from pathlib import Path

# Default message patterns — generic English words describing code evolution.
# These are NOT framework-specific; they match any refactor-style commit message.
_DEFAULT_PATTERNS: list[str] = [
    r"refactor",
    r"idiomatic",
    r"cleanup",
    r"migrate",
    r"replace",
    r"use .* instead",
    r"rewrite",
]


def _get_commit_list(repo_path: Path, *, limit: int = 10_000) -> list[tuple[str, str]]:
    """Return list of (sha, subject) for up to `limit` commits."""
    result = subprocess.run(
        ["git", "log", "--format=%H %s", "-n", str(limit)],
        cwd=repo_path,
        capture_output=True,
        text=True,
        check=True,
    )
    commits: list[tuple[str, str]] = []
    for line in result.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        sha, _, subject = line.partition(" ")
        commits.append((sha, subject))
    return commits


def _get_diff(repo_path: Path, sha: str) -> str:
    """Return the unified diff for a commit (Python files only, 0 context lines)."""
    result = subprocess.run(
        ["git", "show", "--unified=0", sha, "--", "*.py"],
        cwd=repo_path,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def _parse_hunks(diff_text: str) -> list[tuple[str, str]]:
    """Parse a unified diff and return (before_text, after_text) per hunk."""
    pairs: list[tuple[str, str]] = []
    current_before: list[str] = []
    current_after: list[str] = []
    in_hunk = False

    for line in diff_text.splitlines():
        if line.startswith("@@"):
            # Flush previous hunk
            if in_hunk:
                before = "\n".join(current_before).strip()
                after = "\n".join(current_after).strip()
                if before and after:
                    pairs.append((before, after))
            current_before = []
            current_after = []
            in_hunk = True
        elif in_hunk:
            if line.startswith("-") and not line.startswith("---"):
                current_before.append(line[1:])
            elif line.startswith("+") and not line.startswith("+++"):
                current_after.append(line[1:])
            # Context lines (no prefix) are ignored

    # Flush last hunk
    if in_hunk:
        before = "\n".join(current_before).strip()
        after = "\n".join(current_after).strip()
        if before and after:
            pairs.append((before, after))

    return pairs


def _message_matches(subject: str, patterns: list[str]) -> bool:
    """Return True if `subject` contains any of the regex patterns (case-insensitive)."""
    subject_lower = subject.lower()
    return any(re.search(pat, subject_lower) for pat in patterns)


def _has_high_churn(diff_text: str, *, min_churn_lines: int) -> bool:
    """Return True if any single hunk has >= min_churn_lines removed AND added."""
    current_removed = 0
    current_added = 0
    in_hunk = False

    for line in diff_text.splitlines():
        if line.startswith("@@"):
            # Check previous hunk
            if in_hunk and current_removed >= min_churn_lines and current_added >= min_churn_lines:
                return True
            current_removed = 0
            current_added = 0
            in_hunk = True
        elif in_hunk:
            if line.startswith("-") and not line.startswith("---"):
                current_removed += 1
            elif line.startswith("+") and not line.startswith("+++"):
                current_added += 1

    # Check last hunk
    return bool(
        in_hunk and current_removed >= min_churn_lines and current_added >= min_churn_lines
    )


def _token_count(text: str) -> int:
    """Rough token count: split on whitespace."""
    return len(text.split())


def mine_refactor_pairs(
    repo_path: Path,
    *,
    message_patterns: list[str] | None = None,
    min_churn_lines: int = 3,
    max_file_size_tokens: int = 500,
) -> list[tuple[str, str]]:
    """Mine git history for refactor-style before/after hunk pairs.

    Parameters
    ----------
    repo_path:
        Root of the git repository to mine.
    message_patterns:
        Case-insensitive regex patterns matched against the commit subject line.
        Defaults to a set of generic English refactor-style words.
    min_churn_lines:
        Minimum number of removed *and* added lines in a single hunk to qualify
        via the "high churn ratio" filter (regardless of commit message).
    max_file_size_tokens:
        Maximum whitespace-token count for an individual hunk side to be included.
        Prevents embedding very large hunks.

    Returns
    -------
    list of (before_hunk_text, after_hunk_text) pairs.
    """
    if message_patterns is None:
        message_patterns = _DEFAULT_PATTERNS

    # Verify repo exists and has commits
    try:
        commits = _get_commit_list(repo_path)
    except subprocess.CalledProcessError:
        return []

    if not commits:
        return []

    pairs: list[tuple[str, str]] = []

    for sha, subject in commits:
        try:
            diff_text = _get_diff(repo_path, sha)
        except subprocess.CalledProcessError:
            continue

        if not diff_text.strip():
            continue

        by_message = _message_matches(subject, message_patterns)
        by_churn = _has_high_churn(diff_text, min_churn_lines=min_churn_lines)

        if not (by_message or by_churn):
            continue

        for before, after in _parse_hunks(diff_text):
            # Filter out empty or oversized hunks
            if not before.strip() or not after.strip():
                continue
            if _token_count(before) > max_file_size_tokens:
                continue
            if _token_count(after) > max_file_size_tokens:
                continue
            pairs.append((before, after))

    return pairs
