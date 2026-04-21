"""Holdout-partition experiment: scores each corpus hunk with a predictor not trained on it.

Hypothesis: the contamination bias (corpus scores systematically lower than fixtures)
disappears when corpus hunks are scored by a leave-one-block-out predictor.

Protocol
--------
K=3 interleaved blocks; each block is scored by the predictor trained on the other two.
Fixture scoring uses the mean across all three members (same as the current ensemble).

Winner config: b01_t01_w0  (beta=0.1, tau=0.1, warmup=0)
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


def _train_member(train_set: list[dict[str, Any]], seed: int) -> JepaInfoNCEScorer:
    m = JepaInfoNCEScorer(beta=BETA, tau=TAU, warmup_epochs=WARMUP, random_seed=seed)
    m.fit(train_set)
    return m


def _p95(values: list[float]) -> float:
    if not values:
        return float("nan")
    sorted_vals = sorted(values)
    idx = int(0.95 * len(sorted_vals))
    return sorted_vals[min(idx, len(sorted_vals) - 1)]


def _hunk_lines(record: dict[str, Any]) -> list[str]:
    """Reconstruct hunk source lines from hunk_tokens."""
    by_line: dict[int, list[str]] = {}
    for tok in record.get("hunk_tokens", []):
        by_line.setdefault(tok["start_line"], []).append(tok["text"])
    return [" ".join(by_line[ln]) for ln in sorted(by_line)]


def _print_hunk(rank: int, score: float, record: dict[str, Any]) -> None:
    file_path = record.get("file_path", "<unknown>")
    start = record.get("hunk_start_line", "?")
    end = record.get("hunk_end_line", "?")
    lines = _hunk_lines(record)
    max_lines = 15
    if len(lines) > max_lines:
        display = lines[:max_lines] + [f"... [{len(lines) - max_lines} more lines]"]
    else:
        display = lines
    print(f"#{rank:>2}  score={score:.4f}  {file_path}  L{start}-{end}")
    for line in display:
        print(f"    {line}")
    print("-" * 70)


def main() -> None:
    # --- Load or compute scored corpus hunks ---
    if CACHE_PATH.exists():
        print(f"Loading cached scores from {CACHE_PATH} ...")
        with CACHE_PATH.open("rb") as f:
            scored: list[tuple[float, dict[str, Any]]] = pickle.load(f)
        corpus_scores = [s for s, _ in scored]
        # Still need fixtures for stats
        all_fixtures: list[FixtureSpec] = load_manifest(ENTRY)
        controls = [fx for fx in all_fixtures if not fx.is_break]
        breaks = [fx for fx in all_fixtures if fx.is_break]
        # Can't recompute fixture scores without members; skip stats reprint
        control_scores: list[float] = []
        break_scores: list[float] = []
        print("(fixture scores not cached — stats block skipped)\n")
    else:
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
                f"\nTraining member[{holdout_idx}] on {len(train_set)} hunks "
                f"(holdout block={holdout_idx}) ...",
                flush=True,
            )
            members.append(_train_member(train_set, seed=holdout_idx))

        # Audit scoring: each corpus hunk scored by its holdout predictor
        scored = []
        corpus_scores = []
        for k, block in enumerate(blocks):
            block_scores = members[k].score(block)
            for s, rec in zip(block_scores, block, strict=True):
                scored.append((s, rec))
                corpus_scores.append(s)

        # Fixture scoring: ensemble mean over all three members
        all_fixtures = load_manifest(ENTRY)
        controls = [fx for fx in all_fixtures if not fx.is_break]
        breaks = [fx for fx in all_fixtures if fx.is_break]
        control_records = [fixture_to_record(ENTRY, s) for s in controls]
        break_records = [fixture_to_record(ENTRY, s) for s in breaks]

        def ensemble_score(records: list[dict[str, Any]]) -> list[float]:
            all_s = [m.score(records) for m in members]
            return [sum(run[i] for run in all_s) / K for i in range(len(records))]

        control_scores = ensemble_score(control_records)
        break_scores = ensemble_score(break_records)

        corpus_mean = statistics.mean(corpus_scores)
        corpus_p95 = _p95(corpus_scores)
        control_mean = statistics.mean(control_scores) if control_scores else float("nan")
        break_mean = statistics.mean(break_scores) if break_scores else float("nan")
        gap = corpus_mean - control_mean
        delta = break_mean - control_mean

        print("\n" + "=" * 60)
        print("HOLDOUT PARTITION EXPERIMENT RESULTS")
        print("=" * 60)
        print(f"corpus   mean={corpus_mean:.4f}  p95={corpus_p95:.4f}  n={len(corpus_scores)}")
        print(f"control  mean={control_mean:.4f}  n={len(control_scores)}")
        print(f"break    mean={break_mean:.4f}  n={len(break_scores)}")
        print(f"corpus−control gap: {gap:+.4f}  (was −0.6599)")
        print(f"break−control delta: {delta:+.4f}  (was +0.1087)")
        print()

        if abs(gap) < 0.10 and delta > 0.08:
            verdict = "WIN"
            detail = "Contamination resolved; fixture delta preserved."
        elif gap > -0.30:
            verdict = "PARTIAL"
            detail = "Gap closed by more than half; consider K=5 or stacking."
        else:
            verdict = "KILL"
            detail = "Bias structural — predictor-based audit fundamentally broken."

        print(f"VERDICT: {verdict} — {detail}")
        print("=" * 60)

        # Save cache
        CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
        with CACHE_PATH.open("wb") as f:  # type: ignore[assignment]
            pickle.dump(scored, f)
        print(f"\nScores cached to {CACHE_PATH}")

    # --- Top 20 corpus hunks ---
    sorted_scored = sorted(scored, key=lambda x: x[0], reverse=True)

    print("\n" + "=" * 70)
    print("TOP 20 CORPUS HUNKS (highest anomaly score)")
    print("=" * 70)
    for rank, (score, record) in enumerate(sorted_scored[:20], 1):
        _print_hunk(rank, score, record)

    print("\n" + "=" * 70)
    print("BOTTOM 5 CORPUS HUNKS (lowest anomaly score — most idiomatic)")
    print("=" * 70)
    for rank, (score, record) in enumerate(sorted_scored[-5:], 1):
        _print_hunk(len(sorted_scored) - 5 + rank, score, record)


if __name__ == "__main__":
    main()
