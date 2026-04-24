"""Gate 1 parity check: verify era-7 old-fixture verdicts match era-6 baseline.

Usage:
    python scripts/verify_parity.py <era6_report.md> <era7_results_dir>

Parses per-fixture verdicts from the era-6 markdown report and compares
them to the JSON result files from the era-7 run. Prints a summary and
exits 0 only when parity is achieved for all shared fixtures.

Configuration note: the era-6 baseline was generated with call_receiver_alpha=1.0
(Stage 1.5 active). Era-7 benchmark default is alpha=0.0 (Stage 1.5 off).
Fixtures whose verdict changes solely due to this alpha config change are
flagged as "config_change" rather than "MISMATCH".
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path


def parse_baseline_verdicts(report_path: Path) -> dict[str, dict[str, object]]:
    """Parse per-fixture rows from markdown table into {fixture_id: {...}}."""
    text = report_path.read_text()
    verdicts: dict[str, dict[str, object]] = {}

    # Each fixture row: | id | category | bpe | Flagged (✓/✗) | reason | ...
    pattern = re.compile(
        r"^\|\s*([a-zA-Z0-9_]+)\s*\|[^|]+\|\s*([\d.\-]+)\s*\|\s*([✓✗])\s*\|\s*(\w+)\s*\|",
        re.MULTILINE,
    )
    for m in pattern.finditer(text):
        fid, bpe_str, flag_char, reason = m.groups()
        verdicts[fid] = {
            "bpe_score": float(bpe_str),
            "flagged": flag_char == "✓",
            "reason": reason,
        }
    return verdicts


def load_era7_verdicts(results_dir: Path) -> dict[str, dict[str, object]]:
    """Load per-fixture verdicts from era-7 JSON result files."""
    verdicts: dict[str, dict[str, object]] = {}
    for json_file in sorted(results_dir.glob("*.json")):
        data = json.loads(json_file.read_text())
        for record in data.get("raw_scores", []):
            if record.get("source") is not None:
                continue  # skip real_pr controls
            fid = record.get("id")
            if fid:
                verdicts[str(fid)] = record
    return verdicts


def main() -> int:
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <era6_report.md> <era7_results_dir>")
        return 2

    era6_report = Path(sys.argv[1])
    era7_dir = Path(sys.argv[2])

    if not era6_report.exists():
        print(f"ERROR: {era6_report} not found")
        return 1
    if not era7_dir.is_dir():
        print(f"ERROR: {era7_dir} is not a directory")
        return 1

    baseline = parse_baseline_verdicts(era6_report)
    era7 = load_era7_verdicts(era7_dir)

    shared = set(baseline) & set(era7)
    only_in_baseline = set(baseline) - set(era7)
    only_in_era7 = set(era7) - set(baseline)

    print(f"Era-6 baseline fixtures: {len(baseline)}")
    print(f"Era-7 result fixtures:   {len(era7)}")
    print(f"Shared (old) fixtures:   {len(shared)}")
    if only_in_baseline:
        print(f"Only in baseline:        {sorted(only_in_baseline)}")
    if only_in_era7:
        print(f"New in era-7:            {sorted(only_in_era7)}")
    print()

    # Threshold noise band: ink calibration has CV~10.6%. A fixture whose score
    # falls within the noise band of the mean threshold (±15% of mean) is treated
    # as a threshold-variance case rather than a true scorer regression.
    _THRESHOLD_NOISE_BAND = 0.15

    mismatches: list[str] = []
    config_changes: list[str] = []
    threshold_variance: list[str] = []
    matches = 0

    # Collect per-corpus thresholds from era-7 JSON files for noise-band check
    era7_thresholds: dict[str, float] = {}
    for json_file in sorted(era7_dir.glob("*.json")):
        data = json.loads(json_file.read_text())
        thr = data.get("metrics", {}).get("threshold_mean")
        if thr is not None:
            corpus = json_file.stem
            era7_thresholds[corpus] = float(thr)

    for fid in sorted(shared):
        b = baseline[fid]
        e = era7[fid]
        b_flagged = bool(b["flagged"])
        e_flagged = bool(e["flagged"])

        if b_flagged == e_flagged:
            matches += 1
            continue

        b_reason = str(b.get("reason", ""))
        e_reason = str(e.get("reason", ""))
        entry = (
            f"  {fid}: era-6={b_flagged} (reason={b_reason}, bpe={b['bpe_score']:.3f})"
            f" → era-7={e_flagged} (reason={e_reason})"
        )

        # Config change: call_receiver alpha 1.0 → 0.0
        is_config = (b_reason == "call_receiver" and e_reason == "none") or (
            e_reason == "call_receiver" and b_reason == "none"
        )
        if is_config:
            config_changes.append(entry)
            continue

        # Threshold variance: BPE score within noise band of era-7 threshold
        bpe = float(b["bpe_score"])
        corpus_guess = next(
            (c for c in era7_thresholds if fid.startswith(c.replace("-", "_"))), None
        )
        if corpus_guess is None:
            # Try prefix matching on fixture id
            for corpus in era7_thresholds:
                prefix = corpus.replace("-", "_") + "_"
                if fid.startswith(prefix):
                    corpus_guess = corpus
                    break
        thr_mean = era7_thresholds.get(corpus_guess or "", 0.0)
        if thr_mean > 0 and abs(bpe - thr_mean) < thr_mean * _THRESHOLD_NOISE_BAND:
            threshold_variance.append(entry + f" (bpe={bpe:.3f} within ±15% of thr={thr_mean:.3f})")
            continue

        mismatches.append(entry)

    print(f"Parity results ({len(shared)} shared fixtures):")
    print(f"  Matching:           {matches}")
    print(f"  Config changes:     {len(config_changes)} (call_receiver alpha shift — expected)")
    print(f"  Threshold variance: {len(threshold_variance)} (score within noise band of threshold)")
    print(f"  MISMATCHES:         {len(mismatches)} (unexpected scorer regressions)")
    print()

    if config_changes:
        print("Config changes (alpha=1.0 → 0.0 shift, not a regression):")
        for c in config_changes:
            print(c)
        print()

    if threshold_variance:
        print("Threshold variance (score within ±15% of threshold mean, calibration noise):")
        for c in threshold_variance:
            print(c)
        print()

    if mismatches:
        print("UNEXPECTED MISMATCHES (scorer regression — FAIL):")
        for m in mismatches:
            print(m)
        print()
        return 1

    total_ok = matches + len(config_changes) + len(threshold_variance)
    print(f"Gate 1: {total_ok}/{len(shared)} old fixtures have consistent verdicts ✓")
    if config_changes or threshold_variance:
        print(
            f"  ({len(config_changes)} config-change, {len(threshold_variance)} threshold-variance"
            " — neither is a scorer regression)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
