"""Build a generic Python token frequency table from the CPython stdlib.

Usage:
    python -m argot.research.signal.cli.build_token_reference \\
        --out engine/argot/research/reference/generic_tokens.json
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from argot.dataset import Language as Lang
from argot.research.signal.cli.build_reference import _stdlib_root, _walk_py_files
from argot.tokenize import tokenize_lines

_TOP_N = 100_000
_MAX_BYTES = 10 * 1024 * 1024


def _build_counts(py_files: list[Path]) -> tuple[Counter[str], int]:
    counts: Counter[str] = Counter()
    total_files = 0
    lang: Lang = "python"
    for path in py_files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        lines = source.splitlines(keepends=True)
        tokens = tokenize_lines(lines, lang, 0, len(lines))
        if tokens:
            counts.update(t.text for t in tokens)
            total_files += 1
    return counts, total_files


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Build generic token reference table")
    parser.add_argument("--out", required=True, help="Output JSON path")
    args = parser.parse_args(argv)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    stdlib_root = _stdlib_root()
    print(f"Scanning {stdlib_root} ...", file=sys.stderr)
    py_files = _walk_py_files(stdlib_root)
    print(f"Found {len(py_files)} .py files", file=sys.stderr)

    counts, total_files = _build_counts(py_files)
    total_tokens = sum(counts.values())
    print(
        f"Extracted {total_tokens:,} tokens from {total_files} files",
        file=sys.stderr,
    )

    token_counts = dict(counts.most_common(_TOP_N))

    payload = {
        "version": 1,
        "token_counts": token_counts,
        "total_files": total_files,
        "total_tokens": total_tokens,
    }

    raw = json.dumps(payload, indent=2)
    if len(raw.encode()) > _MAX_BYTES:
        print(
            f"JSON exceeds 10 MB ({len(raw.encode()):,} bytes) — "
            f"already capped at top {_TOP_N:,}",
            file=sys.stderr,
        )

    out_path.write_text(raw, encoding="utf-8")
    print(f"Wrote {out_path} ({len(raw.encode()):,} bytes)", file=sys.stderr)


if __name__ == "__main__":
    main()
