from __future__ import annotations

import json
import re
from collections.abc import Iterable, Iterator
from dataclasses import dataclass, field
from itertools import islice
from pathlib import Path
from typing import Literal, cast

from argot_bench.clone import ensure_clone, ensure_sha_checked_out
from argot_bench.extract import ensure_extracted
from argot_bench.fixtures import Fixture, load_catalog, read_hunk
from argot_bench.metrics import (
    auc_catalog,
    calibration_stability,
    fp_rate,
    recall_by_category,
    recall_by_difficulty,
    stage_attribution,
    threshold_cv,
)
from argot_bench.report import CorpusReport
from argot_bench.score import BenchScorer, build_scorer

Language = Literal["python", "typescript"]


_BREAK_META_RE = re.compile(r"^\s*(//|#)\s*Break\s*:")


def _strip_break_meta(
    catalog_content: str, chs: int, che: int
) -> tuple[str, int, int]:
    """Strip ``// Break: ...`` / ``# Break: ...`` lines from catalog content.

    Catalog break files conventionally include a one-line meta-comment
    naming the anomaly (e.g. ``// Break: provider throws mid-generation``).
    That comment is fixture-design metadata, NOT content the production
    scorer should ever see — its presence biases prose-blanking under the
    host-injection path and is the dominant surprise signal under MLM
    scoring (per the ML investigation that uncovered the routing bug).

    Returns ``(stripped_content, new_hunk_start_line, new_hunk_end_line)``.
    The returned line range covers the same code lines, shifted to account
    for the stripped meta-comment(s).
    """
    lines = catalog_content.splitlines()
    new_idx = 0
    old_to_new: list[int] = []  # 1-indexed; 0 means "dropped"
    kept: list[str] = []
    for ln in lines:
        if _BREAK_META_RE.match(ln):
            old_to_new.append(0)
        else:
            new_idx += 1
            old_to_new.append(new_idx)
            kept.append(ln)
    if chs < 1 or che > len(old_to_new):
        return catalog_content, chs, che
    new_chs = next(
        (old_to_new[k - 1] for k in range(chs, che + 1) if old_to_new[k - 1] != 0),
        None,
    )
    new_che = next(
        (old_to_new[k - 1] for k in range(che, chs - 1, -1) if old_to_new[k - 1] != 0),
        None,
    )
    if new_chs is None or new_che is None or new_chs > new_che:
        return catalog_content, chs, che
    new_content = "\n".join(kept) + (
        "\n" if catalog_content.endswith("\n") else ""
    )
    return new_content, new_chs, new_che


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
    typicality_filter: bool = True
    sample_controls: int | None = None
    call_receiver_alpha: float = 2.0
    call_receiver_cap: int = 5
    call_receiver_root_bonus: float = 2.0
    call_receiver_n_clusters: int = 8
    call_receiver_cluster_seed: int = 0
    call_receiver_cluster_bonus: float = 5.0
    call_receiver_cluster_rare_threshold: int = 0
    threshold_percentile: float | None = None
    threshold_iqr_k: float | None = None
    threshold_n_seeds: int = 7


def _read_hunk_pair(catalog_dir: Path, fixture: Fixture) -> tuple[str, str]:
    return read_hunk(catalog_dir, fixture)


def _real_pr_hunks(dataset_path: Path) -> Iterator[dict[str, object]]:
    with dataset_path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            yield json.loads(line)


def _reservoir_sample(
    hunks: Iterable[dict[str, object]],
    n: int,
    seed: int,
) -> list[dict[str, object]]:
    """Algorithm R reservoir sampling — O(n) time, O(k) space."""
    import numpy as np

    rng = np.random.default_rng(seed)
    reservoir: list[dict[str, object]] = []
    for i, h in enumerate(hunks):
        if i < n:
            reservoir.append(h)
        else:
            j = int(rng.integers(0, i + 1))
            if j < n:
                reservoir[j] = h
    return reservoir


def _score_fixtures(
    scorer: BenchScorer,
    catalog_dir: Path,
    fixtures: list[Fixture],
    repo_dir: Path | None = None,
) -> list[dict[str, object]]:
    """Score each catalog fixture against its host context if available.

    Catalog break files live under ``benchmarks/catalogs/<corpus>/breaks/``;
    they are tiny standalone files whose path does NOT exist in the corpus
    repo. Passing the catalog path as ``file_path`` to the scorer makes
    cluster lookup fall back to the Jaccard-nearest-cluster path, which
    systematically routes a `fetch`-only break to whichever cluster has
    `fetch` attested — exactly the cluster where ``cluster_bonus`` will NOT
    fire, defeating era-11's cluster-conditional rule.

    When the manifest provides ``host_file`` + ``host_inject_at_line`` (Phase
    5 host-injection metadata), splice the catalog hunk into the real host
    file and score against the synthesized content with the host file's
    actual path — which IS in ``file_to_cluster`` and resolves to the
    correct cluster. Falls back to the legacy catalog-path behaviour when
    host metadata is absent.
    """
    out: list[dict[str, object]] = []
    for fx in fixtures:
        src, hunk = _read_hunk_pair(catalog_dir, fx)
        file_path = (repo_dir / fx.file) if repo_dir is not None else None
        scored_src: str | None = src
        scored_hs: int | None = fx.hunk_start_line
        scored_he: int | None = fx.hunk_end_line
        if (
            repo_dir is not None
            and fx.host_file is not None
            and fx.host_inject_at_line is not None
        ):
            host_path = repo_dir / fx.host_file
            try:
                host_content = host_path.read_text(encoding="utf-8")
            except OSError:
                host_content = None
            if host_content is not None:

                catalog_path = catalog_dir / fx.file
                try:
                    catalog_content = catalog_path.read_text(encoding="utf-8")
                except OSError:
                    catalog_content = None
                if catalog_content is not None:
                    # Strip fixture-design meta-comments (`// Break: ...`)
                    # before splicing so they don't poison prose-blanking
                    # or surface as false "anomaly" signal.
                    cleaned, clean_hs, clean_he = _strip_break_meta(
                        catalog_content, fx.hunk_start_line, fx.hunk_end_line
                    )
                    # Re-extract hunk content from the cleaned catalog so
                    # the score input matches the catalog hunk after
                    # comment stripping.
                    cleaned_lines = cleaned.splitlines()
                    if 1 <= clean_hs <= clean_he <= len(cleaned_lines):
                        hunk = "\n".join(cleaned_lines[clean_hs - 1 : clean_he])
                    file_path = host_path
                    # Pass file_source=None and hunk line bounds=None.
                    # All we need from the routing fix is correct cluster
                    # routing via file_path = host_path (which IS in the
                    # scorer's file_to_cluster). We deliberately do NOT
                    # pass the synthesized post-injection text as
                    # file_source because:
                    #   (a) prose-blanking on the synthesized file produces
                    #       garbage results when tree-sitter sees an
                    #       out-of-place class mid-host-file (ERROR nodes);
                    #   (b) the typicality_filter on the synthesized file
                    #       triggers `atypical_file` short-circuits
                    #       (catalog imports look foreign vs the host's
                    #       distribution), incorrectly returning
                    #       `flagged=False reason="atypical_file"` for
                    #       legitimate breaks.
                    # With file_source=None, the scorer skips both the
                    # file-level typicality check and prose-blanking;
                    # the hunk-only typicality check still runs.
                    scored_src = None
                    scored_hs = None
                    scored_he = None
        r = scorer.score_hunk(
            hunk,
            file_source=scored_src,
            hunk_start_line=scored_hs,
            hunk_end_line=scored_he,
            file_path=file_path,
        )
        out.append(
            {
                "id": fx.id,
                "category": fx.category,
                "file": fx.file,
                "hunk_start_line": fx.hunk_start_line,
                "hunk_end_line": fx.hunk_end_line,
                "rationale": fx.rationale,
                "difficulty": fx.difficulty,
                "import_score": r.import_score,
                "bpe_score": r.bpe_score,
                "flagged": r.flagged,
                "reason": r.reason,
            }
        )
    return out


def _score_real_hunks(
    scorer: BenchScorer,
    hunks: Iterable[dict[str, object]],
    repo_dir: Path,
) -> list[dict[str, object]]:
    """Score real-PR hunks from an argot-extract dataset record.

    Extract records carry `file_path` + 0-indexed half-open `[hunk_start_line,
    hunk_end_line)` bounds. The scorer expects the full file source and
    1-indexed inclusive line bounds, so we read the file from the checked-out
    repo and convert the indexing here.

    Hunks whose file falls under an excluded directory (test/, docs/, etc.) are
    returned with reason='excluded_path' without invoking the scorer.
    Atypical hunks and data-dominant files produce reason='atypical' or
    'atypical_file' from the scorer itself.
    """
    from argot.scoring.calibration.random_hunk_sampler import DEFAULT_EXCLUDE_DIRS, is_excluded_path

    out: list[dict[str, object]] = []
    for h in hunks:
        file_path_rel = h.get("file_path")
        hs = h.get("hunk_start_line")
        he = h.get("hunk_end_line")
        if not (isinstance(file_path_rel, str) and isinstance(hs, int) and isinstance(he, int)):
            continue
        file_abs = repo_dir / file_path_rel
        if is_excluded_path(file_abs, repo_dir, DEFAULT_EXCLUDE_DIRS):
            out.append(
                {
                    "file_path": file_path_rel,
                    "hunk_start_line": hs,
                    "hunk_end_line": he,
                    "bpe_score": 0.0,
                    "import_score": 0.0,
                    "flagged": False,
                    "reason": "excluded_path",
                }
            )
            continue
        try:
            file_source = file_abs.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        lines = file_source.splitlines()
        hunk_content = "\n".join(lines[hs:he])
        r = scorer.score_hunk(
            hunk_content,
            file_source=file_source,
            hunk_start_line=hs + 1,
            hunk_end_line=he,
            file_path=file_abs,
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
            call_receiver_alpha=cfg.call_receiver_alpha,
            call_receiver_cap=cfg.call_receiver_cap,
            call_receiver_root_bonus=cfg.call_receiver_root_bonus,
            call_receiver_n_clusters=cfg.call_receiver_n_clusters,
            call_receiver_cluster_seed=cfg.call_receiver_cluster_seed,
            call_receiver_cluster_bonus=cfg.call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=cfg.call_receiver_cluster_rare_threshold,
            threshold_percentile=cfg.threshold_percentile,
            threshold_iqr_k=cfg.threshold_iqr_k,
            threshold_n_seeds=cfg.threshold_n_seeds,
        )
        by_cat: dict[str, Fixture] = {}
        for fx in break_fixtures:
            by_cat.setdefault(fx.category, fx)
        break_fixtures = list(by_cat.values())

    repo = ensure_clone(cfg.data_dir, cfg.corpus, cfg.url)

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
            enable_typicality_filter=cfg.typicality_filter,
            call_receiver_alpha=cfg.call_receiver_alpha,
            call_receiver_cap=cfg.call_receiver_cap,
            call_receiver_root_bonus=cfg.call_receiver_root_bonus,
            call_receiver_n_clusters=cfg.call_receiver_n_clusters,
            call_receiver_cluster_seed=cfg.call_receiver_cluster_seed,
            call_receiver_cluster_bonus=cfg.call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=cfg.call_receiver_cluster_rare_threshold,
            threshold_percentile=cfg.threshold_percentile,
            threshold_iqr_k=cfg.threshold_iqr_k,
            threshold_n_seeds=cfg.threshold_n_seeds,
        )
        thresholds.append(scorer.threshold)
        cal_score_signatures.append({f"{i}:{s:.4f}" for i, s in enumerate(scorer.cal_scores)})

        if seed == cfg.seeds[0]:
            fixture_results = _score_fixtures(scorer, cfg.catalog_dir, break_fixtures, repo_dir=repo)
            if cfg.call_receiver_cluster_rare_threshold > 0:
                import sys

                print(
                    f"[rare-counter] {cfg.corpus} fixture path seed={seed}: "
                    f"rare_branch_fire_count={scorer.rare_branch_fire_count}",
                    file=sys.stderr,
                )
            hunks_stream: Iterable[dict[str, object]] = _real_pr_hunks(dataset)
            if cfg.quick:
                hunks_stream = islice(hunks_stream, 50)
            hunks_input: Iterable[dict[str, object]]
            if cfg.sample_controls is not None:
                hunks_input = _reservoir_sample(hunks_stream, cfg.sample_controls, seed)
            else:
                hunks_input = hunks_stream
            real_pr_results = _score_real_hunks(scorer, hunks_input, repo)

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
                enable_typicality_filter=cfg.typicality_filter,
                call_receiver_alpha=cfg.call_receiver_alpha,
                call_receiver_cap=cfg.call_receiver_cap,
                call_receiver_root_bonus=cfg.call_receiver_root_bonus,
                call_receiver_n_clusters=cfg.call_receiver_n_clusters,
                call_receiver_cluster_seed=cfg.call_receiver_cluster_seed,
                call_receiver_cluster_bonus=cfg.call_receiver_cluster_bonus,
            call_receiver_cluster_rare_threshold=cfg.call_receiver_cluster_rare_threshold,
                threshold_percentile=cfg.threshold_percentile,
                threshold_iqr_k=cfg.threshold_iqr_k,
                threshold_n_seeds=cfg.threshold_n_seeds,
            )
            hunks_stream2: Iterable[dict[str, object]] = _real_pr_hunks(dataset)
            hunks_input2: Iterable[dict[str, object]]
            if cfg.sample_controls is not None:
                hunks_input2 = _reservoir_sample(hunks_stream2, cfg.sample_controls, cfg.seeds[0])
            else:
                hunks_input2 = hunks_stream2
            real_pr_results.extend(_score_real_hunks(scorer2, hunks_input2, repo))

    _excluded_reasons = {"atypical", "atypical_file", "excluded_path", "auto_generated"}
    break_scores = [cast(float, r["bpe_score"]) for r in fixture_results]
    ctrl_scores = [
        cast(float, r["bpe_score"])
        for r in real_pr_results
        if r.get("reason") not in _excluded_reasons
    ]

    threshold_mean = sum(thresholds) / len(thresholds) if thresholds else 0.0

    metrics = {
        "auc_catalog": auc_catalog(break_scores, ctrl_scores),
        "recall_by_category": recall_by_category(fixture_results),
        "recall_by_difficulty": recall_by_difficulty(fixture_results),
        "fp_rate_real_pr": fp_rate(
            [r for r in real_pr_results if r.get("reason") not in _excluded_reasons]
        ),
        "threshold_cv": threshold_cv(thresholds),
        "threshold_mean": threshold_mean,
        "thresholds": thresholds,
        "calibration_stability": calibration_stability(cal_score_signatures, thresholds),
        "stage_attribution": stage_attribution(fixture_results),
        "n_fixtures": len(fixture_results),
        "n_real_pr_hunks": len(real_pr_results),
        "typicality_filter": cfg.typicality_filter,
        "sample_controls": cfg.sample_controls,
    }

    return CorpusReport(
        corpus=cfg.corpus,
        language=cfg.language,
        metrics=metrics,
        raw_scores=fixture_results + [dict(r, source="real_pr") for r in real_pr_results],
    )
