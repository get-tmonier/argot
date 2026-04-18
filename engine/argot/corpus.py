from __future__ import annotations

import argparse
import json
import random
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import numpy as np

from argot.train import train_model
from argot.validate import (
    compute_auc,
    inject_foreign,
    score_records,
    shuffle_negatives,
    split_by_time,
)


def stratified_downsample(
    records: list[dict[str, Any]],
    target_size: int,
    seed: int,
) -> list[dict[str, Any]]:
    """Deterministically sample `target_size` records, preserving per-`_repo` proportions.

    If target_size >= len(records), returns all records. Per-repo picks are floor(target * share)
    with the remainder filled by largest-repo picks to hit target_size exactly when possible.
    """
    if target_size >= len(records):
        return list(records)

    by_repo: dict[str, list[dict[str, Any]]] = {}
    for r in records:
        by_repo.setdefault(r["_repo"], []).append(r)

    total = len(records)
    rng = random.Random(seed)

    sampled: list[dict[str, Any]] = []
    for repo in sorted(by_repo):
        share = round(target_size * len(by_repo[repo]) / total)
        share = min(share, len(by_repo[repo]))
        sampled.extend(rng.sample(by_repo[repo], share))

    return sampled


def _load_records(path: Path, max_records: int | None = None) -> list[dict[str, Any]]:
    """Stream JSONL keeping only the four fields the benchmark needs.

    Avoids loading the full raw JSON into a single string (can be GB-scale for
    large corpora) and drops unused token fields (type, line, column, …) which
    dominate per-record size.

    If max_records is given, applies Vitter's reservoir-sampling algorithm R so
    the in-memory footprint is bounded regardless of input file size. The sample
    is representative: each record has equal probability of inclusion.
    """
    reservoir: list[dict[str, Any]] = []
    rng = random.Random(0)
    seen = 0
    with path.open() as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            slim = {
                "_repo": r["_repo"],
                "author_date_iso": r["author_date_iso"],
                "context_before": [{"text": t["text"]} for t in r["context_before"]],
                "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
            }
            seen += 1
            if max_records is None or len(reservoir) < max_records:
                reservoir.append(slim)
            else:
                j = rng.randint(0, seen - 1)
                if j < max_records:
                    reservoir[j] = slim
    return reservoir


def run_benchmark(
    *,
    dataset: Path,
    sizes: list[int],
    seeds: int,
    output: Path,
    epochs: int = 20,
    batch_size: int = 128,
) -> None:
    """Run validate-style AUC measurement at each (size, seed); append to output JSONL."""
    # Load at most 2× the largest target — enough headroom for stratified_downsample
    # while bounding peak RSS regardless of input file size.
    records = _load_records(dataset, max_records=max(sizes) * 2)

    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("a") as out_fh:
        for size in sizes:
            for seed in range(seeds):
                row = _benchmark_one(
                    records=records,
                    size=size,
                    seed=seed,
                    epochs=epochs,
                    batch_size=batch_size,
                )
                out_fh.write(json.dumps(row) + "\n")
                out_fh.flush()
                print(
                    f"size={size:>6d} seed={seed}  "
                    f"shuffled={row['shuffled_auc']:.3f}  "
                    f"cross={row['cross_auc']:.3f}  "
                    f"injected={row['injected_auc']:.3f}"
                )


def _benchmark_one(
    *,
    records: list[dict[str, Any]],
    size: int,
    seed: int,
    epochs: int,
    batch_size: int,
) -> dict[str, Any]:
    sample = stratified_downsample(records, target_size=size, seed=seed)

    repo_groups: dict[str, list[dict[str, Any]]] = {}
    for r in sample:
        repo_groups.setdefault(r["_repo"], []).append(r)

    foreign_name = min(repo_groups, key=lambda n: len(repo_groups[n]))
    foreign = repo_groups[foreign_name]
    home = [r for r in sample if r["_repo"] != foreign_name]

    train_records, held_out = split_by_time(home, ratio=0.8)
    bundle = train_model(train_records, epochs=epochs, batch_size=batch_size)

    good = score_records(bundle, held_out)
    shuffled = score_records(bundle, shuffle_negatives(held_out, seed=seed))
    cross = score_records(bundle, foreign)
    injected = score_records(bundle, inject_foreign(held_out, foreign, seed=seed))

    good_arr = np.array(good) if good else np.array([0.0])
    return {
        "size": size,
        "seed": seed,
        "n_repos": len(repo_groups),
        "n_train": len(train_records),
        "n_held_out": len(held_out),
        "n_foreign": len(foreign),
        "shuffled_auc": compute_auc(good, shuffled),
        "cross_auc": compute_auc(good, cross),
        "injected_auc": compute_auc(good, injected),
        "good_median": float(np.median(good_arr)),
        "good_p95": float(np.percentile(good_arr, 95)),
        "trained_at": datetime.now(UTC).isoformat(),
    }


def _cmd_benchmark(args: argparse.Namespace) -> int:
    dataset = Path(args.dataset)
    if not dataset.exists():
        print(f"error: dataset not found: {dataset}", file=sys.stderr)
        return 2
    sizes = [int(s) for s in args.sizes.split(",")]
    run_benchmark(
        dataset=dataset,
        sizes=sizes,
        seeds=args.seeds,
        output=Path(args.out),
        epochs=args.epochs,
        batch_size=args.batch_size,
    )
    return 0


def concat_datasets(inputs: list[Path], output: Path) -> dict[str, int]:
    """Concatenate tagged JSONL datasets; return per-repo record counts.

    Every record must carry a `_repo` tag (set by `argot-extract --repo-name`).
    """
    counts: dict[str, int] = {}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w") as out_fh:
        for src in inputs:
            for line in src.read_text().splitlines():
                if not line.strip():
                    continue
                record = json.loads(line)
                if "_repo" not in record:
                    raise ValueError(
                        f"record in {src} missing _repo tag " f"(re-extract with --repo-name)"
                    )
                counts[record["_repo"]] = counts.get(record["_repo"], 0) + 1
                out_fh.write(line + "\n")
    return counts


def _cmd_concat(args: argparse.Namespace) -> int:
    inputs = [Path(p) for p in args.inputs]
    for p in inputs:
        if not p.exists():
            print(f"error: input not found: {p}", file=sys.stderr)
            return 2
    counts = concat_datasets(inputs, Path(args.out))
    total = sum(counts.values())
    print(f"wrote {total} records to {args.out}")
    for repo, n in sorted(counts.items()):
        print(f"  {repo}: {n}")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="argot corpus utilities")
    sub = parser.add_subparsers(dest="cmd", required=True)

    concat_p = sub.add_parser("concat", help="Concatenate tagged JSONL datasets")
    concat_p.add_argument("inputs", nargs="+", help="Input JSONL paths")
    concat_p.add_argument("-o", "--out", required=True, help="Output JSONL path")
    concat_p.set_defaults(func=_cmd_concat)

    bench_p = sub.add_parser("benchmark", help="Run AUC benchmark across sizes × seeds")
    bench_p.add_argument("--dataset", required=True, help="Combined tagged JSONL")
    bench_p.add_argument(
        "--sizes",
        default="500,2000,8000",
        help="Comma-separated target dataset sizes",
    )
    bench_p.add_argument("--seeds", type=int, default=3, help="Runs per size (seeds 0..N-1)")
    bench_p.add_argument(
        "--out", default=".argot/research/results.jsonl", help="Append results to this JSONL"
    )
    bench_p.add_argument("--epochs", type=int, default=20)
    bench_p.add_argument("--batch-size", type=int, default=128)
    bench_p.set_defaults(func=_cmd_benchmark)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
