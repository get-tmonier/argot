# engine/argot/research/signal/phase14/experiments/mine_faker_prs_2026_04_22.py
"""Phase 14 Prompt S — Mine merged Faker PRs from the last year.

Fetches merged PRs from joke2k/faker between 2025-04-22 and 2026-04-22,
excluding bots, keeping only PRs that touch at least one Python file under
faker/ (the source tree, not tests/).

Includes mergeCommit.oid so the scoring script can use it directly.

Output: faker_real_pr_base_rate_prs_2026_04_22.jsonl

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/mine_faker_prs_2026_04_22.py
"""

from __future__ import annotations

import json
import subprocess
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

_REPO = "joke2k/faker"
_MIN_DATE = datetime(2025, 4, 22, tzinfo=UTC)
_MAX_DATE = datetime(2026, 4, 22, tzinfo=UTC)
_N_TARGET = 50
_BOT_AUTHORS: frozenset[str] = frozenset(
    {
        "dependabot",
        "dependabot[bot]",
        "renovate",
        "renovate[bot]",
        "pre-commit-ci",
        "github-actions[bot]",
        "apps/dependabot",
    }
)

_SCRIPT_DIR = Path(__file__).parent
_OUT = _SCRIPT_DIR / "faker_real_pr_base_rate_prs_2026_04_22.jsonl"


def _is_source_py(path: str) -> bool:
    return path.startswith("faker/") and path.endswith(".py")


def _is_test_py(path: str) -> bool:
    return (
        path.startswith("tests/") or "test_" in path or path.endswith("_test.py")
    ) and path.endswith(".py")


def main() -> None:
    print(f"Fetching merged PRs from {_REPO}...", flush=True)
    result = subprocess.run(
        [
            "gh",
            "pr",
            "list",
            "--repo",
            _REPO,
            "--state",
            "merged",
            "--json",
            "number,title,author,mergedAt,url,files,mergeCommit",
            "--limit",
            "500",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    prs: list[dict[str, Any]] = json.loads(result.stdout)
    print(f"  Fetched {len(prs)} total merged PRs", flush=True)

    filtered: list[dict[str, Any]] = []
    n_outside_date = 0
    n_bots = 0
    n_no_source_py = 0

    for pr in prs:
        merged_str = pr.get("mergedAt") or ""
        if not merged_str:
            continue
        merged_at = datetime.fromisoformat(merged_str.replace("Z", "+00:00"))
        if not (_MIN_DATE <= merged_at <= _MAX_DATE):
            n_outside_date += 1
            continue

        author_login: str = (pr.get("author") or {}).get("login", "")
        if author_login in _BOT_AUTHORS or "[bot]" in author_login:
            n_bots += 1
            continue

        files: list[dict[str, Any]] = pr.get("files") or []
        source_py = [f["path"] for f in files if _is_source_py(f["path"])]
        test_py = [f["path"] for f in files if _is_test_py(f["path"])]

        if not source_py:
            n_no_source_py += 1
            continue

        mc = pr.get("mergeCommit") or {}
        filtered.append(
            {
                "number": pr["number"],
                "title": pr["title"],
                "author": author_login,
                "mergedAt": pr["mergedAt"],
                "url": pr["url"],
                "source_py_files": source_py,
                "test_py_files": test_py,
                "mergeCommit": {"oid": mc.get("oid", "")},
            }
        )

    print(
        f"  Filtered out: {n_outside_date} outside date range, "
        f"{n_bots} bots, {n_no_source_py} no faker/*.py",
        flush=True,
    )
    print(f"  Qualifying PRs: {len(filtered)}", flush=True)

    # Most recent first, take N_TARGET
    filtered.sort(key=lambda p: p["mergedAt"], reverse=True)
    selected = filtered[:_N_TARGET]

    _OUT.parent.mkdir(parents=True, exist_ok=True)
    with _OUT.open("w", encoding="utf-8") as fh:
        for pr in selected:
            fh.write(json.dumps(pr) + "\n")

    print(f"Selected {len(selected)} PRs → {_OUT}", flush=True)
    for pr in selected:
        print(
            f"  #{pr['number']:5d}  {pr['mergedAt'][:10]}  {pr['title'][:60]}",
            flush=True,
        )


if __name__ == "__main__":
    main()
