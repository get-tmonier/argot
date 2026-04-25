from __future__ import annotations

import argparse
import json
import sys
from datetime import UTC, datetime
from pathlib import Path

import pygit2

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
    cal_hunks = sample_hunks(source_dir, min(n_cal, len(candidates)), args.seed, adapter=adapter)

    print(f"Sampled {len(cal_hunks)} calibration hunks from {source_dir}")

    call_receiver_alpha: float = 2.0
    call_receiver_cap: int = 5
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=model_b_path,
        calibration_hunks=cal_hunks,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        threshold_percentile=args.threshold_percentile,
        threshold_iqr_k=args.threshold_iqr_k,
    )

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
            "n_cal": len(cal_hunks),
            "seed": args.seed,
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
