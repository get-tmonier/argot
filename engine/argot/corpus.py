from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def concat_datasets(inputs: list[Path], output: Path) -> dict[str, int]:
    """Concatenate tagged JSONL datasets; return per-repo record counts.

    Every record must carry a `_repo` tag (set by `argot-extract --repo-name`).
    """
    counts: dict[str, int] = {}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w") as out_fh:
        for src in inputs:
            for line in src.read_text().splitlines():
                if not line.strip():
                    continue
                record = json.loads(line)
                if "_repo" not in record:
                    raise ValueError(
                        f"record in {src} missing _repo tag "
                        f"(re-extract with --repo-name)"
                    )
                counts[record["_repo"]] = counts.get(record["_repo"], 0) + 1
                out_fh.write(line + "\n")
    return counts


def _cmd_concat(args: argparse.Namespace) -> int:
    inputs = [Path(p) for p in args.inputs]
    for p in inputs:
        if not p.exists():
            print(f"error: input not found: {p}", file=sys.stderr)
            return 2
    counts = concat_datasets(inputs, Path(args.out))
    total = sum(counts.values())
    print(f"wrote {total} records to {args.out}")
    for repo, n in sorted(counts.items()):
        print(f"  {repo}: {n}")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="argot corpus utilities")
    sub = parser.add_subparsers(dest="cmd", required=True)

    concat_p = sub.add_parser("concat", help="Concatenate tagged JSONL datasets")
    concat_p.add_argument("inputs", nargs="+", help="Input JSONL paths")
    concat_p.add_argument("-o", "--out", required=True, help="Output JSONL path")
    concat_p.set_defaults(func=_cmd_concat)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
