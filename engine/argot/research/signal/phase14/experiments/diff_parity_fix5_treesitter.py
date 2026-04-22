"""Diff parity check: fix5 baseline vs fix5-treesitter JSONL.

Compares every record matched by (pr_number, file_path, hunk_index) and
reports any differences in flagged, reason, import_score, bpe_score,
and foreign_modules.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

_SCRIPT_DIR = Path(__file__).parent
_BASELINE = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix5_2026_04_22.jsonl"
_NEW = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix5_treesitter_2026_04_22.jsonl"
_TOLERANCE = 1e-9


def _load(path: Path) -> dict[tuple[int, str, int], dict]:
    records: dict[tuple[int, str, int], dict] = {}
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            key = (rec["pr_number"], rec["file_path"], rec["hunk_index"])
            records[key] = rec
    return records


def _floats_close(a: float | None, b: float | None) -> bool:
    if a is None and b is None:
        return True
    if a is None or b is None:
        return False
    return math.isclose(a, b, abs_tol=_TOLERANCE, rel_tol=_TOLERANCE)


def main() -> None:
    print(f"Loading baseline: {_BASELINE}", flush=True)
    baseline = _load(_BASELINE)
    print(f"Loading new:      {_NEW}", flush=True)
    new = _load(_NEW)

    baseline_keys = set(baseline.keys())
    new_keys = set(new.keys())

    only_baseline = baseline_keys - new_keys
    only_new = new_keys - baseline_keys
    common = baseline_keys & new_keys

    mismatches: list[dict] = []

    for key in sorted(common):
        b = baseline[key]
        n = new[key]

        first_diff: str | None = None

        if b.get("flagged") != n.get("flagged"):
            first_diff = f"flagged: {b.get('flagged')!r} → {n.get('flagged')!r}"
        elif b.get("reason") != n.get("reason"):
            first_diff = f"reason: {b.get('reason')!r} → {n.get('reason')!r}"
        elif not _floats_close(b.get("import_score"), n.get("import_score")):
            first_diff = f"import_score: {b.get('import_score')} → {n.get('import_score')}"
        elif not _floats_close(b.get("bpe_score"), n.get("bpe_score")):
            first_diff = f"bpe_score: {b.get('bpe_score')} → {n.get('bpe_score')}"
        elif sorted(b.get("foreign_modules") or []) != sorted(n.get("foreign_modules") or []):
            first_diff = f"foreign_modules: {b.get('foreign_modules')} → {n.get('foreign_modules')}"

        if first_diff:
            mismatches.append(
                {
                    "pr_number": key[0],
                    "file_path": key[1],
                    "hunk_index": key[2],
                    "first_diff": first_diff,
                }
            )

    n_compared = len(common)
    n_matching = n_compared - len(mismatches)

    print(f"\n=== PARITY REPORT ===")
    print(f"Records in baseline:  {len(baseline_keys)}")
    print(f"Records in new:       {len(new_keys)}")
    print(f"Keys only in baseline: {len(only_baseline)}")
    print(f"Keys only in new:      {len(only_new)}")
    print(f"Records compared:      {n_compared}")
    print(f"Records matching:      {n_matching}")
    print(f"Mismatches:            {len(mismatches)}")

    if only_baseline:
        print("\nKeys only in baseline (first 10):")
        for k in sorted(only_baseline)[:10]:
            print(f"  PR#{k[0]} {k[1]} hunk={k[2]}")

    if only_new:
        print("\nKeys only in new (first 10):")
        for k in sorted(only_new)[:10]:
            print(f"  PR#{k[0]} {k[1]} hunk={k[2]}")

    if mismatches:
        print("\nMismatched records (all):")
        for m in mismatches:
            print(f"  PR#{m['pr_number']} {m['file_path']} hunk={m['hunk_index']}  diff={m['first_diff']}")
        sys.exit(1)
    else:
        print("\nAll compared records match. Parity confirmed.")


if __name__ == "__main__":
    main()
