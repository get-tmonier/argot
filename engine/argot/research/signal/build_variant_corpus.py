from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from argot.research.signal.context_variants import build_context

REPO_MAP: dict[str, str] = {
    "fastapi": "/Users/damienmeur/argot-research/fastapi",
}

_VALID_MODES = ("parent_only", "file_only", "siblings_only", "combined")


def _git_show(clone_path: str, commit_sha: str, file_path: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", clone_path, "show", f"{commit_sha}:{file_path}"],
            capture_output=True,
            timeout=30,
        )
    except subprocess.TimeoutExpired:
        return None
    if result.returncode != 0:
        return None
    try:
        return result.stdout.decode("utf-8")
    except UnicodeDecodeError:
        return None


def _build_variant_corpus(
    corpus_path: Path,
    out_path: Path,
    mode: str,
) -> None:
    records: list[dict[str, Any]] = []
    with corpus_path.open() as fh:
        for line in fh:
            line = line.strip()
            if line:
                records.append(json.loads(line))

    total = len(records)
    resolved = 0
    dropped = 0
    fallback = 0

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w") as out_fh:
        for i, record in enumerate(records, start=1):
            repo = record.get("_repo", "")
            clone_path = REPO_MAP.get(repo)
            if clone_path is None:
                print(f"  WARN: unknown repo {repo!r} — skipping record {i}", file=sys.stderr)
                dropped += 1
                continue

            commit_sha: str = record["commit_sha"]
            file_path: str = record["file_path"]
            source = _git_show(clone_path, commit_sha, file_path)
            if source is None:
                print(
                    f"  WARN: git show failed for {commit_sha}:{file_path} — skipping",
                    file=sys.stderr,
                )
                dropped += 1
                if i % 100 == 0:
                    print(
                        f"[{i}/{total}] resolved={resolved} dropped={dropped}",
                        flush=True,
                    )
                continue

            hunk_start: int = record["hunk_start_line"]
            hunk_end: int = record["hunk_end_line"]
            ctx_result = build_context(source, hunk_start, hunk_end, mode)

            updated = dict(record)
            updated["context_before"] = ctx_result.tokens
            updated["_context_truncated"] = ctx_result.truncated
            updated["_context_fallback"] = ctx_result.variant_fallback

            out_fh.write(json.dumps(updated) + "\n")
            resolved += 1
            if ctx_result.truncated or ctx_result.variant_fallback:
                fallback += 1

            if i % 100 == 0:
                print(f"[{i}/{total}] resolved={resolved} dropped={dropped}", flush=True)

    print(
        f"\nDone. resolved={resolved}, fallback={fallback} (truncated/mode-fallback), "
        f"dropped={dropped}",
        flush=True,
    )

    drop_rate = dropped / total if total > 0 else 0.0
    if drop_rate > 0.05:
        print(
            f"ERROR: drop rate {drop_rate:.1%} exceeds 5% threshold — aborting",
            file=sys.stderr,
        )
        sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build a variant corpus by replacing context_before with a structural variant."
    )
    parser.add_argument(
        "--mode",
        choices=_VALID_MODES,
        required=True,
        help="Context variant mode (baseline is not built here — uses the existing corpus.jsonl)",
    )
    default_corpus = (
        Path(__file__).parent.parent.parent.parent
        / "argot"
        / "acceptance"
        / "catalog"
        / "fastapi"
        / "corpus.jsonl"
    )
    parser.add_argument(
        "--corpus",
        default=str(default_corpus),
        help="Path to source corpus.jsonl (default: acceptance/catalog/fastapi/corpus.jsonl)",
    )
    parser.add_argument(
        "--out",
        default=None,
        help="Output path (default: <corpus_dir>/corpus_<mode>.jsonl)",
    )
    args = parser.parse_args()

    corpus_path = Path(args.corpus)
    if args.out is not None:
        out_path = Path(args.out)
    else:
        out_path = corpus_path.parent / f"corpus_{args.mode}.jsonl"

    print(f"mode={args.mode}", flush=True)
    print(f"corpus={corpus_path}", flush=True)
    print(f"out={out_path}", flush=True)

    _build_variant_corpus(corpus_path, out_path, args.mode)


if __name__ == "__main__":
    main()
