from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path
from typing import Any

import numpy as np
import torch

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
    header = f"{'':25s}" + "".join(f"{k:>10s}" for k in keys)
    print("\n=== Score Distribution ===")
    print(header)
    print("-" * len(header))
    for label, p in rows:
        row = f"{label:25s}" + "".join(f"{p[k]:10.4f}" for k in keys)
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
    train_records, held_out = split_by_time(records, ratio=args.ratio)
    print(f"Split: {len(train_records)} train / {len(held_out)} held-out")

    print(f"\nTraining on {len(train_records)} records ({args.epochs} epochs)...")
    bundle = train_model(train_records, epochs=args.epochs, batch_size=args.batch_size)

    print("\nScoring held-out (good) set...")
    good_scores = score_records(bundle, held_out)

    print("Scoring shuffled negatives...")
    negatives = shuffle_negatives(held_out, seed=42)
    bad_scores = score_records(bundle, negatives)

    _print_table(
        [
            ("Good (held-out)", compute_percentiles(good_scores)),
            ("Bad (shuffled)", compute_percentiles(bad_scores)),
        ]
    )

    good_median = compute_percentiles(good_scores)["median"]
    bad_median = compute_percentiles(bad_scores)["median"]
    gm, bm = f"{good_median:.4f}", f"{bad_median:.4f}"
    if bad_median > good_median:
        print(f"✓ Model shows separation: bad median ({bm}) > good median ({gm})")
    else:
        print(f"✗ No separation detected: bad median ({bm}) <= good median ({gm})")
        sys.exit(1)


if __name__ == "__main__":
    main()
