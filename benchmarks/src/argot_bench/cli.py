from __future__ import annotations

import sys


def main(argv: list[str] | None = None) -> int:
    """argot-bench entry point. Implemented incrementally."""
    args = sys.argv[1:] if argv is None else argv
    if not args:
        print("usage: argot-bench [--corpus=NAMES] [--quick] [--fresh]")
        return 0
    print(f"received args: {args}")
    return 0
