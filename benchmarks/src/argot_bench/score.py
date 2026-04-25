"""Adapter wrapping argot's production SequentialImportBpeScorer.

Gives the benchmark harness a single stable surface. When the upstream
scorer's constructor or return type changes, only this file changes.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.calibration.random_hunk_sampler import sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

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
_BPE_MODEL_B = (
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

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
    ) -> ScoreResult:
        raw = self._inner.score_hunk(
            hunk_content,
            file_source=file_source,
            hunk_start_line=hunk_start_line,
            hunk_end_line=hunk_end_line,
        )
        return ScoreResult(
            import_score=float(raw["import_score"]),
            bpe_score=float(raw["bpe_score"]),
            flagged=bool(raw["flagged"]),
            reason=raw["reason"],
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


def build_scorer(
    repo_dir: Path,
    *,
    n_cal: int,
    seed: int,
    language: Language,
    bpe_model_b: Path | None = None,
    enable_typicality_filter: bool = True,
    call_receiver_alpha: float = 2.0,
    call_receiver_cap: int = 5,
    threshold_percentile: float | None = 95.0,
    threshold_iqr_k: float | None = None,
) -> BenchScorer:
    """Build a BenchScorer calibrated on n_cal sampled hunks from repo_dir.

    Args:
        repo_dir: Root of the repo whose voice the scorer should learn.
        n_cal: Number of calibration hunks to sample.
        seed: numpy RNG seed for deterministic sampling.
        language: Source language of the target repo.
        bpe_model_b: Optional override for the generic BPE reference model path.
        enable_typicality_filter: Pass True (default) to let the prod scorer filter
            atypical model-A files and calibration hunks internally.
        call_receiver_alpha: Soft-penalty weight. 0.0 disables Stage 1.5 entirely.
            Default 2.0 (era-9 shipping config).
        call_receiver_cap: Max unattested callees counted in the penalty (default 5).
        threshold_percentile: BPE threshold percentile. 95.0 = p95 (era-10 default,
            robust to outliers); None = max(cal_scores) (legacy).

    Raises:
        ValueError: if repo_dir has no source files, or insufficient qualifying hunks.
    """
    adapter = _resolve_adapter(language)
    files = _source_files(repo_dir, adapter)
    if not files:
        raise ValueError(f"No {language} source files found in {repo_dir}")

    cal_hunks = sample_hunks(repo_dir, n_cal, seed, adapter=adapter)

    inner = SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=bpe_model_b or _BPE_MODEL_B,
        calibration_hunks=cal_hunks,
        adapter=adapter,
        repo_root=repo_dir,
        enable_typicality_filter=enable_typicality_filter,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        threshold_percentile=threshold_percentile,
        threshold_iqr_k=threshold_iqr_k,
    )
    return BenchScorer(inner)
