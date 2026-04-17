from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path
from typing import Any

import numpy as np
import torch
from sklearn.metrics import roc_auc_score  # type: ignore[import-untyped]

from argot.train import ModelBundle, train_model


def split_by_time(
    records: list[dict[str, Any]], *, ratio: float = 0.8
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    sorted_records = sorted(records, key=lambda r: int(r["author_date_iso"]))
    split_idx = int(len(sorted_records) * ratio)
    return sorted_records[:split_idx], sorted_records[split_idx:]


def shuffle_negatives(records: list[dict[str, Any]], *, seed: int = 0) -> list[dict[str, Any]]:
    rng = random.Random(seed)
    hunk_pool = [r["hunk_tokens"] for r in records]
    shuffled_hunks = hunk_pool[:]
    rng.shuffle(shuffled_hunks)
    return [{**r, "hunk_tokens": h} for r, h in zip(records, shuffled_hunks, strict=False)]


def inject_foreign(
    home: list[dict[str, Any]], foreign: list[dict[str, Any]], *, seed: int = 0
) -> list[dict[str, Any]]:
    """Replace hunk_tokens in home records with hunks randomly drawn from a foreign repo."""
    rng = random.Random(seed)
    foreign_hunks = [r["hunk_tokens"] for r in foreign]
    return [{**r, "hunk_tokens": rng.choice(foreign_hunks)} for r in home]


def compute_percentiles(scores: list[float]) -> dict[str, float]:
    arr = np.array(scores)
    return {
        "min": float(np.min(arr)),
        "p25": float(np.percentile(arr, 25)),
        "median": float(np.median(arr)),
        "p75": float(np.percentile(arr, 75)),
        "p95": float(np.percentile(arr, 95)),
        "max": float(np.max(arr)),
    }


def compute_auc(good_scores: list[float], bad_scores: list[float]) -> float:
    labels = [0] * len(good_scores) + [1] * len(bad_scores)
    scores = good_scores + bad_scores
    return float(roc_auc_score(labels, scores))


def score_records(bundle: ModelBundle, records: list[dict[str, Any]]) -> list[float]:
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]
    ctx_x = torch.tensor(bundle.vectorizer.transform(ctx_texts).toarray(), dtype=torch.float32)
    hunk_x = torch.tensor(bundle.vectorizer.transform(hunk_texts).toarray(), dtype=torch.float32)
    bundle.model.eval()
    with torch.no_grad():
        scores = [
            bundle.model.surprise(ctx_x[i : i + 1], hunk_x[i : i + 1]).item()
            for i in range(len(records))
        ]
    return scores


def _print_table(rows: list[tuple[str, dict[str, float]]]) -> None:
    keys = ["min", "p25", "median", "p75", "p95", "max"]
    header = f"{'':30s}" + "".join(f"{k:>10s}" for k in keys) + f"{'AUC':>8s}"
    print("\n=== Score Distribution ===")
    print(header)
    print("-" * len(header))
    for label, p, auc in rows:  # type: ignore[misc]
        row = f"{label:30s}" + "".join(f"{p[k]:10.4f}" for k in keys) + f"{auc:8.4f}"
        print(row)
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate argot model on train/held-out split")
    parser.add_argument("--dataset", default=".argot/training.jsonl")
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--batch-size", type=int, default=128)
    parser.add_argument("--ratio", type=float, default=0.8)
    args = parser.parse_args()

    dataset_path = Path(args.dataset)
    if not dataset_path.exists():
        print(f"error: dataset not found at {dataset_path}", file=sys.stderr)
        sys.exit(2)

    records = [json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()]
    if len(records) < 10:
        print("error: dataset too small for validation", file=sys.stderr)
        sys.exit(2)

    print(f"Loaded {len(records)} records from {dataset_path}")

    # Partition by repo — smallest becomes the foreign (unseen) test set
    repo_groups: dict[str, list[dict[str, Any]]] = {}
    for r in records:
        repo_groups.setdefault(r.get("_repo", "unknown"), []).append(r)

    has_cross_repo = len(repo_groups) >= 2
    if has_cross_repo:
        foreign_name = min(repo_groups, key=lambda n: len(repo_groups[n]))
        foreign = repo_groups[foreign_name]
        home = [r for r in records if r.get("_repo") != foreign_name]
        print(
            f"Repos: {list(repo_groups)} — using '{foreign_name}'"
            f" ({len(foreign)} records) as foreign set"
        )
    else:
        home = records
        foreign = []

    train_records, held_out = split_by_time(home, ratio=args.ratio)
    print(f"Split: {len(train_records)} train / {len(held_out)} held-out")

    print(f"\nTraining on {len(train_records)} records ({args.epochs} epochs)...")
    bundle = train_model(train_records, epochs=args.epochs, batch_size=args.batch_size)

    print("\nScoring...")
    good_scores = score_records(bundle, held_out)
    shuffled_scores = score_records(bundle, shuffle_negatives(held_out, seed=42))
    cross_scores = score_records(bundle, foreign) if has_cross_repo else []
    injected_scores = (
        score_records(bundle, inject_foreign(held_out, foreign, seed=42)) if has_cross_repo else []
    )

    good_p = compute_percentiles(good_scores)
    shuffled_auc = compute_auc(good_scores, shuffled_scores)
    rows: list[Any] = [
        ("Good (held-out)", good_p, 0.5),
        ("Bad — shuffled tokens", compute_percentiles(shuffled_scores), shuffled_auc),
    ]
    if has_cross_repo:
        cross_auc = compute_auc(good_scores, cross_scores)
        injected_auc = compute_auc(good_scores, injected_scores)
        rows += [
            ("Bad — cross-repo foreign", compute_percentiles(cross_scores), cross_auc),
            ("Bad — injected foreign hunk", compute_percentiles(injected_scores), injected_auc),
        ]

    _print_table(rows)

    good_median = good_p["median"]
    checks: dict[str, tuple[bool, float]] = {
        "shuffled": (
            compute_percentiles(shuffled_scores)["median"] > good_median,
            shuffled_auc,
        ),
    }
    if has_cross_repo:
        checks["cross-repo"] = (
            compute_percentiles(cross_scores)["median"] > good_median,
            cross_auc,
        )
        checks["injected"] = (
            compute_percentiles(injected_scores)["median"] > good_median,
            injected_auc,
        )

    all_pass = True
    for name, (sep, auc) in checks.items():
        icon = "✓" if sep else "✗"
        auc_icon = "✓" if auc > 0.6 else "✗"
        auc_label = "ok" if auc > 0.6 else "FAIL — below 0.6"
        sep_label = "ok" if sep else "FAIL"
        print(
            f"  {icon} [{name}] median separation {sep_label}"
            f"   {auc_icon} AUC={auc:.4f} ({auc_label})"
        )
        if not sep or auc <= 0.6:
            all_pass = False

    print()
    if all_pass:
        print("✓ All validation checks passed")
    else:
        print("✗ One or more validation checks failed")
        sys.exit(1)


if __name__ == "__main__":
    main()
