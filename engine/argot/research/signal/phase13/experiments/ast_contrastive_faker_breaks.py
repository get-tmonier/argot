# engine/argot/research/signal/phase13/experiments/ast_contrastive_faker_breaks.py
"""Phase 13 Experiment 13: AST-contrastive break scoring on faker fixtures.

Answers: does ContrastiveAstTreeletScorer carry orthogonal signal on the 5 faker
breaks that defeated BPE-tfidf?  Specifically, does it score mimesis_alt higher
than BPE-tfidf did (4.20)?  If yes, AST carries complementary signal.  If no,
the failure is structural to contrast-of-frequencies regardless of axis.

Steps:
  1. Sanity-check: reproduce Stage 2 FastAPI AUC ~0.9742. Abort if outside [0.90, 1.00].
  2. Fit ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max', depth=3)
     on 722 faker model_a source files.
  3. Score the 5 break fixtures using _fixture_path fallback for hunk slices.
  4. Score 159 calibration hunks from sampled_hunks.jsonl.
  5. Emit ast_contrastive_faker_breaks_scores.json.
  6. Print summary stats.

Usage:
    uv run python \\
        engine/argot/research/signal/phase13/experiments/ast_contrastive_faker_breaks.py
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path
from typing import Any

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

# Script is at engine/argot/research/signal/phase13/experiments/
# 5 parents up = engine/argot/
_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog"
_FASTAPI_DIR = _CATALOG_DIR / "fastapi"
_FAKER_DIR = _CATALOG_DIR / "faker"
_MODEL_A_DIR = _FAKER_DIR / "sources" / "model_a"
_SAMPLED_HUNKS_PATH = _FAKER_DIR / "sampled_hunks.jsonl"
_BREAKS_MANIFEST_PATH = _FAKER_DIR / "breaks_manifest.json"

_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "ast_contrastive_faker_breaks_scores.json"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _percentile(data: list[float], p: float) -> float:
    """Compute percentile p (0-100) using linear interpolation."""
    if not data:
        return 0.0
    sorted_data = sorted(data)
    n = len(sorted_data)
    idx = (p / 100) * (n - 1)
    lo = int(idx)
    hi = lo + 1
    if hi >= n:
        return sorted_data[-1]
    frac = idx - lo
    return sorted_data[lo] + frac * (sorted_data[hi] - sorted_data[lo])


def _percentile_rank(value: float, normal_scores: list[float]) -> float:
    """What % of normal_scores is strictly below value."""
    below = sum(1 for s in normal_scores if s < value)
    return 100.0 * below / len(normal_scores)


def _extract_hunk_lines(source: str, start_line: int, end_line: int) -> str:
    """Extract lines [start_line, end_line] (1-indexed, inclusive)."""
    lines = source.splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


# ---------------------------------------------------------------------------
# Step 1 — FastAPI sanity check
# ---------------------------------------------------------------------------


def _fastapi_sanity_check() -> float:
    """Reproduce Stage 2 FastAPI AUC.  Aborts with SystemExit if AUC outside [0.90, 1.00]."""
    print("Sanity check: reproducing Stage 2 FastAPI AUC ...", flush=True)
    specs = load_manifest(_FASTAPI_DIR)
    records = [fixture_to_record(_FASTAPI_DIR, spec, "file_only") for spec in specs]
    is_break = [spec.is_break for spec in specs]

    control_files = sorted((_FASTAPI_DIR / "fixtures" / "default").glob("control*.py"))
    print(f"  model_A: {len(control_files)} control files", flush=True)

    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    scorer.fit([], model_a_files=control_files)
    scores = scorer.score(records)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    auc = auc_from_scores(break_scores, ctrl_scores)

    print(f"  FastAPI AUC = {auc:.4f}", flush=True)

    if not (0.90 <= auc <= 1.00):
        print(
            f"ERROR: FastAPI AUC {auc:.4f} outside expected [0.90, 1.00]. "
            "Scorer may have drifted — aborting.",
            flush=True,
        )
        raise SystemExit(1)

    print(f"  Sanity check PASSED (AUC={auc:.4f})", flush=True)
    return auc


# ---------------------------------------------------------------------------
# Step 2 — Fit on faker model_a sources
# ---------------------------------------------------------------------------


def _build_faker_scorer() -> ContrastiveAstTreeletScorer:
    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    model_a_files = sorted(_MODEL_A_DIR.glob("*.py"))
    print(f"Fitting on {len(model_a_files)} faker model_a source files ...", flush=True)
    scorer.fit([], model_a_files=model_a_files)
    total_a = scorer._total_a  # noqa: SLF001
    print(f"  model_A total treelets: {total_a}", flush=True)
    return scorer


# ---------------------------------------------------------------------------
# Step 3 — Score break fixtures
# ---------------------------------------------------------------------------


def _load_breaks_manifest() -> list[dict[str, Any]]:
    data: dict[str, Any] = json.loads(_BREAKS_MANIFEST_PATH.read_text(encoding="utf-8"))
    return data["fixtures"]  # type: ignore[no-any-return]


def _score_breaks(
    scorer: ContrastiveAstTreeletScorer,
    manifest: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Score each break fixture using hunk_source → fallback to full file."""
    results: list[dict[str, Any]] = []
    for fixture in manifest:
        fixture_path = _FAKER_DIR / fixture["file"]
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        hunk_source = _extract_hunk_lines(
            source, fixture["hunk_start_line"], fixture["hunk_end_line"]
        )

        # Build record with _fixture_path so the scorer can fall back to full file
        # if the hunk slice is an unparseable mid-block fragment.
        record: dict[str, Any] = {
            "hunk_source": hunk_source,
            "_fixture_path": str(fixture_path.resolve()),
        }

        [score] = scorer.score([record])
        results.append(
            {
                "name": fixture["name"],
                "category": fixture["category"],
                "score": score,
            }
        )
        print(
            f"  {fixture['name']:<35} {fixture['category']:<22} score={score:.4f}",
            flush=True,
        )
    return results


# ---------------------------------------------------------------------------
# Step 4 — Score 159 calibration hunks
# ---------------------------------------------------------------------------


def _load_sampled_hunks() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with _SAMPLED_HUNKS_PATH.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def _score_calibration(
    scorer: ContrastiveAstTreeletScorer,
    records: list[dict[str, Any]],
) -> list[float]:
    """Score calibration hunks.  Each record has hunk_source; no fixture_path needed."""
    scores = scorer.score(records)
    return scores


# ---------------------------------------------------------------------------
# Step 5 — Emit scores JSON
# ---------------------------------------------------------------------------


def _write_scores(
    fastapi_auc: float,
    break_results: list[dict[str, Any]],
    calibration_scores: list[float],
) -> None:
    payload: dict[str, Any] = {
        "experiment": "ast_contrastive_faker_breaks",
        "scorer_params": {
            "epsilon": 1e-7,
            "aggregation": "max",
        },
        "fastapi_sanity_auc": fastapi_auc,
        "n_calibration_hunks": len(calibration_scores),
        "calibration_stats": {
            "min": min(calibration_scores),
            "p50": _percentile(calibration_scores, 50),
            "p90": _percentile(calibration_scores, 90),
            "p99": _percentile(calibration_scores, 99),
            "max": max(calibration_scores),
            "mean": statistics.mean(calibration_scores),
            "stdev": statistics.stdev(calibration_scores),
        },
        "break_scores": break_results,
    }
    _SCORES_OUT.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"\nScores written to {_SCORES_OUT}", flush=True)


# ---------------------------------------------------------------------------
# Step 6 — Print summary
# ---------------------------------------------------------------------------


def _print_summary(
    fastapi_auc: float,
    break_results: list[dict[str, Any]],
    calibration_scores: list[float],
) -> None:
    cal_sorted = sorted(calibration_scores)
    min_cal = min(cal_sorted)
    max_cal = max(cal_sorted)
    p50 = _percentile(cal_sorted, 50)
    p90 = _percentile(cal_sorted, 90)
    p99 = _percentile(cal_sorted, 99)
    mean_cal = statistics.mean(cal_sorted)
    stdev_cal = statistics.stdev(cal_sorted)

    print("\n" + "=" * 70, flush=True)
    print("RESULTS SUMMARY", flush=True)
    print("=" * 70, flush=True)

    print(f"\nFastAPI sanity AUC: {fastapi_auc:.4f}", flush=True)

    print(f"\nCalibration stats (n={len(calibration_scores)}):", flush=True)
    print(
        f"  min={min_cal:.4f}  p50={p50:.4f}  p90={p90:.4f}  p99={p99:.4f}  max={max_cal:.4f}",
        flush=True,
    )
    print(f"  mean={mean_cal:.4f}  stdev={stdev_cal:.4f}", flush=True)

    print("\nBreak scores vs calibration:", flush=True)
    print(
        f"  {'fixture':<35} {'category':<22} {'score':>8} {'vs_p99':>8} {'vs_max':>8}",
        flush=True,
    )
    print("  " + "-" * 83, flush=True)
    for r in break_results:
        vs_p99 = r["score"] - p99
        vs_max = r["score"] - max_cal
        prank = _percentile_rank(r["score"], calibration_scores)
        print(
            f"  {r['name']:<35} {r['category']:<22} {r['score']:>8.4f}"
            f" {vs_p99:>+8.4f} {vs_max:>+8.4f}  (p{prank:.1f} of calibration)",
            flush=True,
        )

    break_scores = [r["score"] for r in break_results]
    min_break = min(break_scores)
    max_break = max(break_scores)
    margin_vs_max = min_break - max_cal
    margin_vs_p99 = min_break - p99

    print("\nSeparation metrics:", flush=True)
    print(f"  max_calibration = {max_cal:.4f}", flush=True)
    print(f"  p99_calibration = {p99:.4f}", flush=True)
    print(f"  min(break_scores) = {min_break:.4f}", flush=True)
    print(f"  max(break_scores) = {max_break:.4f}", flush=True)
    print(f"  margin_vs_max = {margin_vs_max:.4f}", flush=True)
    print(f"  margin_vs_p99 = {margin_vs_p99:.4f}", flush=True)

    if min_break > max_cal:
        verdict = "CLEAN"
    elif min_break > p99:
        verdict = "PARTIAL"
    else:
        verdict = "STRUCTURAL LIMIT"
    print(f"\nVerdict: {verdict}", flush=True)
    print("=" * 70, flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def run() -> dict[str, Any]:
    # Step 1: FastAPI sanity check
    fastapi_auc = _fastapi_sanity_check()

    # Step 2: Fit on faker sources
    print("\nFitting AST-contrastive scorer on faker ...", flush=True)
    scorer = _build_faker_scorer()

    # Step 3: Score break fixtures
    print("\nScoring 5 break fixtures ...", flush=True)
    manifest = _load_breaks_manifest()
    break_results = _score_breaks(scorer, manifest)

    # Step 4: Score 159 calibration hunks
    print("\nScoring calibration hunks ...", flush=True)
    cal_records = _load_sampled_hunks()
    print(f"  Loaded {len(cal_records)} calibration hunks", flush=True)
    calibration_scores = _score_calibration(scorer, cal_records)
    print(f"  Scored {len(calibration_scores)} hunks", flush=True)

    # Step 5: Write scores file
    _write_scores(fastapi_auc, break_results, calibration_scores)

    # Step 6: Print summary
    _print_summary(fastapi_auc, break_results, calibration_scores)

    return {
        "fastapi_auc": fastapi_auc,
        "break_results": break_results,
        "calibration_scores": calibration_scores,
    }


def main() -> None:
    run()


if __name__ == "__main__":
    main()
