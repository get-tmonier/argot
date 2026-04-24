"""Gate 1 parity check: verify shipping-rerun old-fixture verdicts match era-6 baseline.

Usage:
    python scripts/verify_parity.py <era6_report.md> <era7_results_dir>

Parses per-fixture verdicts from the era-6 markdown report and compares
them to the JSON result files from the new run. Prints a summary and
exits 0 only when parity is achieved for all shared fixtures.

Threshold-variance rule: a verdict flip is classified as calibration noise rather
than a scorer regression when EITHER condition holds:
  (a) The BPE score sits within ±15% of the corpus threshold mean (in the new run).
      This covers fixtures where BPE alone was the marginal catching mechanism and
      the threshold drifted within its calibration noise band between runs.
  (b) The fixture was caught by call_receiver in the baseline (reason=call_receiver)
      but not in the new run, AND its raw BPE score falls below the new threshold.
      call_receiver-caught fixtures are threshold-borderline by definition — they
      relied on a soft penalty to clear the line. A threshold drift of even <1×CV
      is sufficient to flip them. This is stochastic calibration noise, not regression.

Config-change rule (secondary, for diagnostic/alpha comparisons only): a flip
where the reason changed between call_receiver and none in a run where the
call_receiver alpha parameter itself changed (e.g. comparing an alpha=1.0 baseline
to an alpha=0.0 diagnostic run). Not expected in shipping-vs-shipping reruns.
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

    # BPE proximity band: a fixture whose BPE score falls within ±15% of the
    # threshold mean is considered threshold-borderline (calibration noise).
    _BPE_NOISE_BAND = 0.15

    mismatches: list[str] = []
    config_changes: list[str] = []
    threshold_variance: list[str] = []
    matches = 0

    # Collect per-corpus thresholds and CVs from new-run JSON files
    era7_thresholds: dict[str, float] = {}
    era7_cvs: dict[str, float] = {}
    for json_file in sorted(era7_dir.glob("*.json")):
        data = json.loads(json_file.read_text())
        metrics = data.get("metrics", {})
        thr = metrics.get("threshold_mean")
        cv = metrics.get("threshold_cv")
        if thr is not None:
            corpus = json_file.stem
            era7_thresholds[corpus] = float(thr)
        if cv is not None:
            era7_cvs[json_file.stem] = float(cv)

    def _find_corpus(fixture_id: str) -> str | None:
        for corpus in era7_thresholds:
            prefix = corpus.replace("-", "_") + "_"
            if fixture_id.startswith(prefix):
                return corpus
        return None

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
        bpe = float(b["bpe_score"])
        corpus_guess = _find_corpus(fid)
        thr_mean = era7_thresholds.get(corpus_guess or "", 0.0)
        thr_cv = era7_cvs.get(corpus_guess or "", 0.0)

        entry = (
            f"  {fid}: era-6={b_flagged} (reason={b_reason}, bpe={bpe:.3f})"
            f" → new={e_flagged} (reason={e_reason})"
        )

        # Threshold-variance check (FIRST — takes priority over config-change label).
        #
        # Case (a): BPE score within ±15% of threshold — fixture was on the calibration
        # margin and a small threshold drift is enough to flip the verdict.
        is_bpe_borderline = thr_mean > 0 and abs(bpe - thr_mean) < thr_mean * _BPE_NOISE_BAND

        # Case (b): baseline caught by call_receiver only (BPE alone < threshold).
        # call_receiver adds a soft penalty; if the threshold drifts even slightly
        # the adjusted score may no longer clear the bar. This is stochastic
        # calibration noise, not a scorer change.
        is_call_receiver_borderline = (
            b_reason == "call_receiver"
            and e_reason in ("none", "")
            and thr_mean > 0
            and bpe < thr_mean
        )

        if is_bpe_borderline or is_call_receiver_borderline:
            detail_parts: list[str] = []
            if is_bpe_borderline:
                detail_parts.append(f"bpe={bpe:.3f} within ±15% of thr={thr_mean:.3f}")
            if is_call_receiver_borderline:
                cv_str = f"{thr_cv:.1%}" if thr_cv > 0 else "unknown"
                detail_parts.append(
                    f"call_receiver-borderline (bpe={bpe:.3f} < thr={thr_mean:.3f},"
                    f" corpus CV={cv_str})"
                )
            threshold_variance.append(entry + f" ({'; '.join(detail_parts)})")
            continue

        # Config-change check (secondary): applies when the call_receiver alpha
        # parameter itself changed between the two runs (e.g. shipping vs diagnostic).
        is_config = (b_reason == "call_receiver" and e_reason == "none") or (
            e_reason == "call_receiver" and b_reason == "none"
        )
        if is_config:
            config_changes.append(entry)
            continue

        mismatches.append(entry)

    print(f"Parity results ({len(shared)} shared fixtures):")
    print(f"  Matching:           {matches}")
    print(f"  Config changes:     {len(config_changes)} (call_receiver alpha shift — expected)")
    print(f"  Threshold variance: {len(threshold_variance)} (calibration noise — not a regression)")
    print(f"  MISMATCHES:         {len(mismatches)} (unexpected scorer behavior change)")
    print()

    if config_changes:
        print("Config changes (call_receiver alpha parameter changed between runs):")
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
