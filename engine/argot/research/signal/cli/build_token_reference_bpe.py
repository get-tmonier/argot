"""Build a generic Python BPE-token frequency table from the CPython stdlib.

Mirrors build_token_reference.py but tokenises with UnixCoder BPE instead of
argot's word-level tokenizer.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/cli/build_token_reference_bpe.py \\
        --out engine/argot/research/reference/generic_tokens_bpe.json
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer

from argot.research.signal.cli.build_reference import _stdlib_root, _walk_py_files

_TOP_N = 100_000
_MAX_BYTES = 10 * 1024 * 1024
_MODEL_NAME = "microsoft/unixcoder-base"


def _build_counts(py_files: list[Path], tokenizer: Any) -> tuple[Counter[int], int]:
    counts: Counter[int] = Counter()
    total_files = 0
    for path in py_files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        if ids:
            counts.update(ids)
            total_files += 1
    return counts, total_files


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Build generic BPE token reference table")
    parser.add_argument("--out", required=True, help="Output JSON path")
    args = parser.parse_args(argv)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Loading tokenizer {_MODEL_NAME} ...", file=sys.stderr)
    tokenizer = AutoTokenizer.from_pretrained(_MODEL_NAME)  # type: ignore[no-untyped-call]

    stdlib_root = _stdlib_root()
    print(f"Scanning {stdlib_root} ...", file=sys.stderr)
    py_files = _walk_py_files(stdlib_root)
    print(f"Found {len(py_files)} .py files", file=sys.stderr)

    counts, total_files = _build_counts(py_files, tokenizer)
    total_tokens = sum(counts.values())
    print(
        f"Extracted {total_tokens:,} BPE tokens from {total_files} files",
        file=sys.stderr,
    )

    # Store token IDs as strings so JSON keys are valid
    token_counts = {str(k): v for k, v in counts.most_common(_TOP_N)}

    payload = {
        "version": 1,
        "model": _MODEL_NAME,
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
