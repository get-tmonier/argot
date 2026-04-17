from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

import pygit2

from argot.dataset import HunkRecord
from argot.git_walk import walk_repo
from argot.tokenize import language_for_path

CONTEXT_LINES = 50


def _extract_context(
    source_lines: list[str],
    hunk_start: int,
    hunk_end: int,
) -> tuple[list[str], list[str], list[str]]:
    before_start = max(0, hunk_start - CONTEXT_LINES)
    after_end = min(len(source_lines), hunk_end + CONTEXT_LINES)
    return (
        source_lines[before_start:hunk_start],
        source_lines[hunk_start:hunk_end],
        source_lines[hunk_end:after_end],
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract dataset from git history")
    parser.add_argument("repo_path", help="Path to git repository")
    parser.add_argument("--out", default=".argot/dataset.jsonl", help="Output JSONL path")
    parser.add_argument("--limit", type=int, default=None, help="Max number of records to emit")
    args = parser.parse_args()

    repo_path = args.repo_path
    out_path = Path(args.out)

    try:
        pygit2.Repository(repo_path)
    except pygit2.GitError:
        print(f"error: repository not found at {repo_path!r}", file=sys.stderr)
        sys.exit(2)

    out_path.parent.mkdir(parents=True, exist_ok=True)

    count = 0

    with open(out_path, "w") as fh:
        for commit, file_path, post_blob, hunks in walk_repo(repo_path):
            lang = language_for_path(file_path)
            if lang is None:
                continue

            try:
                source_lines = post_blob.decode("utf-8", errors="replace").splitlines()
            except Exception:
                continue

            parent_sha = str(commit.parents[0].id) if commit.parents else None
            author_date_iso = commit.author.time

            for hunk in hunks:
                hunk_start = hunk.new_start - 1  # convert to 0-indexed
                hunk_end = hunk_start + hunk.new_lines

                if hunk_start < 0 or hunk_end > len(source_lines):
                    continue

                ctx_before_lines, hunk_lines, ctx_after_lines = _extract_context(
                    source_lines, hunk_start, hunk_end
                )

                before_start_abs = max(0, hunk_start - CONTEXT_LINES)
                after_start_abs = hunk_end

                from argot.tokenize import tokenize_lines

                context_before = tokenize_lines(source_lines, lang, before_start_abs, hunk_start)
                hunk_tokens = tokenize_lines(source_lines, lang, hunk_start, hunk_end)
                context_after = tokenize_lines(
                    source_lines,
                    lang,
                    after_start_abs,
                    min(len(source_lines), after_start_abs + CONTEXT_LINES),
                )

                record = HunkRecord(
                    commit_sha=str(commit.id),
                    file_path=file_path,
                    language=lang,
                    hunk_start_line=hunk_start,
                    hunk_end_line=hunk_end,
                    context_before=context_before,
                    hunk_tokens=hunk_tokens,
                    context_after=context_after,
                    parent_sha=parent_sha,
                    author_date_iso=str(author_date_iso),
                )

                fh.write(json.dumps(asdict(record)))
                fh.write("\n")
                count += 1

                if args.limit is not None and count >= args.limit:
                    print(f"Reached limit of {args.limit} records", file=sys.stderr)
                    print(f"Wrote {count} records to {out_path}")
                    return

    if count == 0:
        print("error: no hunks found — repository may have no history", file=sys.stderr)
        sys.exit(2)

    print(f"Wrote {count} records to {out_path}")


if __name__ == "__main__":
    main()
