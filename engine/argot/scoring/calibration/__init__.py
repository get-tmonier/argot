from __future__ import annotations

import argparse
import json
import statistics
import sys
from datetime import UTC, datetime
from pathlib import Path

import pygit2

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.calibration.random_hunk_sampler import collect_candidates, sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CONFIG_VERSION = 1


def load_config(path: Path) -> dict[str, object]:
    """Load and validate scorer-config.json."""
    raw: dict[str, object] = json.loads(path.read_text(encoding="utf-8"))
    if raw.get("version") != _CONFIG_VERSION:
        raise ValueError(
            f"Unsupported scorer-config.json version: {raw.get('version')!r} "
            f"(expected {_CONFIG_VERSION})"
        )
    return raw


def calibrate_multi_seed(
    *,
    base_seed: int,
    n_seeds: int,
    n_cal: int,
    repo_dir: Path,
    model_a_files: list[Path],
    adapter: LanguageAdapter,
    bpe_model_b_path: Path,
    threshold_percentile: float | None = 95.0,
    threshold_iqr_k: float | None = None,
    call_receiver_alpha: float = 2.0,
    call_receiver_cap: int = 5,
    enable_typicality_filter: bool = True,
) -> float:
    """Run K independent calibrations; return median threshold.

    Runs n_seeds independent calibrations with seeds
    {base_seed, base_seed+1, ..., base_seed+n_seeds-1}.
    Each calibration samples n_cal hunks from repo_dir with its seed and computes a BPE threshold.
    Returns statistics.median(thresholds).

    Optimization: shares the tokenizer across K scorer builds to avoid K model downloads.
    """
    thresholds: list[float] = []

    # Build first scorer — lets the tokenizer load once
    first_hunks = sample_hunks(repo_dir, n_cal, base_seed, adapter=adapter)
    first_scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=bpe_model_b_path,
        calibration_hunks=first_hunks,
        adapter=adapter,
        threshold_percentile=threshold_percentile,
        threshold_iqr_k=threshold_iqr_k,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        enable_typicality_filter=enable_typicality_filter,
    )
    shared_tokenizer = first_scorer._tokenizer
    thresholds.append(first_scorer.bpe_threshold)

    # Build remaining scorers reusing the shared tokenizer
    for k in range(1, n_seeds):
        seed = base_seed + k
        hunks = sample_hunks(repo_dir, n_cal, seed, adapter=adapter)
        scorer = SequentialImportBpeScorer(
            model_a_files=model_a_files,
            bpe_model_b_path=bpe_model_b_path,
            calibration_hunks=hunks,
            adapter=adapter,
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            enable_typicality_filter=enable_typicality_filter,
            _tokenizer=shared_tokenizer,
        )
        thresholds.append(scorer.bpe_threshold)

    return statistics.median(thresholds)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Calibrate argot scorer and write scorer-config.json"
    )
    parser.add_argument("--repo", default=".", help="Path to target repository")
    parser.add_argument("--n-cal", type=int, default=500, help="Number of calibration hunks")
    parser.add_argument("--seed", type=int, default=0, help="RNG seed for hunk sampling")
    parser.add_argument(
        "--threshold-percentile",
        type=float,
        default=95.0,
        help=(
            "Percentile of calibration scores to use as BPE threshold (default 95.0 = p95). "
            "More robust than max to single high-scoring calibration outliers. "
            "Pass 100 to restore legacy max-of-calibration behaviour."
        ),
    )
    parser.add_argument(
        "--threshold-iqr-k",
        type=float,
        default=None,
        help=(
            "IQR-margin multiplier k: threshold = p75(cal_scores) + k * IQR. "
            "Overrides --threshold-percentile when set. Default None (use percentile/max)."
        ),
    )
    parser.add_argument(
        "--threshold-n-seeds",
        type=int,
        default=1,
        help=(
            "Number of independent calibration seeds for multi-seed median threshold. "
            "K independent calibrations are run (seeds: seed, seed+1, ..., seed+K-1); "
            "the median threshold is used. Default 1 (= single-seed, era-9 behavior)."
        ),
    )
    parser.add_argument(
        "--model-a",
        default=".argot/model_a.txt",
        help="File listing model-A source paths (produced by argot-train)",
    )
    parser.add_argument(
        "--model-b",
        default=".argot/model_b.json",
        help="Path to BPE reference JSON (produced by argot-train)",
    )
    parser.add_argument(
        "--output",
        default=".argot/scorer-config.json",
        help="Output path for scorer-config.json",
    )
    args = parser.parse_args()

    repo_path = Path(args.repo).resolve()
    model_a_path = Path(args.model_a)
    model_b_path = Path(args.model_b)

    if not model_a_path.exists():
        print(
            f"error: model_a file not found at {model_a_path} — run argot-train first",
            file=sys.stderr,
        )
        sys.exit(2)
    if not model_b_path.exists():
        print(
            f"error: model_b not found at {model_b_path} — run argot-train first",
            file=sys.stderr,
        )
        sys.exit(2)

    model_a_files = [Path(line) for line in model_a_path.read_text().splitlines() if line.strip()]
    if not model_a_files:
        print("error: model_a.txt is empty", file=sys.stderr)
        sys.exit(2)

    n_cal = args.n_cal
    adapter = adapter_for_files([str(p) for p in model_a_files])
    source_dir = repo_path
    candidates = collect_candidates(source_dir, adapter=adapter)
    effective_n_cal = min(n_cal, len(candidates))

    call_receiver_alpha: float = 2.0
    call_receiver_cap: int = 5

    if args.threshold_n_seeds > 1:
        print(
            f"Running {args.threshold_n_seeds} independent calibrations "
            f"(seeds {args.seed}–{args.seed + args.threshold_n_seeds - 1}, "
            f"n_cal={effective_n_cal} each) from {source_dir}"
        )
        threshold = calibrate_multi_seed(
            base_seed=args.seed,
            n_seeds=args.threshold_n_seeds,
            n_cal=effective_n_cal,
            repo_dir=source_dir,
            model_a_files=model_a_files,
            adapter=adapter,
            bpe_model_b_path=model_b_path,
            threshold_percentile=args.threshold_percentile,
            threshold_iqr_k=args.threshold_iqr_k,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
        )
        scorer = SequentialImportBpeScorer(
            model_a_files=model_a_files,
            bpe_model_b_path=model_b_path,
            bpe_threshold=threshold,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
        )
        n_cal_used = effective_n_cal
    else:
        cal_hunks = sample_hunks(source_dir, effective_n_cal, args.seed, adapter=adapter)
        print(f"Sampled {len(cal_hunks)} calibration hunks from {source_dir}")
        scorer = SequentialImportBpeScorer(
            model_a_files=model_a_files,
            bpe_model_b_path=model_b_path,
            calibration_hunks=cal_hunks,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            threshold_percentile=args.threshold_percentile,
            threshold_iqr_k=args.threshold_iqr_k,
        )
        n_cal_used = scorer.n_calibration

    try:
        repo = pygit2.Repository(str(repo_path))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    config: dict[str, object] = {
        "version": _CONFIG_VERSION,
        "threshold": scorer.bpe_threshold,
        "call_receiver_alpha": call_receiver_alpha,
        "call_receiver_cap": call_receiver_cap,
        "calibration": {
            "n_cal": n_cal_used,
            "seed": args.seed,
            "n_seeds": args.threshold_n_seeds,
            "repo_sha": repo_sha,
            "timestamp_utc": datetime.now(tz=UTC).isoformat(),
        },
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(config, indent=2))
    print(f"threshold: {scorer.bpe_threshold:.4f} → {out_path}")


if __name__ == "__main__":
    main()
