"""Build a generic Python treelet frequency table from the CPython stdlib.

Usage:
    python -m argot.research.signal.cli.build_reference \
        --out engine/argot/research/reference/generic_treelets.json
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

from argot.research.signal.treelet_extractor import extract_treelets

_TOP_N = 100_000
_MAX_BYTES = 10 * 1024 * 1024


def _stdlib_root() -> Path:
    result = subprocess.run(
        [sys.executable, "-c", "import sysconfig; print(sysconfig.get_path('stdlib'))"],
        capture_output=True,
        text=True,
        check=True,
    )
    root = Path(result.stdout.strip())
    if not root.is_dir():
        raise RuntimeError(f"stdlib path not found: {root}")
    return root


def _walk_py_files(root: Path) -> list[Path]:
    return list(root.rglob("*.py"))


def _build_counts(py_files: list[Path]) -> tuple[Counter[str], int]:
    counts: Counter[str] = Counter()
    total_files = 0
    for path in py_files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        treelets = extract_treelets(source)
        if treelets:
            counts.update(treelets)
            total_files += 1
    return counts, total_files


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Build generic treelet reference table")
    parser.add_argument("--out", required=True, help="Output JSON path")
    args = parser.parse_args(argv)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    stdlib_root = _stdlib_root()
    print(f"Scanning {stdlib_root} ...", file=sys.stderr)
    py_files = _walk_py_files(stdlib_root)
    print(f"Found {len(py_files)} .py files", file=sys.stderr)

    counts, total_files = _build_counts(py_files)
    total_treelets = sum(counts.values())
    print(
        f"Extracted {total_treelets:,} treelets from {total_files} files",
        file=sys.stderr,
    )

    treelet_counts = dict(counts.most_common(_TOP_N))

    payload = {
        "version": 1,
        "treelet_counts": treelet_counts,
        "total_files": total_files,
        "total_treelets": total_treelets,
    }

    raw = json.dumps(payload, indent=2)
    if len(raw.encode()) > _MAX_BYTES:
        print(
            f"JSON exceeds 10 MB ({len(raw.encode()):,} bytes) — already capped at top {_TOP_N:,}",
            file=sys.stderr,
        )

    out_path.write_text(raw, encoding="utf-8")
    print(f"Wrote {out_path} ({len(raw.encode()):,} bytes)", file=sys.stderr)


if __name__ == "__main__":
    main()
