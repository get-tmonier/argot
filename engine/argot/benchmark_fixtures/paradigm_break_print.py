from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def _load_records(dataset_path: Path) -> list[dict[str, Any]]:
    if not dataset_path.exists():
        print(f"error: dataset not found at {dataset_path}", file=sys.stderr)
        sys.exit(2)
    records = [json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()]
    if not records:
        print("error: dataset is empty", file=sys.stderr)
        sys.exit(2)
    return records


def _emit_summary(records: list[dict[str, Any]]) -> None:
    by_language: dict[str, int] = {}
    for r in records:
        lang = r.get("language", "unknown")
        by_language[lang] = by_language.get(lang, 0) + 1
    summary = {"total": len(records), "by_language": by_language}
    sys.stdout.write(json.dumps(summary) + "\n")


def _emit_per_repo(records: list[dict[str, Any]]) -> None:
    by_repo: dict[str, int] = {}
    for r in records:
        repo = r.get("_repo", "unknown")
        by_repo[repo] = by_repo.get(repo, 0) + 1
    print("per-repo hunk counts:", by_repo)


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize an argot dataset")
    parser.add_argument("--dataset", default=".argot/dataset.jsonl")
    parser.add_argument("--per-repo", action="store_true")
    args = parser.parse_args()

    records = _load_records(Path(args.dataset))
    _emit_summary(records)
    if args.per_repo:
        _emit_per_repo(records)


if __name__ == "__main__":
    main()
