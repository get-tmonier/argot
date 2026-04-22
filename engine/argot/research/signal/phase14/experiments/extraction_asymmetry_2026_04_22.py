# engine/argot/research/signal/phase14/experiments/extraction_asymmetry_2026_04_22.py
"""Phase 14 Experiment 7 Step 1 — Measure calibration vs inference extraction lengths.

Calibration:   raw AST-extracted function/class bodies (sample_hunks seed=0, n=100)
Inference:     file-start-to-hunk-end reconstructed from exp #6 fix1 JSONL (1373 hunks)
Flagged:       subset where flagged == True (258 hunks)

Outputs a markdown report appended to:
  docs/research/scoring/signal/phase14/experiments/extraction_asymmetry_2026-04-22.md
"""

from __future__ import annotations

import json
from pathlib import Path

from argot.research.signal.phase14.calibration.random_hunk_sampler import sample_hunks

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_FASTAPI_REPO = _REPOS_DIR / "fastapi"

_SCRIPT_DIR = Path(__file__).parent
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix1_2026_04_22.jsonl"

_DOCS_DIR = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
)
_REPORT_PATH = _DOCS_DIR / "extraction_asymmetry_2026-04-22.md"

_N_CAL = 100
_CAL_SEED = 0


def _percentiles(values: list[int]) -> dict[str, float]:
    """Compute distribution stats for a list of integers."""
    if not values:
        return {}
    sorted_v = sorted(values)
    n = len(sorted_v)

    def _p(pct: float) -> float:
        idx = pct / 100.0 * (n - 1)
        lo = int(idx)
        hi = min(lo + 1, n - 1)
        frac = idx - lo
        return sorted_v[lo] * (1 - frac) + sorted_v[hi] * frac

    return {
        "min": float(sorted_v[0]),
        "p25": _p(25),
        "median": _p(50),
        "p75": _p(75),
        "p95": _p(95),
        "max": float(sorted_v[-1]),
        "count": float(n),
    }


def _extract_file_to_hunk_end(file_path: Path, end_line: int) -> str | None:
    if not file_path.exists():
        return None
    lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
    hi = min(len(lines), end_line)
    return "\n".join(lines[:hi])


def main() -> None:
    # ── Calibration extraction lengths ────────────────────────────────────────
    print("Sampling calibration hunks (seed=0, n=100)...", flush=True)
    cal_hunks = sample_hunks(_FASTAPI_REPO, _N_CAL, _CAL_SEED)
    cal_lines = [len(h.splitlines()) for h in cal_hunks]
    cal_chars = [len(h) for h in cal_hunks]
    print(f"  Calibration hunks: {len(cal_hunks)}", flush=True)

    # ── Inference extraction lengths ───────────────────────────────────────────
    print(f"Loading inference hunks from {_HUNKS_JSONL.name}...", flush=True)
    records: list[dict[str, object]] = []
    with _HUNKS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    print(f"  Total records in JSONL: {len(records)}", flush=True)

    inf_lines: list[int] = []
    inf_chars: list[int] = []
    flag_lines: list[int] = []
    flag_chars: list[int] = []
    n_missing = 0

    for rec in records:
        # Only measure source hunks (is_test == False) — matches exp #6 scope
        if rec.get("is_test", False):
            continue
        fp = _FASTAPI_REPO / rec["file_path"]
        end_line = rec["hunk_end_line"]
        content = _extract_file_to_hunk_end(fp, end_line)
        if content is None:
            n_missing += 1
            continue
        ln = len(content.splitlines())
        ch = len(content)
        inf_lines.append(ln)
        inf_chars.append(ch)
        if rec.get("flagged", False):
            flag_lines.append(ln)
            flag_chars.append(ch)

    print(f"  Reconstructed: {len(inf_lines)}, missing: {n_missing}", flush=True)
    print(f"  Flagged subset: {len(flag_lines)}", flush=True)

    # ── Compute stats ──────────────────────────────────────────────────────────
    cal_ln_stats = _percentiles(cal_lines)
    cal_ch_stats = _percentiles(cal_chars)
    inf_ln_stats = _percentiles(inf_lines)
    inf_ch_stats = _percentiles(inf_chars)
    flag_ln_stats = _percentiles(flag_lines)
    flag_ch_stats = _percentiles(flag_chars)

    # ── Pre-registered verdict ─────────────────────────────────────────────────
    med_ratio_lines = inf_ln_stats["median"] / cal_ln_stats["median"]
    p95_ratio_lines = inf_ln_stats["p95"] / cal_ln_stats["p95"]
    confirmed = med_ratio_lines >= 3.0 and p95_ratio_lines >= 5.0
    rejected = med_ratio_lines <= 1.5 and p95_ratio_lines <= 1.5
    if confirmed:
        verdict = "ASYMMETRY CONFIRMED"
    elif rejected:
        verdict = "ASYMMETRY REJECTED"
    else:
        verdict = "ASYMMETRY INCONCLUSIVE"

    print(f"\n  median ratio (inf/cal) lines: {med_ratio_lines:.2f}x", flush=True)
    print(f"  p95 ratio (inf/cal) lines:    {p95_ratio_lines:.2f}x", flush=True)
    print(f"  Verdict: {verdict}", flush=True)

    # ── Print summary table to stdout ──────────────────────────────────────────
    def _fmt(stats: dict[str, float], key: str) -> str:
        return f"{stats[key]:,.0f}"

    print("\n--- Lines Distribution ---")
    print(f"{'Set':<20} {'n':>6} {'min':>6} {'p25':>6} {'med':>6} {'p75':>6} {'p95':>7} {'max':>7}")
    for label, stats in [
        ("calibration", cal_ln_stats),
        ("inference_all", inf_ln_stats),
        ("inference_flagged", flag_ln_stats),
    ]:
        print(
            f"{label:<20} {_fmt(stats,'count'):>6} {_fmt(stats,'min'):>6} "
            f"{_fmt(stats,'p25'):>6} {_fmt(stats,'median'):>6} "
            f"{_fmt(stats,'p75'):>6} {_fmt(stats,'p95'):>7} {_fmt(stats,'max'):>7}"
        )

    print("\n--- Chars Distribution ---")
    print(f"{'Set':<20} {'n':>6} {'min':>6} {'p25':>7} {'med':>7} {'p75':>7} {'p95':>8} {'max':>8}")
    for label, stats in [
        ("calibration", cal_ch_stats),
        ("inference_all", inf_ch_stats),
        ("inference_flagged", flag_ch_stats),
    ]:
        print(
            f"{label:<20} {_fmt(stats,'count'):>6} {_fmt(stats,'min'):>6} "
            f"{_fmt(stats,'p25'):>7} {_fmt(stats,'median'):>7} "
            f"{_fmt(stats,'p75'):>7} {_fmt(stats,'p95'):>8} {_fmt(stats,'max'):>8}"
        )

    # ── Write / append markdown report ────────────────────────────────────────
    _DOCS_DIR.mkdir(parents=True, exist_ok=True)

    if confirmed:
        verdict_detail = (
            "Both conditions for CONFIRMED are satisfied: inference extractions are "
            "substantially longer than calibration hunks. The scorer sees much more "
            "context at inference time than at calibration time, which is a plausible "
            "driver of the false-positive rate."
        )
    elif rejected:
        verdict_detail = (
            "Both conditions for REJECTED are satisfied: the distributions are close, "
            "so extraction length asymmetry is not the issue."
        )
    else:
        verdict_detail = (
            "Neither CONFIRMED nor REJECTED threshold is fully met. "
            "The asymmetry is partial — some conditions hold, others do not."
        )

    result_line = (
        f"**Result:** median ratio = {med_ratio_lines:.2f}x (threshold 3×), "
        f"p95 ratio = {p95_ratio_lines:.2f}x (threshold 5×)"
    )
    confirmed_criterion = (
        "- ASYMMETRY CONFIRMED requires: median(inf_lines) ≥ 3× median(cal_lines)"
        " AND p95(inf_lines) ≥ 5× p95(cal_lines)"
    )

    def _row(
        label: str,
        n: int,
        ln: dict[str, float],
        ch: dict[str, float],
    ) -> str:
        return (
            f"| {label} | {n:,} "
            f"| {ln['min']:.0f} / {ln['p25']:.0f} / {ln['median']:.0f} "
            f"/ {ln['p75']:.0f} / {ln['p95']:.0f} / {ln['max']:.0f} "
            f"| {ch['min']:.0f} / {ch['p25']:.0f} / {ch['median']:.0f} "
            f"/ {ch['p75']:.0f} / {ch['p95']:.0f} / {ch['max']:.0f} |"
        )

    report = f"""# Phase 14 Experiment 7 — Calibration/Inference Extraction Asymmetry (2026-04-22)

## §1. Extraction Length Distributions

Calibration uses raw AST-extracted function/class bodies (`sample_hunks(fastapi, n=100, seed=0)`).
Inference uses file-start-to-hunk-end extraction (`_extract_file_to_hunk_end`), reconstructed
from the 1373 records in `real_pr_base_rate_hunks_fix1_2026_04_22.jsonl`.

| Set | n | Lines (min/p25/med/p75/p95/max) | Chars (min/p25/med/p75/p95/max) |
|-----|---|--------------------------------|--------------------------------|
{_row("calibration", len(cal_lines), cal_ln_stats, cal_ch_stats)}
{_row("inference (all)", len(inf_lines), inf_ln_stats, inf_ch_stats)}
{_row("inference (flagged)", len(flag_lines), flag_ln_stats, flag_ch_stats)}

### Ratios (inference_all / calibration)

| Stat | Lines ratio | Chars ratio |
|------|-------------|-------------|
| median | {med_ratio_lines:.2f}x | {inf_ch_stats['median'] / cal_ch_stats['median']:.2f}x |
| p95    | {p95_ratio_lines:.2f}x | {inf_ch_stats['p95'] / cal_ch_stats['p95']:.2f}x |

### Pre-registered verdict: **{verdict}**

**Criteria:**
{confirmed_criterion}
- ASYMMETRY REJECTED requires: both stats within 50% of each other

{result_line}

{verdict_detail}
"""

    with _REPORT_PATH.open("a", encoding="utf-8") as fh:
        fh.write(report)

    print(f"\nReport appended → {_REPORT_PATH}", flush=True)


if __name__ == "__main__":
    main()
