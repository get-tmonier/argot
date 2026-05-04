"""Adapter wrapping argot's production SequentialImportBpeScorer.

Gives the benchmark harness a single stable surface. When the upstream
scorer's constructor or return type changes, only this file changes.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

# Side-effect import: registers shape primitives so they are addressable
# from --enable-shape-primitives via their canonical names.
import argot.scoring.scorers.shape_primitive_registrations  # noqa: F401, E402
from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.calibration import calibrate_multi_seed
from argot.scoring.calibration.random_hunk_sampler import sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
from argot.scoring.scorers.shape_primitive_registry import build_shape_primitives

Language = Literal["python", "typescript"]
Reason = Literal[
    "import",
    "call_receiver",
    "bpe",
    "none",
    "auto_generated",
    "atypical",
    "atypical_file",
    "excluded_path",
]

# engine/argot/scoring/bpe/generic_tokens_bpe.json
# score.py -> argot_bench -> src -> benchmarks -> <repo root>
_BPE_GENERIC_BASELINE = (
    Path(__file__).resolve().parent.parent.parent.parent
    / "engine"
    / "argot"
    / "scoring"
    / "bpe"
    / "generic_tokens_bpe.json"
)


@dataclass(frozen=True)
class ScoreResult:
    """Structured result from scoring a single hunk."""

    import_score: float
    bpe_score: float
    flagged: bool
    reason: Reason
    call_receiver_unattested: tuple[str, ...] = ()


class BenchScorer:
    """Thin wrapper so the harness has one stable adapter surface.

    When SequentialImportBpeScorer's constructor or return type changes,
    only this file changes.  Stage 1.5 (call-receiver) is now handled
    natively by the inner scorer.
    """

    def __init__(self, inner: SequentialImportBpeScorer) -> None:
        self._inner = inner

    @property
    def threshold(self) -> float:
        return self._inner.bpe_threshold

    @property
    def cal_scores(self) -> list[float]:
        return list(self._inner.cal_scores)

    @property
    def rare_branch_fire_count(self) -> int:
        """Times the cluster-rare branch fired in weighted_contribution_for_file."""
        return self._inner.rare_branch_fire_count

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
        file_path: Path | None = None,
    ) -> ScoreResult:
        raw = self._inner.score_hunk(
            hunk_content,
            file_source=file_source,
            hunk_start_line=hunk_start_line,
            hunk_end_line=hunk_end_line,
            file_path=file_path,
        )
        return ScoreResult(
            import_score=float(raw.stages.import_score),
            bpe_score=float(raw.stages.bpe_score),
            flagged=bool(raw.flagged),
            reason=raw.reason,
        )


def _resolve_adapter(language: Language) -> LanguageAdapter:
    if language == "python":
        return PythonAdapter()
    from argot.scoring.adapters.typescript import TypeScriptAdapter

    return TypeScriptAdapter()


def _source_files(repo_dir: Path, adapter: LanguageAdapter) -> list[Path]:
    out: list[Path] = []
    for ext in sorted(adapter.file_extensions):
        out.extend(sorted(repo_dir.rglob(f"*{ext}")))
    return out


def _load_diff_hunks_for_probe(
    dataset_path: Path,
    repo_dir: Path,
    n: int,
    seed: int,
) -> list[tuple[str, Path, str]]:
    """Sample n diff hunks from extract's dataset.jsonl for the auto-detect probe.

    Returns (hunk_content, file_abs_path, file_source) tuples. Skips records
    with empty hunks, unreadable files, or out-of-range line bounds.
    Deterministic: same (dataset_path, n, seed) always returns same hunks.
    """
    import json as _json
    import random as _random

    records: list[dict[str, object]] = []
    with dataset_path.open() as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(_json.loads(line))
    rng = _random.Random(seed)
    rng.shuffle(records)
    out: list[tuple[str, Path, str]] = []
    seen: set[str] = set()
    for rec in records:
        if len(out) >= n:
            break
        fp = rec.get("file_path")
        hs = rec.get("hunk_start_line")
        he = rec.get("hunk_end_line")
        if not (isinstance(fp, str) and isinstance(hs, int) and isinstance(he, int)):
            continue
        key = f"{fp}:{hs}:{he}"
        if key in seen:
            continue
        seen.add(key)
        file_abs = repo_dir / fp
        try:
            file_source = file_abs.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        lines = file_source.splitlines()
        if hs < 0 or he > len(lines) or he <= hs:
            continue
        hunk_content = "\n".join(lines[hs:he])
        if not hunk_content.strip():
            continue
        out.append((hunk_content, file_abs, file_source))
    return out


def build_scorer(
    repo_dir: Path,
    *,
    n_cal: int,
    seed: int,
    language: Language,
    bpe_generic_baseline: Path | None = None,
    enable_typicality_filter: bool = True,
    call_receiver_alpha: float = 2.0,
    call_receiver_cap: int = 5,
    call_receiver_root_bonus: float = 2.0,
    call_receiver_n_clusters: int = 8,
    call_receiver_cluster_seed: int = 0,
    call_receiver_cluster_bonus: float = 5.0,
    call_receiver_cluster_rare_threshold: int = 0,
    call_receiver_cluster_size_min: int = 0,
    call_receiver_shape_primitive_names: tuple[str, ...] = (),
    threshold_percentile: float | None = None,
    threshold_iqr_k: float | None = None,
    threshold_n_seeds: int = 7,
    apply_optional_contributions_to_cal: bool = False,
    auto_select_asym_cal: bool = False,
    asym_fire_rate_threshold: float = 0.05,
    auto_detect_probe_dataset: Path | None = None,
) -> BenchScorer:
    """Build a BenchScorer calibrated on n_cal sampled hunks from repo_dir.

    Args:
        repo_dir: Root of the repo whose voice the scorer should learn.
        n_cal: Number of calibration hunks to sample.
        seed: numpy RNG seed for deterministic sampling.
        language: Source language of the target repo.
        bpe_generic_baseline: Optional override for the generic BPE reference model path.
        enable_typicality_filter: Pass True (default) to let the prod scorer filter
            atypical model-A files and calibration hunks internally.
        call_receiver_alpha: Soft-penalty weight. 0.0 disables Stage 1.5 entirely.
            Default 2.0 (era-9 shipping config).
        call_receiver_cap: Max unattested callees counted in the penalty (default 5).
        threshold_percentile: BPE threshold percentile. None = max(cal_scores) (era-10
            shipping config); 95.0 = p95 (robust to outliers).

    Raises:
        ValueError: if repo_dir has no source files, or insufficient qualifying hunks.
    """
    adapter = _resolve_adapter(language)
    files = _source_files(repo_dir, adapter)
    if not files:
        raise ValueError(f"No {language} source files found in {repo_dir}")

    # ---- Per-corpus auto-detect (era-13.5) ----
    # If requested, probe the calibration distribution to decide whether the
    # cluster_rare rule is informative on this corpus. Build one probe scorer
    # with the rule enabled and measure the fire-rate (rare_branch_fire_count
    # divided by n_cal). If fire-rate >= threshold, real-PR controls also fire
    # heavily on this corpus → enabling the rule would FP-flood. Override
    # cluster_rare_threshold to 0 so the rule is disabled for both
    # calibration AND scoring (= baseline behaviour, no regression).
    if (
        auto_select_asym_cal
        and call_receiver_cluster_rare_threshold > 0
        and call_receiver_n_clusters > 1
    ):
        # Probe with diff hunks (from extracted dataset) when available — they
        # match the real-PR control distribution. Fall back to random source
        # hunks when no dataset is provided (signal will be noisier; the rule
        # may not be picked even on asym-safe corpora like faker-js).
        if auto_detect_probe_dataset is not None and auto_detect_probe_dataset.exists():
            probe_meta = _load_diff_hunks_for_probe(
                auto_detect_probe_dataset, repo_dir, n_cal, seed,
            )
        else:
            from argot.scoring.calibration.random_hunk_sampler import (
                sample_hunks_with_metadata,
            )
            probe_meta = sample_hunks_with_metadata(repo_dir, n_cal, seed, adapter=adapter)
        probe = SequentialImportBpeScorer(
            repo_corpus_files=files,
            bpe_generic_baseline_path=bpe_generic_baseline or _BPE_GENERIC_BASELINE,
            calibration_hunks=[h for h, _, _ in probe_meta],
            calibration_hunks_with_metadata=probe_meta,
            adapter=adapter,
            enable_typicality_filter=enable_typicality_filter,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold,
            call_receiver_cluster_size_min=call_receiver_cluster_size_min,
            call_receiver_shape_primitives=build_shape_primitives(
                list(call_receiver_shape_primitive_names)
            ),
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
        )
        # Per-hunk fire rate (= fraction of cal hunks where the rare rule fires
        # at least once). Robust to "many fires per hunk vs few fires per hunk"
        # which is corpus-dependent (Zipf tail length matters).
        hunks_seen = max(probe.hunks_scored, 1)
        fire_rate = probe.rare_branch_hunks_fired / hunks_seen
        keep_rule = fire_rate < asym_fire_rate_threshold
        import sys as _sys
        print(
            f"[auto-asym] cluster_rare probe: "
            f"rare_hunks_fired={probe.rare_branch_hunks_fired}/{hunks_seen} "
            f"fire_rate={fire_rate:.3f} "
            f"threshold={asym_fire_rate_threshold:.3f} "
            f"→ {'KEEP rule (asym, +catches expected)' if keep_rule else 'DISABLE rule (rare=0, baseline)'}",
            file=_sys.stderr,
        )
        del probe
        if not keep_rule:
            # Disable cluster_rare rule entirely for this corpus.
            # apply_optional_contributions_to_cal stays at its default (False)
            # because with rare=0 and no primitives, asym vs sym is a no-op.
            call_receiver_cluster_rare_threshold = 0

    if threshold_n_seeds > 1:
        median_threshold = calibrate_multi_seed(
            base_seed=seed,
            n_seeds=threshold_n_seeds,
            n_cal=n_cal,
            repo_dir=repo_dir,
            repo_corpus_files=files,
            adapter=adapter,
            bpe_generic_baseline_path=bpe_generic_baseline or _BPE_GENERIC_BASELINE,
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold,
            call_receiver_cluster_size_min=call_receiver_cluster_size_min,
            call_receiver_shape_primitive_names=call_receiver_shape_primitive_names,
            enable_typicality_filter=enable_typicality_filter,
            apply_optional_contributions_to_cal=apply_optional_contributions_to_cal,
        )
        inner = SequentialImportBpeScorer(
            repo_corpus_files=files,
            bpe_generic_baseline_path=bpe_generic_baseline or _BPE_GENERIC_BASELINE,
            bpe_threshold=median_threshold,
            adapter=adapter,
            repo_root=repo_dir,
            enable_typicality_filter=enable_typicality_filter,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold,
            call_receiver_cluster_size_min=call_receiver_cluster_size_min,
            call_receiver_shape_primitives=build_shape_primitives(
                list(call_receiver_shape_primitive_names)
            ),
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
        )
    else:
        cal_hunks = sample_hunks(repo_dir, n_cal, seed, adapter=adapter)
        inner = SequentialImportBpeScorer(
            repo_corpus_files=files,
            bpe_generic_baseline_path=bpe_generic_baseline or _BPE_GENERIC_BASELINE,
            calibration_hunks=cal_hunks,
            adapter=adapter,
            repo_root=repo_dir,
            enable_typicality_filter=enable_typicality_filter,
            call_receiver_alpha=call_receiver_alpha,
            call_receiver_cap=call_receiver_cap,
            call_receiver_root_bonus=call_receiver_root_bonus,
            call_receiver_n_clusters=call_receiver_n_clusters,
            call_receiver_cluster_seed=call_receiver_cluster_seed,
            call_receiver_cluster_bonus=call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold,
            call_receiver_cluster_size_min=call_receiver_cluster_size_min,
            call_receiver_shape_primitives=build_shape_primitives(
                list(call_receiver_shape_primitive_names)
            ),
            threshold_percentile=threshold_percentile,
            threshold_iqr_k=threshold_iqr_k,
        )
    return BenchScorer(inner)
