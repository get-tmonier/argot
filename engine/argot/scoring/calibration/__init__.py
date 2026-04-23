from __future__ import annotations

import argparse
import json
import sys
from datetime import UTC, datetime
from pathlib import Path

import pygit2

from argot.scoring.calibration.random_hunk_sampler import sample_hunks
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
    all_py = [p for p in model_a_files if p.suffix == ".py"]
    source_dir = repo_path
    cal_hunks = sample_hunks(source_dir, min(n_cal, max(1, len(all_py) * 5)), args.seed)

    print(f"Sampled {len(cal_hunks)} calibration hunks from {source_dir}")

    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=model_b_path,
        calibration_hunks=cal_hunks,
    )

    try:
        repo = pygit2.Repository(str(repo_path))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    config: dict[str, object] = {
        "version": _CONFIG_VERSION,
        "threshold": scorer.bpe_threshold,
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
