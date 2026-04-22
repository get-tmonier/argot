"""Structural-refactor null test (Experiment G) — re-aggregation over fix4 jsonl."""

import json
import statistics
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

JSONL = Path(__file__).parent / "real_pr_base_rate_hunks_fix4_2026_04_22.jsonl"
REFACTOR_PRS = {14564, 14575}
REF_DATE = datetime(2026, 4, 22, tzinfo=timezone.utc)

records = [json.loads(l) for l in JSONL.read_text().splitlines() if l.strip()]
source = [r for r in records if not r["is_test"]]
residual = [r for r in source if r["pr_number"] not in REFACTOR_PRS]
flagged = [r for r in residual if r["flagged"]]

total_prs = len({r["pr_number"] for r in residual})
n_prs_flagged = len({r["pr_number"] for r in flagged})
stage1 = [r for r in flagged if r["import_score"] > 0]
stage2 = [r for r in flagged if r["import_score"] == 0]
bpe = sorted(r["bpe_score"] for r in flagged)

print(f"Total PRs: {total_prs}")
print(f"Total source hunks: {len(residual)}")
print(f"Flagged source hunks: {len(flagged)}")
print(f"PRs flagged: {n_prs_flagged} ({n_prs_flagged/total_prs*100:.1f}%)")
print(f"Stage 1 flags: {len(stage1)}, Stage 2 flags: {len(stage2)}")
print(f"Distinct files flagged: {len({r['file_path'] for r in flagged})}")
if bpe:
    q = lambda p: bpe[int(len(bpe) * p)]
    print(f"BPE dist: min={bpe[0]:.2f} p25={q(0.25):.2f} median={statistics.median(bpe):.2f} p75={q(0.75):.2f} max={bpe[-1]:.2f}")

print("\nAge brackets:")
buckets: dict[str, list] = defaultdict(list)
for r in residual:
    age = (REF_DATE - datetime.fromisoformat(r["pr_mergedAt"].replace("Z", "+00:00"))).days
    buckets["≤90d" if age <= 90 else "91-180d"].append(r)
for label, recs in buckets.items():
    fl = [r for r in recs if r["flagged"]]
    prs = {r["pr_number"] for r in recs}
    prs_fl = {r["pr_number"] for r in fl}
    rate = len(fl) / len(recs) * 100 if recs else 0
    print(f"  {label}: {len(prs)} PRs, {len(recs)} hunks, {len(fl)} flagged ({rate:.1f}%), {len(prs_fl)} PRs flagged")
