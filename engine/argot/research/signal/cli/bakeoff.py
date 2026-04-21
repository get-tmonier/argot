"""Phase 12 S4 — MLM surprise + existing scorer bakeoff.

Runs a list of scorers (from REGISTRY) on a catalog entry, computes AUC
(overall and per-category), and writes a Markdown report plus a JSON scores file.

Usage::

    uv run --package argot-engine python -m argot.research.signal.cli.bakeoff \\
        --scorers mlm_surprise_mean,mlm_surprise_min,mlm_surprise_p05,tfidf_anomaly \\
        --context-mode file_only \\
        --entry fastapi \\
        --out docs/research/scoring/signal/phase12
"""

from __future__ import annotations

import argparse
import datetime
import json
from pathlib import Path
from typing import Any

# Side-effect imports — populate REGISTRY before lookup
import argot.research.signal.scorers.ast_structural  # noqa: F401
import argot.research.signal.scorers.knn_cosine  # noqa: F401
import argot.research.signal.scorers.lm_perplexity  # noqa: F401
import argot.research.signal.scorers.lof_embedding  # noqa: F401
import argot.research.signal.scorers.mlm_surprise  # noqa: F401
import argot.research.signal.scorers.tfidf_anomaly  # noqa: F401
from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
)
from argot.research.signal.base import REGISTRY, SignalScorer
from argot.research.signal.bootstrap import auc_from_scores, paired_bootstrap_ci

# Phase 11 production winner AUC for reference in report header
_PHASE11_WINNER_AUC = 0.6532
_PHASE11_WINNER_NAME = "EnsembleJepa mean_z @ file_only"

# Baseline scorer name for pairwise CI (picked as the simplest one if present)
_CI_BASELINE = "tfidf_anomaly"


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------


def _load_corpus_for_mode(entry_dir: Path, context_mode: str) -> list[dict[str, Any]]:
    """Load the corpus for the given context mode."""
    if context_mode == "baseline":
        return load_corpus(entry_dir)
    variant_path = entry_dir / f"corpus_{context_mode}.jsonl"
    if not variant_path.exists():
        raise FileNotFoundError(
            f"Variant corpus not found: {variant_path}\n"
            f"Run first: uv run --package argot-engine python -m "
            f"argot.research.signal.build_variant_corpus --mode {context_mode}"
        )
    corpus: list[dict[str, Any]] = []
    with variant_path.open() as f:
        for line in f:
            if line.strip():
                corpus.append(json.loads(line))
    return corpus


# ---------------------------------------------------------------------------
# Core bakeoff logic
# ---------------------------------------------------------------------------


def _run_bakeoff(
    entry_dir: Path,
    scorer_names: list[str],
    context_mode: str,
) -> dict[str, Any]:
    """Run all scorers on all fixtures; return structured results dict.

    Returns
    -------
    dict with keys:
      - ``scorer_names``: list[str]
      - ``fixtures``: list of {name, scope, is_break, category, set, scores: {scorer: float}}
      - ``scorer_aucs``: {scorer: {"overall": float, "by_category": {cat: float}}}
      - ``scorer_ci``: {scorer: {"delta": float, "ci_lo": float, "ci_hi": float}} | None
    """
    corpus = _load_corpus_for_mode(entry_dir, context_mode)
    specs: list[FixtureSpec] = load_manifest(entry_dir)
    load_scopes(entry_dir)  # validate entry has scopes.json; actual scope partition unused here

    # Build fixture records (context-aware)
    fixture_records: list[dict[str, Any]] = [
        fixture_to_record(entry_dir, spec, context_mode) for spec in specs
    ]

    # Fit + score each scorer across all scopes merged
    # (bakeoff operates on the merged corpus to match how phase11 sweep works)
    scorer_scores: dict[str, list[float]] = {}
    for name in scorer_names:
        if name not in REGISTRY:
            raise KeyError(f"Scorer {name!r} not in REGISTRY. Available: {sorted(REGISTRY)}")
        scorer: SignalScorer = REGISTRY[name]()
        print(f"  Fitting {name!r} ...", flush=True)
        scorer.fit(corpus)
        print(f"  Scoring {name!r} ...", flush=True)
        scorer_scores[name] = scorer.score(fixture_records)

    # Build per-fixture result dicts
    fixture_results: list[dict[str, Any]] = []
    for idx, spec in enumerate(specs):
        entry: dict[str, Any] = {
            "name": spec.name,
            "scope": spec.scope,
            "is_break": spec.is_break,
            "category": spec.category,
            "set": spec.set,
            "scores": {name: scorer_scores[name][idx] for name in scorer_names},
        }
        fixture_results.append(entry)

    # Compute AUC per scorer (overall + per-category)
    scorer_aucs: dict[str, dict[str, Any]] = {}
    for name in scorer_names:
        break_scores = [f["scores"][name] for f in fixture_results if f["is_break"]]
        ctrl_scores = [f["scores"][name] for f in fixture_results if not f["is_break"]]
        overall_auc = auc_from_scores(break_scores, ctrl_scores)

        categories = sorted({f["category"] for f in fixture_results})
        by_category: dict[str, float] = {}
        for cat in categories:
            cat_break = [
                f["scores"][name] for f in fixture_results if f["is_break"] and f["category"] == cat
            ]
            cat_ctrl = [
                f["scores"][name]
                for f in fixture_results
                if not f["is_break"] and f["category"] == cat
            ]
            by_category[cat] = auc_from_scores(cat_break, cat_ctrl)

        scorer_aucs[name] = {"overall": overall_auc, "by_category": by_category}

    # Bootstrap CI: each scorer vs _CI_BASELINE (if present)
    scorer_ci: dict[str, dict[str, float]] | None = None
    if _CI_BASELINE in scorer_names:
        scorer_ci = {}
        base_break = [f["scores"][_CI_BASELINE] for f in fixture_results if f["is_break"]]
        base_ctrl = [f["scores"][_CI_BASELINE] for f in fixture_results if not f["is_break"]]
        for name in scorer_names:
            var_break = [f["scores"][name] for f in fixture_results if f["is_break"]]
            var_ctrl = [f["scores"][name] for f in fixture_results if not f["is_break"]]
            delta, ci_lo, ci_hi = paired_bootstrap_ci(base_break, base_ctrl, var_break, var_ctrl)
            scorer_ci[name] = {"delta": delta, "ci_lo": ci_lo, "ci_hi": ci_hi}

    return {
        "scorer_names": scorer_names,
        "fixtures": fixture_results,
        "scorer_aucs": scorer_aucs,
        "scorer_ci": scorer_ci,
        "context_mode": context_mode,
        "entry": entry_dir.name,
    }


# ---------------------------------------------------------------------------
# Report writing
# ---------------------------------------------------------------------------


def _write_report(result: dict[str, Any], out_dir: Path) -> None:
    """Write Markdown report and JSON scores file."""
    out_dir.mkdir(parents=True, exist_ok=True)
    date_str = datetime.date.today().isoformat()
    md_path = out_dir / f"b_mlm_and_existing_{date_str}.md"
    json_path = out_dir / f"b_scores_{date_str}.json"

    scorer_names: list[str] = result["scorer_names"]
    fixtures: list[dict[str, Any]] = result["fixtures"]
    scorer_aucs: dict[str, dict[str, Any]] = result["scorer_aucs"]
    scorer_ci: dict[str, dict[str, float]] | None = result["scorer_ci"]
    context_mode: str = result["context_mode"]
    entry: str = result["entry"]
    categories = sorted({f["category"] for f in fixtures})

    lines: list[str] = []
    lines.append("# Phase 12 S4 — MLM Surprise Bakeoff\n")
    lines.append(f"Date: {date_str}  Entry: `{entry}`  Context mode: `{context_mode}`\n")
    lines.append(
        f"> Phase 11 production winner: **{_PHASE11_WINNER_NAME}** AUC={_PHASE11_WINNER_AUC:.4f}\n"
    )
    if scorer_ci is None:
        lines.append(
            f"> Pairwise CI vs `{_CI_BASELINE}` not computed "
            f"(scorer not in the bakeoff list).\n"
        )
    else:
        lines.append(
            f"> Pairwise CI computed vs `{_CI_BASELINE}` (paired bootstrap, n=1000, α=0.05).\n"
        )
    lines.append("")

    # Summary table
    lines.append("## Summary: Overall AUC\n")
    if scorer_ci is not None:
        lines.append("| scorer | overall_auc | auc_ci_lo | auc_ci_hi |")
        lines.append("|---|---|---|---|")
        for name in scorer_names:
            auc = scorer_aucs[name]["overall"]
            ci = scorer_ci[name]
            # ci_lo/ci_hi here are *delta* CI, not absolute AUC CI —
            # report them as delta_ci for clarity
            lines.append(f"| {name} | {auc:.4f} | {ci['ci_lo']:+.4f} | {ci['ci_hi']:+.4f} |")
    else:
        lines.append("| scorer | overall_auc |")
        lines.append("|---|---|")
        for name in scorer_names:
            auc = scorer_aucs[name]["overall"]
            lines.append(f"| {name} | {auc:.4f} |")
    lines.append("")

    # Per-category AUC matrix
    lines.append("## Per-Category AUC\n")
    cat_header = " | ".join(categories)
    lines.append(f"| scorer | {cat_header} |")
    lines.append("|---|" + "---|" * len(categories))
    for name in scorer_names:
        cat_cols = " | ".join(
            f"{scorer_aucs[name]['by_category'].get(cat, 0.5):.4f}" for cat in categories
        )
        lines.append(f"| {name} | {cat_cols} |")
    lines.append("")

    # Per-fixture scores table
    lines.append("## Per-Fixture Scores\n")
    scorer_header = " | ".join(scorer_names)
    lines.append(f"| fixture | category | type | {scorer_header} |")
    lines.append("|---|---|---|" + "---|" * len(scorer_names))
    for f in fixtures:
        ftype = "break" if f["is_break"] else "control"
        score_cols = " | ".join(f"{f['scores'].get(s, 0.0):.4f}" for s in scorer_names)
        lines.append(f"| {f['name']} | {f['category']} | {ftype} | {score_cols} |")
    lines.append("")

    md_path.write_text("\n".join(lines))
    print(f"Report written to {md_path}", flush=True)

    # JSON scores — same schema as phase11 scores JSON
    scores_json: dict[str, Any] = {
        "entry": entry,
        "context_mode": context_mode,
        "date": date_str,
        "scorers": scorer_names,
        "fixtures": [
            {
                "name": f["name"],
                "scope": f["scope"],
                "is_break": f["is_break"],
                "category": f["category"],
                "set": f["set"],
                "scores": f["scores"],
            }
            for f in fixtures
        ],
        "scorer_aucs": {
            name: {
                "overall": scorer_aucs[name]["overall"],
                "by_category": scorer_aucs[name]["by_category"],
            }
            for name in scorer_names
        },
    }
    json_path.write_text(json.dumps(scores_json, indent=2))
    print(f"Scores JSON written to {json_path}", flush=True)


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Phase 12 S4: bakeoff of MLM surprise + existing scorers"
    )
    parser.add_argument(
        "--scorers",
        default=(
            "mlm_surprise_mean,mlm_surprise_min,mlm_surprise_p05,"
            "tfidf_anomaly,knn_cosine,lof_embedding,lm_perplexity,"
            "ast_structural_ll,ast_structural_zscore,ast_structural_oov"
        ),
        help="Comma-separated scorer names from REGISTRY",
    )
    parser.add_argument(
        "--context-mode",
        default="file_only",
        dest="context_mode",
        help="Context mode for fixture loading (default: file_only)",
    )
    parser.add_argument(
        "--entry",
        default="fastapi",
        help="Catalog entry name (default: fastapi)",
    )
    parser.add_argument(
        "--catalog",
        default=str(CATALOG_DIR),
        help="Path to acceptance catalog directory",
    )
    parser.add_argument(
        "--out",
        default="docs/research/scoring/signal/phase12",
        help="Output directory for the report",
    )
    args = parser.parse_args()

    scorer_names = [s.strip() for s in args.scorers.split(",") if s.strip()]
    entry_dir = Path(args.catalog) / args.entry
    out_dir = Path(args.out)

    print(
        f"=== Phase 12 S4 bakeoff "
        f"entry={args.entry!r} context_mode={args.context_mode!r} "
        f"scorers={scorer_names} ===",
        flush=True,
    )

    result = _run_bakeoff(entry_dir, scorer_names, args.context_mode)
    _write_report(result, out_dir)

    # Print summary
    print("\nOverall AUC summary:")
    for name in scorer_names:
        auc = result["scorer_aucs"][name]["overall"]
        print(f"  {name}: {auc:.4f}")


if __name__ == "__main__":
    main()
