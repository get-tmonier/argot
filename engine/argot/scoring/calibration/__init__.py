from __future__ import annotations

import argparse
import json
import statistics
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import pygit2

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.calibration.evidence_builder import build_evidence_corpus
from argot.scoring.calibration.random_hunk_sampler import (
    collect_candidates,
    sample_hunks,
    sample_hunks_with_metadata,
)
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
from argot.scoring.scorers.shape_primitive import ShapePrimitive
from argot.scoring.scorers.shape_primitive_registry import build_shape_primitives

_CONFIG_VERSION = 1
# Top-N sample size baked into the evidence_corpus block. 50 is comfortably
# above the rendered top-3 + ``(+N more)`` cap and leaves headroom for future
# UX tweaks without a re-calibration. Configurable on the calibration CLI.
_DEFAULT_EVIDENCE_TOP_N = 50


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
    repo_corpus_files: list[Path],
    adapter: LanguageAdapter,
    bpe_generic_baseline_path: Path,
    threshold_percentile: float | None = 95.0,
    threshold_iqr_k: float | None = None,
    call_receiver_alpha: float = 2.0,
    call_receiver_cap: int = 5,
    call_receiver_root_bonus: float = 2.0,
    call_receiver_n_clusters: int = 8,
    call_receiver_cluster_seed: int = 0,
    call_receiver_cluster_bonus: float = 5.0,
    call_receiver_cluster_rare_threshold: int = 0,
    call_receiver_cluster_size_min: int = 0,
    call_receiver_shape_primitive_names: tuple[str, ...] = (),
    enable_typicality_filter: bool = True,
    apply_optional_contributions_to_cal: bool = False,
) -> float:
    """Run K independent calibrations; return median threshold.

    Runs n_seeds independent calibrations with seeds
    {base_seed, base_seed+1, ..., base_seed+n_seeds-1}.
    Each calibration samples n_cal hunks from repo_dir with its seed and computes a BPE threshold.
    Returns statistics.median(thresholds).

    Optimization: shares the tokenizer across K scorer builds to avoid K model downloads.

    G7 calibration contract
    -----------------------
    The calibration threshold is a per-cluster bound on what typical code scores under the
    base scorer plus the era-11 cluster_bonus contribution. That contribution is
    asymmetric-by-construction: calibration hunks come from repo-corpus files whose callees are a
    subset of their cluster's attested set, so the cluster-absent-callee branch of
    weighted_contribution_for_file does not fire on calibration hunks.

    New optional contributions — the cluster_rare_threshold rule (era-13 Phase 10) and
    ShapePrimitive penalties (era-13 Phase 4) — fire symmetrically on calibration and fixture
    hunks. Their calibration-side firing inflates the threshold by the same magnitude they add
    to fixture scores, cancelling their recall contribution entirely. See
    docs/research/evidence/era13-final.md § cancellation for the empirical evidence.

    Suppressing them on the calibration path (apply_optional_contributions_to_cal=False,
    the default) is mathematically equivalent to "calibration never accumulates these signals,"
    not "calibration and fixture use different scorers." The base scorer and the era-11
    cluster_bonus are identical on both paths. Only the optional contributions are asymmetric.

    When apply_optional_contributions_to_cal=False (default), calibration scorers are built
    with cluster_rare_threshold=0 and shape_primitives=[], regardless of the values passed to
    this function. The fixture/scoring path (outside this function) uses the full parameters.
    Set apply_optional_contributions_to_cal=True only when intentionally reproducing the
    symmetric (era-13 status quo) calibration behaviour for comparison.

    """
    thresholds: list[float] = []

    # Resolve shape-primitive names → factories once so each per-seed
    # scorer build gets fresh instances (primitives may carry per-cluster
    # baseline state that's specific to its scorer).
    def _make_primitives() -> list[ShapePrimitive[Any]]:
        return build_shape_primitives(list(call_receiver_shape_primitive_names))

    # When n_clusters > 1, use the metadata-aware sampler so that
    # cluster_bonus contributions can be folded into calibration scores.
    # When n_clusters == 1 the simpler hunk-only path is used.
    use_metadata = call_receiver_n_clusters > 1

    def _sample(seed: int) -> tuple[list[str] | None, list[tuple[str, Path, str]] | None]:
        if use_metadata:
            meta = sample_hunks_with_metadata(repo_dir, n_cal, seed, adapter=adapter)
            return [h for h, _, _ in meta], meta
        return sample_hunks(repo_dir, n_cal, seed, adapter=adapter), None

    _probe_tokenizer = None

    # Cal-side optional contributions: suppressed when flag=False (the default)
    # so that the threshold reflects only the base scorer + era-11 cluster_bonus.
    # See G7 contract in this function's docstring.
    cal_rare_threshold = (
        call_receiver_cluster_rare_threshold if apply_optional_contributions_to_cal else 0
    )

    def _cal_primitives() -> list[ShapePrimitive[Any]]:
        return _make_primitives() if apply_optional_contributions_to_cal else []

    # Asymmetric-cal mode is active when the flag is False and the caller
    # passed a non-zero rare threshold or non-empty primitive list — the
    # [rare-counter] line captures this for bench observability.
    _asym_cal_active = not apply_optional_contributions_to_cal

    # Build first scorer — lets the tokenizer load once
    first_hunks, first_meta = _sample(base_seed)
    first_scorer = SequentialImportBpeScorer(
        repo_corpus_files=repo_corpus_files,
        bpe_generic_baseline_path=bpe_generic_baseline_path,
        calibration_hunks=first_hunks,
        calibration_hunks_with_metadata=first_meta,
        adapter=adapter,
        threshold_percentile=threshold_percentile,
        threshold_iqr_k=threshold_iqr_k,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        call_receiver_root_bonus=call_receiver_root_bonus,
        call_receiver_n_clusters=call_receiver_n_clusters,
        call_receiver_cluster_seed=call_receiver_cluster_seed,
        call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        call_receiver_cluster_rare_threshold=cal_rare_threshold,
        call_receiver_cluster_size_min=call_receiver_cluster_size_min,
        call_receiver_shape_primitives=_cal_primitives(),
        enable_typicality_filter=enable_typicality_filter,
        **({"_tokenizer": _probe_tokenizer} if _probe_tokenizer is not None else {}),
    )
    shared_tokenizer = first_scorer._tokenizer
    thresholds.append(first_scorer.bpe_threshold)
    if call_receiver_cluster_rare_threshold > 0:
        print(
            f"[rare-counter] cal seed={base_seed}: "
            f"rare_branch_fire_count={first_scorer.rare_branch_fire_count} "
            f"threshold={first_scorer.bpe_threshold:.4f} "
            f"asym_cal={_asym_cal_active}",
            file=sys.stderr,
        )

    # Build remaining scorers reusing the shared tokenizer
    for k in range(1, n_seeds):
        seed = base_seed + k
        hunks, meta = _sample(seed)
        scorer = SequentialImportBpeScorer(
            repo_corpus_files=repo_corpus_files,
            bpe_generic_baseline_path=bpe_generic_baseline_path,
            calibration_hunks=hunks,
            calibration_hunks_with_metadata=meta,
            adapter=adapter,
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=cal_rare_threshold,
            call_receiver_cluster_size_min=call_receiver_cluster_size_min,
            call_receiver_shape_primitives=_cal_primitives(),
            enable_typicality_filter=enable_typicality_filter,
            _tokenizer=shared_tokenizer,
        )
        thresholds.append(scorer.bpe_threshold)
        if call_receiver_cluster_rare_threshold > 0:
            print(
                f"[rare-counter] cal seed={seed}: "
                f"rare_branch_fire_count={scorer.rare_branch_fire_count} "
                f"threshold={scorer.bpe_threshold:.4f} "
                f"asym_cal={_asym_cal_active}",
                file=sys.stderr,
            )

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
        default=100.0,
        help=(
            "Percentile of calibration scores to use as BPE threshold. "
            "Default 100.0 (max). Pass 95.0 for p95 outlier-robust mode."
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
        default=7,
        help=(
            "Number of independent calibration seeds for multi-seed median threshold. "
            "K independent calibrations are run (seeds: seed, seed+1, ..., seed+K-1); "
            "the median threshold is used. Default 7."
        ),
    )
    parser.add_argument(
        "--repo-corpus",
        default=".argot/repo-corpus.txt",
        help="File listing repo corpus source paths (produced by `argot fit`)",
    )
    parser.add_argument(
        "--generic-baseline",
        default=".argot/generic-baseline.json",
        help="Path to BPE generic baseline JSON (produced by `argot fit`)",
    )
    parser.add_argument(
        "--output",
        default=".argot/scorer-config.json",
        help="Output path for scorer-config.json",
    )
    parser.add_argument(
        "--evidence-top-n",
        type=int,
        default=_DEFAULT_EVIDENCE_TOP_N,
        help=(
            "Number of top entries per dimension to bake into the "
            "evidence_corpus block of scorer-config.json (default "
            f"{_DEFAULT_EVIDENCE_TOP_N})."
        ),
    )
    args = parser.parse_args()

    repo_path = Path(args.repo).resolve()
    repo_corpus_path = Path(args.repo_corpus)
    generic_baseline_path = Path(args.generic_baseline)

    if not repo_corpus_path.exists():
        print(
            f"error: repo corpus file not found at {repo_corpus_path} — run `argot fit` first",
            file=sys.stderr,
        )
        sys.exit(2)
    if not generic_baseline_path.exists():
        print(
            f"error: generic baseline not found at {generic_baseline_path} — run `argot fit` first",
            file=sys.stderr,
        )
        sys.exit(2)

    repo_corpus_files = [
        Path(line) for line in repo_corpus_path.read_text().splitlines() if line.strip()
    ]
    if not repo_corpus_files:
        print(f"error: {repo_corpus_path} is empty", file=sys.stderr)
        sys.exit(2)

    n_cal = args.n_cal
    adapter = adapter_for_files([str(p) for p in repo_corpus_files])
    source_dir = repo_path
    candidates = collect_candidates(source_dir, adapter=adapter)
    effective_n_cal = min(n_cal, len(candidates))

    call_receiver_alpha: float = 2.0
    call_receiver_cap: int = 5
    call_receiver_root_bonus: float = 2.0
    call_receiver_n_clusters: int = 8
    call_receiver_cluster_seed: int = 0
    call_receiver_cluster_bonus: float = 5.0

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
            repo_corpus_files=repo_corpus_files,
            adapter=adapter,
            bpe_generic_baseline_path=generic_baseline_path,
            threshold_percentile=args.threshold_percentile,
            threshold_iqr_k=args.threshold_iqr_k,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        )
        scorer = SequentialImportBpeScorer(
            repo_corpus_files=repo_corpus_files,
            bpe_generic_baseline_path=generic_baseline_path,
            bpe_threshold=threshold,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        )
        n_cal_used = effective_n_cal
    else:
        cal_hunks = sample_hunks(source_dir, effective_n_cal, args.seed, adapter=adapter)
        print(f"Sampled {len(cal_hunks)} calibration hunks from {source_dir}")
        scorer = SequentialImportBpeScorer(
            repo_corpus_files=repo_corpus_files,
            bpe_generic_baseline_path=generic_baseline_path,
            calibration_hunks=cal_hunks,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            threshold_percentile=args.threshold_percentile,
            threshold_iqr_k=args.threshold_iqr_k,
        )
        n_cal_used = scorer.n_calibration

    try:
        repo = pygit2.Repository(str(repo_path))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    # Pre-compute the per-dimension top-N samples that the evidence layer
    # uses at check time. Persisted alongside the threshold so check doesn't
    # have to retokenize the whole repo on every run.
    evidence_corpus = build_evidence_corpus(scorer, repo_corpus_files, top_n=args.evidence_top_n)

    config: dict[str, object] = {
        "version": _CONFIG_VERSION,
        "threshold": scorer.bpe_threshold,
        "call_receiver_alpha": call_receiver_alpha,
        "call_receiver_cap": call_receiver_cap,
        "call_receiver_root_bonus": call_receiver_root_bonus,
        "call_receiver_n_clusters": call_receiver_n_clusters,
        "call_receiver_cluster_seed": call_receiver_cluster_seed,
        "call_receiver_cluster_bonus": call_receiver_cluster_bonus,
        "calibration": {
            "n_cal": n_cal_used,
            "seed": args.seed,
            "n_seeds": args.threshold_n_seeds,
            "repo_sha": repo_sha,
            "timestamp_utc": datetime.now(tz=UTC).isoformat(),
        },
        "evidence_corpus": evidence_corpus.to_json_dict(),
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(config, indent=2))
    print(f"threshold: {scorer.bpe_threshold:.4f} → {out_path}")


if __name__ == "__main__":
    main()
