# engine/argot/research/signal/phase13/experiments/ast_contrastive_rich.py
"""Phase 13 Experiment 15: AST-contrastive scoring on rich (terminal rendering) fixtures.

Adapts ast_contrastive_faker_breaks.py to the rich domain.  Uses the same 20 fixtures
(10 breaks + 10 controls) as bpe_contrastive_tfidf_rich.py so results are directly comparable.

Answers: does ContrastiveAstTreeletScorer generalise from FastAPI to rich (72 source files)?
Specifically, does AUC on rich clear the 0.85 threshold that would confirm the scorer is
not fundamentally FastAPI-tuned?

Steps:
  1. Sanity-check: reproduce Stage 2 FastAPI AUC ~0.9742.  Abort if outside [0.90, 1.00].
  2. Fit ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max', depth=3)
     on 72 rich source files (acceptance/catalog/rich/sources/model_a/).
  3. Score all 10 break + 10 control rich fixtures.
  4. Compute overall AUC + per-category AUC.
  5. Emit ast_contrastive_rich_scores.json.
  6. Print summary stats.

Usage:
    uv run python \\
        engine/argot/research/signal/phase13/experiments/ast_contrastive_rich.py
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from argot.acceptance.runner import FixtureSpec, fixture_to_record, load_manifest
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
_RICH_DIR = _CATALOG_DIR / "rich"
_RICH_MANIFEST_PATH = _RICH_DIR / "manifest.json"
_RICH_MODEL_A_DIR = _RICH_DIR / "sources" / "model_a"

_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "ast_contrastive_rich_scores.json"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _per_category_auc(
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
    ctrl_scores: list[float],
) -> dict[str, tuple[int, float]]:
    cat_breaks: dict[str, list[float]] = defaultdict(list)
    for s, b, cat in zip(scores, is_break, categories, strict=False):
        if b:
            cat_breaks[cat].append(s)
    return {
        cat: (len(cat_s), auc_from_scores(cat_s, ctrl_scores))
        for cat, cat_s in sorted(cat_breaks.items())
    }


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
# Step 2 — Fit on rich model_a sources
# ---------------------------------------------------------------------------


def _build_rich_scorer() -> ContrastiveAstTreeletScorer:
    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    model_a_files = sorted(_RICH_MODEL_A_DIR.glob("*.py"))
    print(f"Fitting on {len(model_a_files)} rich model_a source files ...", flush=True)
    scorer.fit([], model_a_files=model_a_files)
    total_a = scorer._total_a  # noqa: SLF001
    print(f"  model_A total treelets: {total_a}", flush=True)
    return scorer


# ---------------------------------------------------------------------------
# Step 3 — Load rich fixtures (same as bpe_contrastive_tfidf_rich.py)
# ---------------------------------------------------------------------------


def _load_rich_fixtures() -> tuple[list[dict[str, Any]], list[bool], list[str], list[str]]:
    manifest = json.loads(_RICH_MANIFEST_PATH.read_text(encoding="utf-8"))
    records: list[dict[str, Any]] = []
    is_break: list[bool] = []
    names: list[str] = []
    categories: list[str] = []
    for f in manifest["fixtures"]:
        spec = FixtureSpec(
            name=f["name"],
            scope="default",
            file=f["file"],
            hunk_start_line=f["hunk_start_line"],
            hunk_end_line=f["hunk_end_line"],
            is_break=f["is_break"],
            rationale=f.get("rationale", ""),
            category=f.get("category", "control" if not f["is_break"] else "break"),
        )
        records.append(fixture_to_record(_RICH_DIR, spec, "file_only"))
        is_break.append(spec.is_break)
        names.append(spec.name)
        categories.append(spec.category)
    return records, is_break, names, categories


# ---------------------------------------------------------------------------
# Step 4 — Score fixtures and compute AUC
# ---------------------------------------------------------------------------


def _score_and_eval(
    scorer: ContrastiveAstTreeletScorer,
    records: list[dict[str, Any]],
    is_break: list[bool],
    names: list[str],
    categories: list[str],
) -> tuple[list[float], float, dict[str, tuple[int, float]]]:
    scores = scorer.score(records)

    print("\nPer-fixture scores:", flush=True)
    for name, cat, b, s in zip(names, categories, is_break, scores, strict=False):
        label = "BREAK" if b else "ctrl"
        print(f"  [{label}] {name:<40} ({cat}): {s:.4f}", flush=True)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    return scores, overall_auc, per_cat


# ---------------------------------------------------------------------------
# Step 5 — Write scores JSON
# ---------------------------------------------------------------------------


def _write_scores(
    fastapi_auc: float,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    fixtures_out = [
        {
            "name": name,
            "category": cat,
            "is_break": b,
            "score": s,
        }
        for name, cat, b, s in zip(names, categories, is_break, scores, strict=False)
    ]
    per_cat_out = {cat: {"n_breaks": n, "auc": auc_val} for cat, (n, auc_val) in per_cat.items()}
    payload: dict[str, Any] = {
        "experiment": "ast_contrastive_rich",
        "scorer_params": {
            "epsilon": 1e-7,
            "aggregation": "max",
            "depth": 3,  # fixed in treelet_extractor.py (depth-1, -2, -3 treelets)
        },
        "model_a": "rich/sources/model_a/ (72 files)",
        "fastapi_sanity_auc": fastapi_auc,
        "overall_auc": overall_auc,
        "per_category_auc": per_cat_out,
        "fixtures": fixtures_out,
    }
    _SCORES_OUT.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"\nScores written to {_SCORES_OUT}", flush=True)


# ---------------------------------------------------------------------------
# Step 6 — Print summary
# ---------------------------------------------------------------------------


def _print_summary(
    fastapi_auc: float,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
) -> None:
    print("\n" + "=" * 70, flush=True)
    print("RESULTS SUMMARY", flush=True)
    print("=" * 70, flush=True)
    print(f"\nFastAPI sanity AUC:   {fastapi_auc:.4f}  (expected: ~0.9742)", flush=True)
    print(f"Rich overall AUC:     {overall_auc:.4f}", flush=True)
    print("BPE-tfidf rich AUC:  0.9900  (reference, exp #14)", flush=True)

    print("\nPer-category AUC:", flush=True)
    for cat, (n, cat_auc) in per_cat.items():
        print(f"  {cat:<20} n={n}  AUC={cat_auc:.4f}", flush=True)

    print("\nVerdict thresholds:", flush=True)
    if overall_auc >= 0.85:
        verdict = (
            f"AUC {overall_auc:.4f} >= 0.85 — AST-contrastive GENERALISES to rich. "
            "Corpus-starvation story confirmed: click (13 files) was too small; "
            "72 files are sufficient."
        )
    elif overall_auc >= 0.65:
        verdict = (
            f"AUC {overall_auc:.4f} in [0.65, 0.85) — PARTIAL generalisation. "
            "Corpus-starvation story plausible but weaker. Click failure not fully explained "
            "by corpus size alone."
        )
    else:
        verdict = (
            f"AUC {overall_auc:.4f} < 0.65 — AST-contrastive is GENUINELY FRAGILE cross-domain. "
            "Click's FastAPI-tuned verdict was correct; faker was the fluke."
        )
    print(f"\n{verdict}", flush=True)
    print("=" * 70, flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def run() -> dict[str, Any]:
    # Step 1: FastAPI sanity check
    fastapi_auc = _fastapi_sanity_check()

    # Step 2: Fit on rich sources
    print("\nFitting AST-contrastive scorer on rich ...", flush=True)
    scorer = _build_rich_scorer()

    # Step 3: Load rich fixtures
    print("\nLoading rich fixtures ...", flush=True)
    records, is_break, names, categories = _load_rich_fixtures()
    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    print(f"  Loaded {n_breaks} breaks + {n_ctrls} controls", flush=True)

    # Step 4: Score and compute AUC
    scores, overall_auc, per_cat = _score_and_eval(scorer, records, is_break, names, categories)

    # Step 5: Write scores file
    _write_scores(fastapi_auc, overall_auc, per_cat, names, scores, is_break, categories)

    # Step 6: Print summary
    _print_summary(fastapi_auc, overall_auc, per_cat)

    return {
        "fastapi_auc": fastapi_auc,
        "overall_auc": overall_auc,
        "per_cat": per_cat,
        "scores": scores,
        "is_break": is_break,
        "names": names,
        "categories": categories,
    }


def main() -> None:
    run()


if __name__ == "__main__":
    main()
