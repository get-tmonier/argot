"""Holdout-partition category delta test: v1 vs v2 regression analysis.

Reuses the K=3 holdout-partition ensemble from holdout_partition_test.py
(cached in .argot/holdout_top20_cache.pkl if available) and computes
per-set (v1/v2) deltas to determine whether the 14% aggregate drop is
uniform or concentrated.

Baseline (ensemble_jepa Stage 6 b01_t01_w0+ensemble_n3, seeds 42–46):
  delta_v1 = 0.2291  (source: docs/research/scoring/signal/sweep_fastapi_stage6_2026-04-20.md)
  delta_v2 = 0.1087  (source: docs/research/scoring/signal/sweep_fastapi_v2_baseline_2026-04-20.md)
"""

from __future__ import annotations

import pickle
import random
import statistics
from pathlib import Path
from typing import Any

from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    fixture_to_record,
    load_corpus,
    load_manifest,
)
from argot.research.signal.scorers.jepa_infonce import JepaInfoNCEScorer

ENTRY = CATALOG_DIR / "fastapi"
K = 3
BETA = 0.1
TAU = 0.1
WARMUP = 0
CACHE_PATH = Path(".argot/holdout_top20_cache.pkl")

# Baseline numbers from ensemble_jepa Stage 6 (b01_t01_w0+ensemble_n3, seeds 42–46)
BASELINE_DELTA_V1 = 0.2291
BASELINE_DELTA_V2 = 0.1087


def _train_member(train_set: list[dict[str, Any]], seed: int) -> JepaInfoNCEScorer:
    m = JepaInfoNCEScorer(beta=BETA, tau=TAU, warmup_epochs=WARMUP, random_seed=seed)
    m.fit(train_set)
    return m


def _ensemble_score(members: list[JepaInfoNCEScorer], records: list[dict[str, Any]]) -> list[float]:
    all_s = [m.score(records) for m in members]
    return [sum(run[i] for run in all_s) / K for i in range(len(records))]


def _score_fixture_group(
    members: list[JepaInfoNCEScorer],
    fixtures: list[FixtureSpec],
    label: str,
) -> tuple[float, float, float, int, int]:
    """Return (control_mean, break_mean, delta, n_controls, n_breaks)."""
    controls = [fx for fx in fixtures if not fx.is_break]
    breaks = [fx for fx in fixtures if fx.is_break]

    if not controls or not breaks:
        print(f"  WARNING: {label} has {len(controls)} controls, {len(breaks)} breaks — skipping")
        return float("nan"), float("nan"), float("nan"), len(controls), len(breaks)

    ctrl_records = [fixture_to_record(ENTRY, fx) for fx in controls]
    brk_records = [fixture_to_record(ENTRY, fx) for fx in breaks]

    ctrl_scores = _ensemble_score(members, ctrl_records)
    brk_scores = _ensemble_score(members, brk_records)

    ctrl_mean = statistics.mean(ctrl_scores)
    brk_mean = statistics.mean(brk_scores)
    delta = brk_mean - ctrl_mean
    return ctrl_mean, brk_mean, delta, len(controls), len(breaks)


def main() -> None:
    print("=" * 60)
    print("ensemble_jepa baseline (from existing reports)")
    print("=" * 60)
    print(
        "Source: docs/research/scoring/signal/sweep_fastapi_stage6_2026-04-20.md"
        " + sweep_fastapi_v2_baseline_2026-04-20.md"
    )
    print(f"delta_v1 = {BASELINE_DELTA_V1:.4f}")
    print(f"delta_v2 = {BASELINE_DELTA_V2:.4f}")
    print()

    all_fixtures: list[FixtureSpec] = load_manifest(ENTRY)
    v1_fixtures = [fx for fx in all_fixtures if fx.set == "v1"]
    v2_fixtures = [fx for fx in all_fixtures if fx.set == "v2"]

    if CACHE_PATH.exists():
        print(f"Cache found at {CACHE_PATH} — retraining members for fixture scoring ...")
        # Cache only contains corpus scores; we still need trained members for fixtures.
        # Retrain with identical protocol so fixture scores match.

    random.seed(0)
    corpus = load_corpus(ENTRY)
    random.shuffle(corpus)

    blocks: list[list[dict[str, Any]]] = [corpus[k::K] for k in range(K)]
    for k, b in enumerate(blocks):
        print(f"block[{k}]: {len(b)} hunks")

    members: list[JepaInfoNCEScorer] = []
    for holdout_idx in range(K):
        train_set = [h for k, b in enumerate(blocks) for h in b if k != holdout_idx]
        print(
            f"Training member[{holdout_idx}] on {len(train_set)} hunks "
            f"(holdout block={holdout_idx}) ...",
            flush=True,
        )
        members.append(_train_member(train_set, seed=holdout_idx))

    print()

    ctrl_v1, brk_v1, delta_v1, nc_v1, nb_v1 = _score_fixture_group(members, v1_fixtures, "v1")
    ctrl_v2, brk_v2, delta_v2, nc_v2, nb_v2 = _score_fixture_group(members, v2_fixtures, "v2")

    print("=" * 60)
    print("holdout_partition_jepa (this run)")
    print("=" * 60)
    print(
        f"v1: control_mean={ctrl_v1:.4f} (n={nc_v1})  "
        f"break_mean={brk_v1:.4f} (n={nb_v1})  "
        f"delta_v1={delta_v1:+.4f}"
    )
    print(
        f"v2: control_mean={ctrl_v2:.4f} (n={nc_v2})  "
        f"break_mean={brk_v2:.4f} (n={nb_v2})  "
        f"delta_v2={delta_v2:+.4f}"
    )
    print()

    change_v1 = delta_v1 - BASELINE_DELTA_V1
    change_v2 = delta_v2 - BASELINE_DELTA_V2

    print("=" * 60)
    print("Regression analysis")
    print("=" * 60)
    print(f"delta_v1 change: {change_v1:+.4f}  (absolute)")
    print(f"delta_v2 change: {change_v2:+.4f}  (absolute)")
    print()

    # Decision gate
    v1_neg = delta_v1 < 0
    v2_neg = delta_v2 < 0
    v1_large_drop = abs(change_v1) > 0.05
    v2_large_drop = abs(change_v2) > 0.05
    v1_moderate_drop = abs(change_v1) > 0.02
    v2_moderate_drop = abs(change_v2) > 0.02

    print("=" * 60)
    print("Decision")
    print("=" * 60)
    if v1_neg or v2_neg or v1_large_drop or v2_large_drop:
        verdict = "DO NOT PROMOTE"
        reason = "delta goes negative" if (v1_neg or v2_neg) else "drop exceeds 0.05 absolute"
    elif v1_moderate_drop or v2_moderate_drop:
        verdict = "PROMOTE AS AUDIT-ONLY"
        reason = "delta drops > 0.02 absolute but both remain positive"
    else:
        verdict = "SAFE TO PROMOTE AS DEFAULT"
        reason = "both delta_v1 and delta_v2 drop by ≤ 0.02 absolute"
    print(f"{verdict}: {reason}")
    print("=" * 60)

    # Save cache if not already present
    if not CACHE_PATH.exists():
        scored_corpus: list[tuple[float, dict[str, Any]]] = []
        for k, block in enumerate(blocks):
            block_scores = members[k].score(block)
            for s, rec in zip(block_scores, block, strict=True):
                scored_corpus.append((s, rec))
        CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
        with CACHE_PATH.open("wb") as f:
            pickle.dump(scored_corpus, f)
        print(f"\nCorpus scores cached to {CACHE_PATH}")


if __name__ == "__main__":
    main()
