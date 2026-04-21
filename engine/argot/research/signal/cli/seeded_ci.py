"""
Phase 12 S0 — Multi-seed CI CLI.

Runs mean_z scoring under both ``file_only`` and ``baseline`` context modes on the
``fastapi`` fixture suite across seeds [0, 1, 2, 3, 4].  Computes a paired-bootstrap
95% CI on ΔAUC = AUC(file_only) − AUC(baseline) using the seed-0 fixture-level scores
and reports per-seed AUC variance.

Usage::

    uv run --package argot-engine python -m argot.research.signal.cli.seeded_ci \\
        --out docs/research/scoring/signal/phase12
"""

from __future__ import annotations

import argparse
import datetime
import json
import random
import statistics
from pathlib import Path
from typing import Any

import numpy as np
import torch

from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    ScopeConfig,
    fixture_to_record,
    load_corpus,
    load_manifest,
    load_scopes,
)
from argot.research.signal.bootstrap import auc_from_scores, paired_bootstrap_ci
from argot.research.signal.scorers.ensemble_jepa import EnsembleJepaScorer

# Production winner config (Stage 5 mean_z)
_MEAN_Z_CONFIG: dict[str, Any] = {
    "n": 3,
    "aggregation": "mean",
    "topk_k": 64,
    "zscore_vs_corpus": True,
}

SEEDS: list[int] = [0, 1, 2, 3, 4]
CONTEXT_MODES: list[str] = ["baseline", "file_only"]
ENTRY_NAME = "fastapi"
CORPUS_CAP = 2000


def _load_corpus_for_mode(entry_dir: Path, mode: str) -> list[dict[str, Any]]:
    """Load the corpus for the given context mode (raw or variant JSONL)."""
    if mode == "baseline":
        corpus = load_corpus(entry_dir)
    else:
        variant_path = entry_dir / f"corpus_{mode}.jsonl"
        if not variant_path.exists():
            raise FileNotFoundError(
                f"Variant corpus not found: {variant_path}\n"
                f"Run first: uv run --package argot-engine python -m "
                f"argot.research.signal.build_variant_corpus --mode {mode}"
            )
        corpus = []
        with variant_path.open() as f:
            for line in f:
                if line.strip():
                    corpus.append(json.loads(line))
    return corpus[:CORPUS_CAP]


def _score_one_run(
    entry_dir: Path,
    scopes: list[ScopeConfig],
    specs: list[FixtureSpec],
    corpus: list[dict[str, Any]],
    context_mode: str,
    seed: int,
) -> tuple[list[float], list[float]]:
    """Run one seed's scoring and return (break_scores, ctrl_scores)."""
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)

    break_scores: list[float] = []
    ctrl_scores: list[float] = []

    for scope in scopes:
        scope_corpus = sorted(
            [r for r in corpus if r.get("file_path", "").startswith(scope.path_prefix)],
            key=lambda r: int(r["author_date_iso"]),
        )
        if not scope_corpus:
            print(
                f"  WARNING: scope={scope.name!r} context={context_mode} seed={seed}: "
                "empty corpus — skipping"
            )
            continue

        scope_specs = [s for s in specs if s.scope == scope.name]
        fixture_records = [fixture_to_record(entry_dir, spec, context_mode) for spec in scope_specs]

        scorer = EnsembleJepaScorer(**_MEAN_Z_CONFIG)
        # Pre-encoding is skipped here: seeded_ci runs only once per experiment session,
        # so the re-encoding overhead (one pass per seed×mode) is acceptable vs. sweep.py's
        # corpus-pre-encode-once pattern which is designed for many configs.
        scorer.fit(scope_corpus, preencoded=None)
        scores = scorer.score(fixture_records)

        for idx, spec in enumerate(scope_specs):
            if spec.is_break:
                break_scores.append(scores[idx])
            else:
                ctrl_scores.append(scores[idx])

    return break_scores, ctrl_scores


def run_seeded_ci(
    entry_name: str = ENTRY_NAME,
    seeds: list[int] = SEEDS,
    catalog_dir: Path | None = None,
) -> dict[str, Any]:
    """
    Execute the full multi-seed seeded-CI run.

    Returns a dict with keys:
    - ``seed_aucs``: {mode -> [auc_seed0, ..., auc_seedN]}
    - ``baseline_scores_s0``: (break, ctrl) lists for seed 0 baseline
    - ``file_only_scores_s0``: (break, ctrl) lists for seed 0 file_only
    - ``delta``, ``ci_lo``, ``ci_hi``: bootstrap CI on seed-0 fixture-level scores
    """
    if catalog_dir is None:
        catalog_dir = CATALOG_DIR

    entry_dir = catalog_dir / entry_name
    scopes: list[ScopeConfig] = load_scopes(entry_dir)
    specs: list[FixtureSpec] = load_manifest(entry_dir)

    # Pre-load corpora for both modes (done once, reused across seeds)
    corpora: dict[str, list[dict[str, Any]]] = {}
    for mode in CONTEXT_MODES:
        print(f"Loading corpus for mode={mode!r} ...", flush=True)
        corpora[mode] = _load_corpus_for_mode(entry_dir, mode)

    seed_aucs: dict[str, list[float]] = {m: [] for m in CONTEXT_MODES}
    # Capture seed-0 fixture-level scores for CI computation
    s0_scores: dict[str, tuple[list[float], list[float]]] = {}

    for seed in seeds:
        for mode in CONTEXT_MODES:
            print(f"  seed={seed} mode={mode!r} ...", flush=True)
            break_s, ctrl_s = _score_one_run(entry_dir, scopes, specs, corpora[mode], mode, seed)
            auc = auc_from_scores(break_s, ctrl_s)
            seed_aucs[mode].append(auc)
            print(f"    AUC={auc:.4f}  breaks={len(break_s)}  ctrls={len(ctrl_s)}", flush=True)
            if seed == 0:
                s0_scores[mode] = (break_s, ctrl_s)

    # Paired bootstrap CI using seed-0 fixture-level scores
    base_break, base_ctrl = s0_scores["baseline"]
    fo_break, fo_ctrl = s0_scores["file_only"]
    delta, ci_lo, ci_hi = paired_bootstrap_ci(base_break, base_ctrl, fo_break, fo_ctrl)

    return {
        "seed_aucs": seed_aucs,
        "seeds": seeds,
        "baseline_scores_s0": s0_scores["baseline"],
        "file_only_scores_s0": s0_scores["file_only"],
        "delta": delta,
        "ci_lo": ci_lo,
        "ci_hi": ci_hi,
    }


def _write_report(result: dict[str, Any], out_dir: Path) -> None:
    """Write the Markdown report to out_dir/a_seeded_ci_<date>.md."""
    out_dir.mkdir(parents=True, exist_ok=True)
    date_str = datetime.date.today().isoformat()
    out_path = out_dir / f"a_seeded_ci_{date_str}.md"

    seed_aucs: dict[str, list[float]] = result["seed_aucs"]
    seeds: list[int] = result["seeds"]
    delta: float = result["delta"]
    ci_lo: float = result["ci_lo"]
    ci_hi: float = result["ci_hi"]

    lines: list[str] = []
    lines.append("# Phase 12 S0 — Seeded CI: file_only vs baseline (mean_z, fastapi)\n")
    lines.append(f"Date: {date_str}  Seeds: {seeds}\n")

    # Per-seed AUC table
    lines.append("## Per-seed AUC\n")
    lines.append("| seed | AUC(baseline) | AUC(file_only) | ΔAUC |")
    lines.append("|---|---|---|---|")
    for i, seed in enumerate(seeds):
        b_auc = seed_aucs["baseline"][i]
        fo_auc = seed_aucs["file_only"][i]
        d = fo_auc - b_auc
        lines.append(f"| {seed} | {b_auc:.4f} | {fo_auc:.4f} | {d:+.4f} |")
    lines.append("")

    # Summary row
    lines.append("## Mean AUC ± std\n")
    lines.append("| context_mode | mean_AUC | std_AUC |")
    lines.append("|---|---|---|")
    for mode in CONTEXT_MODES:
        aucs = seed_aucs[mode]
        mean_auc = statistics.mean(aucs)
        std_auc = statistics.stdev(aucs) if len(aucs) >= 2 else 0.0
        lines.append(f"| {mode} | {mean_auc:.4f} | {std_auc:.4f} |")
    lines.append("")

    # Bootstrap CI section
    lines.append("## ΔAUC Bootstrap 95% CI (seed-0 fixture-level scores)\n")
    lines.append(f"ΔAUC = AUC(file_only) − AUC(baseline) = **{delta:+.4f}**\n")
    lines.append(f"95% CI: [{ci_lo:+.4f}, {ci_hi:+.4f}]\n")

    # Verdict
    lines.append("## Verdict\n")
    if ci_lo > 0:
        lines.append(
            "**Phase 11 promotion statistically real** — "
            f"CI lower bound {ci_lo:+.4f} > 0; file_only context meaningfully improves AUC.\n"
        )
    else:
        lines.append(
            "**WARN: CI crosses zero** — "
            f"CI [{ci_lo:+.4f}, {ci_hi:+.4f}] includes 0; promotion not conclusively real.\n"
        )

    out_path.write_text("\n".join(lines))
    print(f"Report written to {out_path}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Phase 12 S0: multi-seed CI for mean_z file_only vs baseline"
    )
    parser.add_argument("--entry", default=ENTRY_NAME, help="Catalog entry name")
    parser.add_argument(
        "--seeds",
        default=",".join(str(s) for s in SEEDS),
        help="Comma-separated seed list (default: 0,1,2,3,4)",
    )
    parser.add_argument(
        "--catalog",
        default=str(CATALOG_DIR),
        help="Path to acceptance catalog directory",
    )
    parser.add_argument(
        "--out",
        default="docs/research/scoring/signal/phase12",
        help="Output directory for the Markdown report",
    )
    args = parser.parse_args()

    seeds = [int(s.strip()) for s in args.seeds.split(",")]
    catalog_dir = Path(args.catalog)
    out_dir = Path(args.out)

    print(
        f"=== Phase 12 S0: seeded_ci entry={args.entry!r} seeds={seeds} ===",
        flush=True,
    )

    result = run_seeded_ci(
        entry_name=args.entry,
        seeds=seeds,
        catalog_dir=catalog_dir,
    )

    _write_report(result, out_dir)

    # Print summary
    delta = result["delta"]
    ci_lo = result["ci_lo"]
    ci_hi = result["ci_hi"]
    print(f"\nDelta={delta:+.4f}  CI=[{ci_lo:+.4f}, {ci_hi:+.4f}]")
    if ci_lo > 0:
        print("VERDICT: Phase 11 promotion statistically real")
    else:
        print("VERDICT: WARN: CI crosses zero")


if __name__ == "__main__":
    main()
