"""Adapter wrapping argot's production SequentialImportBpeScorer.

Gives the benchmark harness a single stable surface. When the upstream
scorer's constructor or return type changes, only this file changes.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Literal

import numpy as np
from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.calibration.random_hunk_sampler import collect_candidates, sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

if TYPE_CHECKING:
    from argot_bench.typicality import TypicalityModel

Language = Literal["python", "typescript"]
Reason = Literal["import", "bpe", "none", "auto_generated"]

# engine/argot/scoring/bpe/generic_tokens_bpe.json
# score.py → argot_bench → src → benchmarks → <repo root>
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


class BenchScorer:
    """Thin wrapper so the harness has one stable adapter surface.

    When SequentialImportBpeScorer's constructor or return type changes,
    only this file changes.
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
    # Lazy-import TypeScript adapter so Python-only runs don't pay its import cost.
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
    typicality_model: TypicalityModel | None = None,
) -> BenchScorer:
    """Build a BenchScorer calibrated on n_cal sampled hunks from repo_dir.

    Args:
        repo_dir: Root of the repo whose voice the scorer should learn.
        n_cal: Number of calibration hunks to sample.
        seed: numpy RNG seed for deterministic sampling.
        language: Source language of the target repo.
        bpe_model_b: Optional override for the generic BPE reference model path.
        typicality_model: Optional model to filter atypical hunks from the
            calibration pool. When provided, only hunks not flagged as atypical
            are eligible for sampling.

    Raises:
        ValueError: if repo_dir has no source files for the language, or
            if fewer than n_cal qualifying hunks are available for sampling.
    """
    adapter = _resolve_adapter(language)
    files = _source_files(repo_dir, adapter)
    if not files:
        raise ValueError(f"No {language} source files found in {repo_dir}")

    if typicality_model is None:
        cal_hunks = sample_hunks(repo_dir, n_cal, seed, adapter=adapter)
    else:
        pool = collect_candidates(repo_dir, adapter=adapter)
        filtered = [h for h in pool if not typicality_model.is_atypical(h)[0]]
        if len(filtered) < n_cal:
            raise ValueError(
                f"After typicality filter, only {len(filtered)} qualifying hunks "
                f"remain in {repo_dir!r}; cannot sample n_cal={n_cal}."
            )
        rng = np.random.default_rng(seed)
        indices = rng.choice(len(filtered), size=n_cal, replace=False)
        cal_hunks = [filtered[int(i)] for i in sorted(indices)]

    inner = SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=bpe_model_b or _BPE_MODEL_B,
        calibration_hunks=cal_hunks,
        adapter=adapter,
        repo_root=repo_dir,
    )
    return BenchScorer(inner)
