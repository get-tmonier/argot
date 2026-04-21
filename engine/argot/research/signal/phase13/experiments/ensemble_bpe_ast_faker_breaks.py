# engine/argot/research/signal/phase13/experiments/ensemble_bpe_ast_faker_breaks.py
"""Phase 13 Experiment 14: Ensemble (max of BPE-tfidf and AST-contrastive) on faker breaks.

Composes the two existing scorers without re-implementing either:
  - BPE axis: reuses _score_hunk, model_A/B loading, calibration pipeline from
    bpe_contrastive_tfidf_faker_breaks.py
  - AST axis: reuses scorer fitting and scoring from ast_contrastive_faker_breaks.py

For each hunk (5 break fixtures + 159 calibration hunks):
  1. Compute BPE max_score (existing pipeline)
  2. Compute AST max_score (existing pipeline)
  3. ensemble = max(bpe_score, ast_score)

Verdict logic identical to exp #13: CLEAN if min(break_ensemble) > max(calibration_ensemble).

Usage:
    uv run python \\
        engine/argot/research/signal/phase13/experiments/ensemble_bpe_ast_faker_breaks.py
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path
from typing import Any

# Import AST pipeline from sibling script
from argot.research.signal.phase13.experiments.ast_contrastive_faker_breaks import (
    _build_faker_scorer,
    _fastapi_sanity_check,
    _load_breaks_manifest,
    _score_calibration,
)

# Import BPE pipeline from sibling script
# We import the helpers directly rather than calling run() to avoid the report
# side-effect and to get raw per-hunk scores.
from argot.research.signal.phase13.experiments.bpe_contrastive_tfidf_faker_breaks import (
    _build_model_a_bpe_faker,
    _extract_hunk_lines,
    _get_tokenizer,
    _load_model_b_bpe,
    _load_sampled_hunks,
    _percentile,
    _score_hunk,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog" / "faker"
_FIXTURES_DIR = _CATALOG_DIR / "fixtures"

_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "ensemble_bpe_ast_faker_breaks_scores.json"


# ---------------------------------------------------------------------------
# BPE break scoring (mirrors _score_breaks from bpe script but without report)
# ---------------------------------------------------------------------------


def _score_breaks_bpe(
    manifest: list[dict[str, Any]],
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for fixture in manifest:
        fixture_path = _CATALOG_DIR / fixture["file"]
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        hunk_source = _extract_hunk_lines(
            source, fixture["hunk_start_line"], fixture["hunk_end_line"]
        )
        max_score, mean_score, n_tokens, top_3 = _score_hunk(
            hunk_source, tokenizer, id_to_token, model_a, total_a, model_b, total_b
        )
        results.append(
            {
                "name": fixture["name"],
                "category": fixture["category"],
                "bpe_score": max_score,
                "bpe_mean_score": mean_score,
                "bpe_top_3_tokens": top_3,
            }
        )
    return results


# ---------------------------------------------------------------------------
# AST break scoring (thin wrapper around _score_breaks from ast script)
# ---------------------------------------------------------------------------


def _score_breaks_ast(
    scorer: Any,
    manifest: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Score each break fixture on the AST axis."""
    results: list[dict[str, Any]] = []
    for fixture in manifest:
        fixture_path = _CATALOG_DIR / fixture["file"]
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        hunk_source = _extract_hunk_lines(
            source, fixture["hunk_start_line"], fixture["hunk_end_line"]
        )
        record: dict[str, Any] = {
            "hunk_source": hunk_source,
            "_fixture_path": str(fixture_path.resolve()),
        }
        [score] = scorer.score([record])
        results.append(
            {
                "name": fixture["name"],
                "category": fixture["category"],
                "ast_score": score,
            }
        )
    return results


# ---------------------------------------------------------------------------
# BPE calibration scoring
# ---------------------------------------------------------------------------


def _score_calibration_bpe(
    records: list[dict[str, Any]],
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[float]:
    scores: list[float] = []
    for rec in records:
        max_score, _, _, _ = _score_hunk(
            rec["hunk_source"], tokenizer, id_to_token, model_a, total_a, model_b, total_b
        )
        scores.append(max_score)
    return scores


# ---------------------------------------------------------------------------
# Ensemble combiner
# ---------------------------------------------------------------------------


def _ensemble(bpe: float, ast: float) -> float:
    return max(bpe, ast)


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------


def _write_scores(
    fastapi_auc: float,
    break_rows: list[dict[str, Any]],
    cal_bpe: list[float],
    cal_ast: list[float],
    cal_ensemble: list[float],
) -> None:
    payload: dict[str, Any] = {
        "experiment": "ensemble_bpe_ast_faker_breaks",
        "axes": ["bpe_tfidf", "ast_contrastive"],
        "ensemble_rule": "max(bpe, ast)",
        "fastapi_sanity_auc": fastapi_auc,
        "n_calibration_hunks": len(cal_ensemble),
        "calibration_stats": {
            "bpe": {
                "min": min(cal_bpe),
                "p50": _percentile(cal_bpe, 50),
                "p90": _percentile(cal_bpe, 90),
                "p99": _percentile(cal_bpe, 99),
                "max": max(cal_bpe),
                "mean": statistics.mean(cal_bpe),
                "stdev": statistics.stdev(cal_bpe),
            },
            "ast": {
                "min": min(cal_ast),
                "p50": _percentile(cal_ast, 50),
                "p90": _percentile(cal_ast, 90),
                "p99": _percentile(cal_ast, 99),
                "max": max(cal_ast),
                "mean": statistics.mean(cal_ast),
                "stdev": statistics.stdev(cal_ast),
            },
            "ensemble": {
                "min": min(cal_ensemble),
                "p50": _percentile(cal_ensemble, 50),
                "p90": _percentile(cal_ensemble, 90),
                "p99": _percentile(cal_ensemble, 99),
                "max": max(cal_ensemble),
                "mean": statistics.mean(cal_ensemble),
                "stdev": statistics.stdev(cal_ensemble),
            },
        },
        "break_scores": break_rows,
    }
    _SCORES_OUT.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"\nScores written to {_SCORES_OUT}", flush=True)


def _print_summary(
    fastapi_auc: float,
    break_rows: list[dict[str, Any]],
    cal_bpe: list[float],
    cal_ast: list[float],
    cal_ensemble: list[float],
) -> None:
    print("\n" + "=" * 80, flush=True)
    print("ENSEMBLE RESULTS SUMMARY", flush=True)
    print("=" * 80, flush=True)

    print(f"\nFastAPI sanity AUC: {fastapi_auc:.4f}", flush=True)

    for axis, cal in [("BPE", cal_bpe), ("AST", cal_ast), ("ENSEMBLE", cal_ensemble)]:
        p50 = _percentile(cal, 50)
        p90 = _percentile(cal, 90)
        p99 = _percentile(cal, 99)
        print(
            f"\n{axis} calibration (n={len(cal)}): "
            f"min={min(cal):.4f}  p50={p50:.4f}  p90={p90:.4f}  "
            f"p99={p99:.4f}  max={max(cal):.4f}",
            flush=True,
        )

    print("\nBreak scores (BPE / AST / ENSEMBLE):", flush=True)
    hdr = f"  {'fixture':<35} {'bpe':>8} {'ast':>8} {'ens':>8} {'vs_max':>8}"
    print(hdr, flush=True)
    print("  " + "-" * 69, flush=True)

    max_cal_ens = max(cal_ensemble)
    for row in break_rows:
        vs_max = row["ensemble_score"] - max_cal_ens
        print(
            f"  {row['name']:<35} {row['bpe_score']:>8.4f} {row['ast_score']:>8.4f}"
            f" {row['ensemble_score']:>8.4f} {vs_max:>+8.4f}",
            flush=True,
        )

    ens_break_scores = [r["ensemble_score"] for r in break_rows]
    min_break_ens = min(ens_break_scores)
    margin_vs_max = min_break_ens - max_cal_ens
    p99_cal_ens = _percentile(cal_ensemble, 99)
    margin_vs_p99 = min_break_ens - p99_cal_ens

    print("\nEnsemble separation metrics:", flush=True)
    print(f"  max(calibration_ensemble) = {max_cal_ens:.4f}", flush=True)
    print(f"  p99(calibration_ensemble) = {p99_cal_ens:.4f}", flush=True)
    print(f"  min(break_ensemble)        = {min_break_ens:.4f}", flush=True)
    print(f"  margin_vs_max              = {margin_vs_max:+.4f}", flush=True)
    print(f"  margin_vs_p99              = {margin_vs_p99:+.4f}", flush=True)

    if min_break_ens > max_cal_ens:
        verdict = "CLEAN"
    elif min_break_ens > p99_cal_ens:
        verdict = "PARTIAL"
    else:
        verdict = "REGRESSED"
    print(f"\nVerdict: {verdict}", flush=True)
    print("=" * 80, flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def run() -> dict[str, Any]:
    # Step 1: FastAPI sanity check (reuse from AST script)
    fastapi_auc = _fastapi_sanity_check()

    # Step 2: BPE pipeline setup
    print("\nLoading BPE tokenizer and corpora ...", flush=True)
    tokenizer = _get_tokenizer()
    vocab = tokenizer.get_vocab()
    id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

    print("Building BPE Model A (faker sources) ...", flush=True)
    model_a_bpe, total_a_bpe = _build_model_a_bpe_faker(tokenizer)
    print(f"  Model A: {len(model_a_bpe)} unique tokens, {total_a_bpe} total", flush=True)

    print("Loading BPE Model B (generic reference) ...", flush=True)
    model_b_bpe, total_b_bpe = _load_model_b_bpe()
    print(f"  Model B: {len(model_b_bpe)} unique tokens, {total_b_bpe} total", flush=True)

    # Step 3: AST pipeline setup
    print("\nFitting AST-contrastive scorer on faker ...", flush=True)
    ast_scorer = _build_faker_scorer()

    # Step 4: Load calibration hunks
    print("\nLoading calibration hunks ...", flush=True)
    cal_records = _load_sampled_hunks()
    print(f"  Loaded {len(cal_records)} calibration hunks", flush=True)

    # Step 5: Score calibration hunks on both axes
    print("\nScoring calibration hunks on BPE axis ...", flush=True)
    cal_bpe = _score_calibration_bpe(
        cal_records, tokenizer, id_to_token, model_a_bpe, total_a_bpe, model_b_bpe, total_b_bpe
    )
    print(f"  BPE calibration max = {max(cal_bpe):.4f}", flush=True)

    print("Scoring calibration hunks on AST axis ...", flush=True)
    cal_ast = _score_calibration(ast_scorer, cal_records)
    print(f"  AST calibration max = {max(cal_ast):.4f}", flush=True)

    cal_ensemble = [_ensemble(b, a) for b, a in zip(cal_bpe, cal_ast, strict=True)]
    print(f"  Ensemble calibration max = {max(cal_ensemble):.4f}", flush=True)

    # Step 6: Score break fixtures on both axes
    print("\nLoading break fixtures manifest ...", flush=True)
    manifest = _load_breaks_manifest()

    print("Scoring breaks on BPE axis ...", flush=True)
    bpe_break_results = _score_breaks_bpe(
        manifest, tokenizer, id_to_token, model_a_bpe, total_a_bpe, model_b_bpe, total_b_bpe
    )

    print("Scoring breaks on AST axis ...", flush=True)
    ast_break_results = _score_breaks_ast(ast_scorer, manifest)

    # Merge into ensemble rows
    break_rows: list[dict[str, Any]] = []
    for bpe_r, ast_r in zip(bpe_break_results, ast_break_results, strict=True):
        assert bpe_r["name"] == ast_r["name"], "Fixture order mismatch"
        ens_score = _ensemble(bpe_r["bpe_score"], ast_r["ast_score"])
        row: dict[str, Any] = {
            "name": bpe_r["name"],
            "category": bpe_r["category"],
            "bpe_score": bpe_r["bpe_score"],
            "ast_score": ast_r["ast_score"],
            "ensemble_score": ens_score,
            "bpe_top_3_tokens": bpe_r["bpe_top_3_tokens"],
        }
        break_rows.append(row)
        print(
            f"  {bpe_r['name']:<35} bpe={bpe_r['bpe_score']:.4f}"
            f"  ast={ast_r['ast_score']:.4f}  ens={ens_score:.4f}",
            flush=True,
        )

    # Step 7: Write scores JSON
    _write_scores(fastapi_auc, break_rows, cal_bpe, cal_ast, cal_ensemble)

    # Step 8: Print summary
    _print_summary(fastapi_auc, break_rows, cal_bpe, cal_ast, cal_ensemble)

    return {
        "fastapi_auc": fastapi_auc,
        "break_rows": break_rows,
        "cal_bpe": cal_bpe,
        "cal_ast": cal_ast,
        "cal_ensemble": cal_ensemble,
    }


def main() -> None:
    run()


if __name__ == "__main__":
    main()
