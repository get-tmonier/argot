from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Literal, cast

if TYPE_CHECKING:
    from argot_bench.typicality import TypicalityModel

from argot_bench.clone import ensure_clone, ensure_sha_checked_out
from argot_bench.extract import ensure_extracted
from argot_bench.fixtures import Fixture, load_catalog, read_hunk
from argot_bench.metrics import (
    auc_catalog,
    calibration_stability,
    fp_rate,
    recall_by_category,
    stage_attribution,
    threshold_cv,
)
from argot_bench.report import CorpusReport
from argot_bench.score import BenchScorer, build_scorer

Language = Literal["python", "typescript"]


@dataclass(frozen=True)
class RunConfig:
    corpus: str
    url: str
    language: Language
    prs: list[tuple[int, str]]  # (pr_num, sha)
    catalog_dir: Path
    data_dir: Path
    n_cal: int = 100
    seeds: list[int] = field(default_factory=lambda: [0, 1, 2, 3, 4])
    quick: bool = False
    fresh: bool = False
    typicality_filter: bool = False
    sample_controls: int | None = None


def _read_hunk_pair(catalog_dir: Path, fixture: Fixture) -> tuple[str, str]:
    return read_hunk(catalog_dir, fixture)


def _real_pr_hunks(
    dataset_path: Path,
    *,
    max_hunks: int | None = None,
) -> list[dict[str, object]]:
    hunks: list[dict[str, object]] = []
    with dataset_path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            hunks.append(rec)
            if max_hunks is not None and len(hunks) >= max_hunks:
                break
    return hunks


def _subsample_hunks(
    hunks: list[dict[str, object]], n: int, seed: int
) -> list[dict[str, object]]:
    """Return n hunks chosen in a reproducible random order (seed-stable)."""
    import numpy as np

    rng = np.random.default_rng(seed)
    indices = rng.permutation(len(hunks))[:n]
    return [hunks[int(i)] for i in indices]


def _score_fixtures(
    scorer: BenchScorer,
    catalog_dir: Path,
    fixtures: list[Fixture],
) -> list[dict[str, object]]:
    out: list[dict[str, object]] = []
    for fx in fixtures:
        src, hunk = _read_hunk_pair(catalog_dir, fx)
        r = scorer.score_hunk(
            hunk,
            file_source=src,
            hunk_start_line=fx.hunk_start_line,
            hunk_end_line=fx.hunk_end_line,
        )
        out.append(
            {
                "id": fx.id,
                "category": fx.category,
                "file": fx.file,
                "hunk_start_line": fx.hunk_start_line,
                "hunk_end_line": fx.hunk_end_line,
                "rationale": fx.rationale,
                "import_score": r.import_score,
                "bpe_score": r.bpe_score,
                "flagged": r.flagged,
                "reason": r.reason,
            }
        )
    return out


def _score_real_hunks(
    scorer: BenchScorer,
    hunks: list[dict[str, object]],
    repo_dir: Path,
    *,
    typicality_model: TypicalityModel | None = None,
    filter_stats: dict[str, int] | None = None,
) -> list[dict[str, object]]:
    """Score real-PR hunks from an argot-extract dataset record.

    Extract records carry `file_path` + 0-indexed half-open `[hunk_start_line,
    hunk_end_line)` bounds. The scorer expects the full file source and
    1-indexed inclusive line bounds, so we read the file from the checked-out
    repo and convert the indexing here.
    """
    out: list[dict[str, object]] = []
    for h in hunks:
        file_path_rel = h.get("file_path")
        hs = h.get("hunk_start_line")
        he = h.get("hunk_end_line")
        if not (isinstance(file_path_rel, str) and isinstance(hs, int) and isinstance(he, int)):
            continue
        file_abs = repo_dir / file_path_rel
        try:
            file_source = file_abs.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        lines = file_source.splitlines()
        hunk_content = "\n".join(lines[hs:he])
        if typicality_model is not None:
            is_atypical, distance, features = typicality_model.is_atypical(hunk_content)
            if is_atypical:
                if filter_stats is not None:
                    filter_stats["controls_filtered"] = (
                        filter_stats.get("controls_filtered", 0) + 1
                    )
                out.append(
                    {
                        "file_path": file_path_rel,
                        "hunk_start_line": hs,
                        "hunk_end_line": he,
                        "bpe_score": 0.0,
                        "import_score": 0.0,
                        "flagged": False,
                        "reason": "atypical",
                        "typicality_distance": distance,
                        "typicality_features": list(features),
                    }
                )
                continue
        r = scorer.score_hunk(
            hunk_content,
            file_source=file_source,
            hunk_start_line=hs + 1,
            hunk_end_line=he,
        )
        out.append(
            {
                "file_path": file_path_rel,
                "hunk_start_line": hs,
                "hunk_end_line": he,
                "bpe_score": r.bpe_score,
                "import_score": r.import_score,
                "flagged": r.flagged,
                "reason": r.reason,
            }
        )
    return out


def run_corpus(cfg: RunConfig) -> CorpusReport:
    catalog = load_catalog(cfg.catalog_dir)
    break_fixtures = catalog.fixtures

    # Quick mode: 1 PR, 1 fixture per category
    if cfg.quick and cfg.prs:
        cfg = RunConfig(
            corpus=cfg.corpus,
            url=cfg.url,
            language=cfg.language,
            prs=[cfg.prs[0]],
            catalog_dir=cfg.catalog_dir,
            data_dir=cfg.data_dir,
            n_cal=min(cfg.n_cal, 20),
            seeds=[cfg.seeds[0]],
            quick=True,
            fresh=cfg.fresh,
            typicality_filter=cfg.typicality_filter,
            sample_controls=cfg.sample_controls,
        )
        by_cat: dict[str, Fixture] = {}
        for fx in break_fixtures:
            by_cat.setdefault(fx.category, fx)
        break_fixtures = list(by_cat.values())

    repo = ensure_clone(cfg.data_dir, cfg.corpus, cfg.url)

    typicality_model: TypicalityModel | None = None
    filter_stats: dict[str, int] = {"pool_size": 0, "pool_filtered": 0, "controls_filtered": 0}
    if cfg.typicality_filter:
        from argot.scoring.adapters.language_adapter import LanguageAdapter
        from argot.scoring.adapters.python_adapter import PythonAdapter
        from argot.scoring.calibration.random_hunk_sampler import collect_candidates

        from argot_bench.typicality import TypicalityModel

        adapter_for_fit: LanguageAdapter
        if cfg.language == "python":
            adapter_for_fit = PythonAdapter()
        else:
            from argot.scoring.adapters.typescript import TypeScriptAdapter

            adapter_for_fit = TypeScriptAdapter()
        pool = collect_candidates(repo, adapter=adapter_for_fit)
        typicality_model = TypicalityModel(language=cfg.language)
        typicality_model.fit(pool)
        filter_stats["pool_size"] = len(pool)
        filter_stats["pool_filtered"] = sum(
            1 for h in pool if typicality_model.is_atypical(h)[0]
        )

    fixture_results: list[dict[str, object]] = []
    real_pr_results: list[dict[str, object]] = []
    thresholds: list[float] = []
    cal_score_signatures: list[set[str]] = []

    primary_pr, primary_sha = cfg.prs[0]

    for seed in cfg.seeds:
        ensure_sha_checked_out(repo, primary_sha)
        dataset = ensure_extracted(
            repo,
            cfg.data_dir / cfg.corpus / primary_sha / "dataset.jsonl",
        )
        scorer = build_scorer(
            repo,
            n_cal=cfg.n_cal,
            seed=seed,
            language=cfg.language,
            typicality_model=typicality_model,
        )
        thresholds.append(scorer.threshold)
        cal_score_signatures.append({f"{i}:{s:.4f}" for i, s in enumerate(scorer.cal_scores)})

        if seed == cfg.seeds[0]:
            fixture_results = _score_fixtures(scorer, cfg.catalog_dir, break_fixtures)
            hunks = _real_pr_hunks(dataset, max_hunks=None if not cfg.quick else 50)
            if cfg.sample_controls is not None and len(hunks) > cfg.sample_controls:
                hunks = _subsample_hunks(hunks, cfg.sample_controls, seed)
            real_pr_results = _score_real_hunks(
                scorer, hunks, repo, typicality_model=typicality_model, filter_stats=filter_stats
            )

    # For each injection-host PR beyond the primary, score real hunks (not in quick)
    if not cfg.quick:
        for pr_num, sha in cfg.prs[1:]:
            ensure_sha_checked_out(repo, sha)
            dataset = ensure_extracted(
                repo,
                cfg.data_dir / cfg.corpus / sha / "dataset.jsonl",
            )
            scorer2 = build_scorer(
                repo,
                n_cal=cfg.n_cal,
                seed=cfg.seeds[0],
                language=cfg.language,
                typicality_model=typicality_model,
            )
            hunks = _real_pr_hunks(dataset)
            if cfg.sample_controls is not None and len(hunks) > cfg.sample_controls:
                hunks = _subsample_hunks(hunks, cfg.sample_controls, cfg.seeds[0])
            real_pr_results.extend(
                _score_real_hunks(
                    scorer2,
                    hunks,
                    repo,
                    typicality_model=typicality_model,
                    filter_stats=filter_stats,
                )
            )

    break_scores = [cast(float, r["bpe_score"]) for r in fixture_results]
    ctrl_scores = [
        cast(float, r["bpe_score"]) for r in real_pr_results if r.get("reason") != "atypical"
    ]

    threshold_mean = sum(thresholds) / len(thresholds) if thresholds else 0.0

    metrics = {
        "auc_catalog": auc_catalog(break_scores, ctrl_scores),
        "recall_by_category": recall_by_category(fixture_results),
        "fp_rate_real_pr": fp_rate([r for r in real_pr_results if r.get("reason") != "atypical"]),
        "threshold_cv": threshold_cv(thresholds),
        "threshold_mean": threshold_mean,
        "thresholds": thresholds,
        "calibration_stability": calibration_stability(cal_score_signatures, thresholds),
        "stage_attribution": stage_attribution(fixture_results),
        "n_fixtures": len(fixture_results),
        "n_real_pr_hunks": len(real_pr_results),
        "typicality_filter": cfg.typicality_filter,
        "typicality_stats": filter_stats,
        "sample_controls": cfg.sample_controls,
    }

    return CorpusReport(
        corpus=cfg.corpus,
        language=cfg.language,
        metrics=metrics,
        raw_scores=fixture_results + [dict(r, source="real_pr") for r in real_pr_results],
    )
