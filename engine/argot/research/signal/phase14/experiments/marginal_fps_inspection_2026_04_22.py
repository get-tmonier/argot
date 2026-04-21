# engine/argot/research/signal/phase14/experiments/marginal_fps_inspection_2026_04_22.py
"""Phase 14 Diagnostic — Marginal FP root-cause inspection (2026-04-22).

Reproduces exp #2c cal/ctrl splits for FastAPI and rich (seed=2 FP cases)
and profiles the BPE calibration distribution to determine whether the threshold
is anchored by a dominant outlier (fragile) or a dense top (robust noise).

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/\\
        marginal_fps_inspection_2026_04_22.py
"""

from __future__ import annotations

import ast
import statistics
import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    _DEFAULT_EXCLUDE_DIRS,
    MIN_BODY_LINES,
    _is_excluded,
)
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent  # engine/argot
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent  # engine/argot/research
_PROJECT_ROOT = _ARGOT_PKG.parent.parent  # argot/
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_BPE_MODEL_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"
_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "marginal_fps_inspection_2026-04-22.md"
)

N_CAL = 100
N_CTRL = 20
N_SEEDS = 5
SEEDS = list(range(N_SEEDS))

# (domain, seed) pairs where FP occurred in exp #2c
FP_CASES: list[tuple[str, int, int]] = [
    ("fastapi", 2, 5),   # domain, seed, ctrl_index of FP hunk
    ("rich", 2, 7),
]


# ---------------------------------------------------------------------------
# Candidate collection with source identifiers
# ---------------------------------------------------------------------------


@dataclass
class HunkRecord:
    file_path: Path
    start_line: int
    end_line: int
    text: str


def collect_candidates_with_ids(source_dir: Path) -> list[HunkRecord]:
    """Like collect_candidates() but returns HunkRecord objects with provenance."""
    records: list[HunkRecord] = []

    for py_file in sorted(source_dir.rglob("*.py")):
        if _is_excluded(py_file, source_dir, _DEFAULT_EXCLUDE_DIRS):
            continue
        try:
            source = py_file.read_text(encoding="utf-8", errors="replace")
            tree = ast.parse(source)
        except SyntaxError:
            continue

        lines = source.splitlines()
        for node in ast.iter_child_nodes(tree):
            if not isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                continue
            start = node.lineno
            end = node.end_lineno
            if end is None:
                continue
            if (end - start) < MIN_BODY_LINES:
                continue
            text = "\n".join(lines[start - 1 : end])
            records.append(HunkRecord(py_file, start, end, text))

    return records


def sample_disjoint_with_ids(
    records: list[HunkRecord],
    n_cal: int,
    n_ctrl: int,
    seed: int,
) -> tuple[list[HunkRecord], list[HunkRecord]]:
    """Mirror of sample_hunks_disjoint() but preserves HunkRecord identity."""
    needed = n_cal + n_ctrl
    if len(records) < needed:
        raise ValueError(
            f"Only {len(records)} records, need {needed} (n_cal={n_cal} + n_ctrl={n_ctrl})"
        )
    rng = np.random.default_rng(seed)
    perm = rng.permutation(len(records))
    cal_recs = [records[int(i)] for i in perm[:n_cal]]
    ctrl_recs = [records[int(i)] for i in perm[n_cal : n_cal + n_ctrl]]
    return cal_recs, ctrl_recs


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(
        p for p in repo_dir.rglob("*.py")
        if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


def _ensure_repo(name: str, url: str) -> Path:
    repo_dir = _REPOS_DIR / name
    if not repo_dir.exists():
        print(f"  Cloning {url} → {repo_dir}", flush=True)
        subprocess.run(["git", "clone", "--depth=1", url, str(repo_dir)], check=True)
    else:
        print(f"  Using cached: {repo_dir}", flush=True)
    return repo_dir


# ---------------------------------------------------------------------------
# Percentile helper
# ---------------------------------------------------------------------------


def _pct(sorted_data: list[float], p: float) -> float:
    """Compute percentile p (0–100) of pre-sorted ascending list."""
    if not sorted_data:
        return 0.0
    n = len(sorted_data)
    idx = (p / 100.0) * (n - 1)
    lo = int(idx)
    hi = min(lo + 1, n - 1)
    frac = idx - lo
    return sorted_data[lo] + frac * (sorted_data[hi] - sorted_data[lo])


# ---------------------------------------------------------------------------
# Per-seed analysis
# ---------------------------------------------------------------------------


@dataclass
class SeedAnalysis:
    seed: int
    threshold: float
    second_max: float
    gap_max_p99: float
    gap_max_second: float
    p99: float
    p95: float
    p90: float
    p75: float
    mean: float
    std: float
    top10: list[tuple[float, HunkRecord]]  # (bpe_score, record)
    cal_max_record: HunkRecord
    cal_max_score: float
    # Only populated for FP seeds
    fp_ctrl_record: HunkRecord | None
    fp_ctrl_score: float | None
    fp_ctrl_index: int | None
    holdout_threshold: float | None
    fp_fires_after_holdout: bool | None


def analyse_seed(
    scorer: SequentialImportBpeScorer,
    cal_recs: list[HunkRecord],
    ctrl_recs: list[HunkRecord],
    seed: int,
    fp_ctrl_index: int | None,
) -> SeedAnalysis:
    # Score all calibration hunks
    cal_scored: list[tuple[float, HunkRecord]] = []
    for rec in cal_recs:
        score = scorer._bpe_score(rec.text)  # noqa: SLF001
        cal_scored.append((score, rec))

    cal_scores_only = [s for s, _ in cal_scored]
    sorted_scores = sorted(cal_scores_only)
    n = len(sorted_scores)

    max_score = max(cal_scores_only)
    sorted_desc = sorted(cal_scored, key=lambda x: x[0], reverse=True)
    second_max = sorted_desc[1][0] if n >= 2 else max_score
    cal_max_score, cal_max_record = sorted_desc[0]

    p99 = _pct(sorted_scores, 99)
    p95 = _pct(sorted_scores, 95)
    p90 = _pct(sorted_scores, 90)
    p75 = _pct(sorted_scores, 75)
    mean = statistics.mean(cal_scores_only)
    std = statistics.pstdev(cal_scores_only)

    gap_max_p99 = max_score - p99
    gap_max_second = max_score - second_max

    top10 = sorted_desc[:10]

    fp_ctrl_record: HunkRecord | None = None
    fp_ctrl_score: float | None = None
    holdout_threshold: float | None = None
    fp_fires: bool | None = None

    if fp_ctrl_index is not None:
        fp_rec = ctrl_recs[fp_ctrl_index]
        fp_score = scorer._bpe_score(fp_rec.text)  # noqa: SLF001
        fp_ctrl_record = fp_rec
        fp_ctrl_score = fp_score

        # Holdout: remove cal-max, new threshold = max of remaining
        remaining_scores = [s for s, r in cal_scored if r is not cal_max_record]
        holdout_threshold = max(remaining_scores) if remaining_scores else 0.0
        fp_fires = fp_score > holdout_threshold

    return SeedAnalysis(
        seed=seed,
        threshold=scorer.bpe_threshold,
        second_max=second_max,
        gap_max_p99=gap_max_p99,
        gap_max_second=gap_max_second,
        p99=p99,
        p95=p95,
        p90=p90,
        p75=p75,
        mean=mean,
        std=std,
        top10=top10,
        cal_max_record=cal_max_record,
        cal_max_score=cal_max_score,
        fp_ctrl_record=fp_ctrl_record,
        fp_ctrl_score=fp_ctrl_score,
        fp_ctrl_index=fp_ctrl_index,
        holdout_threshold=holdout_threshold,
        fp_fires_after_holdout=fp_fires,
    )


def run_domain(
    domain: str,
    repo_dir: Path,
    fp_ctrl_by_seed: dict[int, int],
) -> list[SeedAnalysis]:
    model_a_files = _collect_source_files(repo_dir)
    all_records = collect_candidates_with_ids(repo_dir)
    print(
        f"  {domain}: {len(model_a_files)} model_a files, {len(all_records)} candidates",
        flush=True,
    )

    analyses: list[SeedAnalysis] = []
    for seed in SEEDS:
        cal_recs, ctrl_recs = sample_disjoint_with_ids(all_records, N_CAL, N_CTRL, seed)
        scorer = SequentialImportBpeScorer(
            model_a_files=model_a_files,
            bpe_model_b_path=_BPE_MODEL_PATH,
            calibration_hunks=[r.text for r in cal_recs],
        )
        fp_idx = fp_ctrl_by_seed.get(seed)
        analysis = analyse_seed(scorer, cal_recs, ctrl_recs, seed, fp_idx)
        analyses.append(analysis)
        print(
            f"  seed={seed}: threshold={analysis.threshold:.4f} "
            f"gap_max_second={analysis.gap_max_second:.4f} "
            f"gap_max_p99={analysis.gap_max_p99:.4f}",
            flush=True,
        )

    return analyses


# ---------------------------------------------------------------------------
# Report helpers
# ---------------------------------------------------------------------------


def _rel(path: Path) -> str:
    try:
        return str(path.relative_to(_PROJECT_ROOT))
    except ValueError:
        return str(path)


def _hunk_preview(text: str, max_lines: int = 30) -> str:
    lines = text.splitlines()
    if len(lines) <= max_lines:
        return text
    half = max_lines // 2
    omitted = len(lines) - max_lines
    mid = f"\n\n... [{omitted} lines omitted] ...\n\n"
    return "\n".join(lines[:half]) + mid + "\n".join(lines[-half:])


def _write_report(
    fastapi_analyses: list[SeedAnalysis],
    rich_analyses: list[SeedAnalysis],
) -> None:
    lines: list[str] = [
        "# Phase 14 Diagnostic — Marginal FP Root-Cause Inspection (2026-04-22)",
        "",
        "**Purpose:** Determine whether the two marginal FPs from exp #2c are caused by a dominant"
        " calibration outlier (fragile threshold) or a dense top of distribution (robust noise).",
        "",
        "**Cases inspected:**",
        "- FastAPI seed=2: ctrl_index=5, bpe=4.0668, threshold=4.0185, margin=+0.048",
        "- rich seed=2: ctrl_index=7, bpe=4.8159, threshold=4.7608, margin=+0.055",
        "",
        "---",
        "",
        "## §1 — Calibration Distribution Profile",
        "",
    ]

    for domain, analyses in [("FastAPI", fastapi_analyses), ("Rich", rich_analyses)]:
        lines += [
            f"### {domain}",
            "",
            "| seed | max | 2nd_max | gap(max-2nd) | gap(max-p99)"
            " | p99 | p95 | p90 | p75 | mean | std |",
            "|---|---|---|---|---|---|---|---|---|---|---|",
        ]
        for a in analyses:
            lines.append(
                f"| {a.seed}"
                f" | {a.threshold:.4f}"
                f" | {a.second_max:.4f}"
                f" | {a.gap_max_second:.4f}"
                f" | {a.gap_max_p99:.4f}"
                f" | {a.p99:.4f}"
                f" | {a.p95:.4f}"
                f" | {a.p90:.4f}"
                f" | {a.p75:.4f}"
                f" | {a.mean:.4f}"
                f" | {a.std:.4f} |"
            )
        lines.append("")

    lines += [
        "---",
        "",
        "## §2 — Cal-Max Hunk Content per (Domain, Seed)",
        "",
    ]

    for domain, analyses in [("FastAPI", fastapi_analyses), ("Rich", rich_analyses)]:
        lines += [f"### {domain}", ""]
        for a in analyses:
            rec = a.cal_max_record
            rel_path = _rel(rec.file_path)
            lines += [
                f"#### seed={a.seed} — bpe={a.cal_max_score:.4f}",
                "",
                f"**File:** `{rel_path}` lines {rec.start_line}–{rec.end_line}",
                "",
                "```python",
                _hunk_preview(rec.text),
                "```",
                "",
            ]

    lines += [
        "---",
        "",
        "## §3 — FP Control Hunk Content",
        "",
    ]

    for domain, analyses in [("FastAPI", fastapi_analyses), ("Rich", rich_analyses)]:
        a2 = next(a for a in analyses if a.seed == 2)
        if a2.fp_ctrl_record is None:
            continue
        rec = a2.fp_ctrl_record
        rel_path = _rel(rec.file_path)
        lines += [
            f"### {domain} seed=2 — ctrl_index={a2.fp_ctrl_index}, bpe={a2.fp_ctrl_score:.4f}, threshold={a2.threshold:.4f}",  # noqa: E501
            "",
            f"**File:** `{rel_path}` lines {rec.start_line}–{rec.end_line}",
            "",
            "```python",
            _hunk_preview(rec.text),
            "```",
            "",
            "**Cal-max for this seed:**",
            f"File: `{_rel(a2.cal_max_record.file_path)}` lines {a2.cal_max_record.start_line}–{a2.cal_max_record.end_line}",  # noqa: E501
            f"Score: {a2.cal_max_score:.4f}",
            "",
        ]

    lines += [
        "---",
        "",
        "## §4 — Holdout Diagnostic",
        "",
        "Remove the cal-max hunk; new threshold = max of remaining cal scores.",
        "If FP no longer fires → dominant-outlier construction artifact confirmed.",
        "",
        "| domain | seed | fp_bpe | threshold | holdout_threshold | gap_max_second | fp_fires_after_holdout |",  # noqa: E501
        "|---|---|---|---|---|---|---|",
    ]

    for domain, analyses in [("fastapi", fastapi_analyses), ("rich", rich_analyses)]:
        a2 = next(a for a in analyses if a.seed == 2)
        if a2.fp_ctrl_score is None or a2.holdout_threshold is None:
            continue
        lines.append(
            f"| {domain}"
            f" | 2"
            f" | {a2.fp_ctrl_score:.4f}"
            f" | {a2.threshold:.4f}"
            f" | {a2.holdout_threshold:.4f}"
            f" | {a2.gap_max_second:.4f}"
            f" | {'YES — FP survives' if a2.fp_fires_after_holdout else 'NO — FP eliminated'} |"
        )

    lines += [
        "",
        "---",
        "",
        "## §5 — Cal-Max Stability Across Seeds",
        "",
        "Does the same source file/hunk appear repeatedly at the calibration ceiling?",
        "",
    ]

    for domain, analyses in [("FastAPI", fastapi_analyses), ("Rich", rich_analyses)]:
        lines += [f"### {domain}", ""]
        lines += [
            "| seed | threshold | cal-max file | lines | score |",
            "|---|---|---|---|---|",
        ]
        for a in analyses:
            rec = a.cal_max_record
            lines.append(
                f"| {a.seed}"
                f" | {a.threshold:.4f}"
                f" | `{_rel(rec.file_path)}`"
                f" | {rec.start_line}–{rec.end_line}"
                f" | {a.cal_max_score:.4f} |"
            )

        # Identify recurring max files
        max_files = [_rel(a.cal_max_record.file_path) for a in analyses]
        file_counts = Counter(max_files)
        lines += [
            "",
            f"Cal-max file frequency across {N_SEEDS} seeds: {dict(file_counts)}",
            "",
        ]

    lines += [
        "---",
        "",
        "## §6 — Diagnosis",
        "",
    ]

    # Auto-generate diagnosis text based on data
    diag_lines: list[str] = []

    for domain, analyses in [("FastAPI", fastapi_analyses), ("Rich", rich_analyses)]:
        a2 = next(a for a in analyses if a.seed == 2)
        gap_second = a2.gap_max_second

        max_files = [_rel(a.cal_max_record.file_path) for a in analyses]
        file_counts = Counter(max_files)
        top_file, top_count = file_counts.most_common(1)[0]
        recurring = top_count >= 3

        if a2.fp_fires_after_holdout is False:
            holdout_finding = "FP eliminated by holdout → dominant-outlier construction artifact"
        elif a2.fp_fires_after_holdout is True:
            holdout_finding = "FP survives holdout → threshold genuinely anchored by distribution"
        else:
            holdout_finding = "holdout not applicable"

        if gap_second > 0.5:
            pattern = "**dominant outlier (fragile)**"
            detail = (
                f"gap(max−2nd)={gap_second:.4f} > 0.5 in seed=2. "
                f"{holdout_finding}. "
                f"The threshold is set by a single anomalous hunk, not the bulk distribution."
            )
        elif gap_second < 0.2:
            pattern = "**dense top (robust noise)**"
            detail = (
                f"gap(max−2nd)={gap_second:.4f} < 0.2 in seed=2. "
                f"{holdout_finding}. "
                "The threshold is genuinely set by the distribution; the FP is boundary noise."
            )
        else:
            pattern = "**borderline (moderate gap)**"
            detail = (
                f"gap(max−2nd)={gap_second:.4f} in seed=2 — moderate, not decisive. "
                f"{holdout_finding}."
            )

        if recurring:
            corpus_note = (
                f"The same file `{top_file}` dominates the ceiling in {top_count}/{N_SEEDS} seeds, "
                f"suggesting a corpus-specific anomaly that is excludable."
            )
        else:
            corpus_note = (
                "The cal-max file varies across seeds (no single file recurs ≥3/5), "
                "suggesting the top is structurally spread across the corpus."
            )

        diag_lines += [
            f"**{domain}:** Pattern = {pattern}.",
            f"{detail} {corpus_note}",
            "",
        ]

    lines += diag_lines

    lines += [
        "---",
        "",
        "## §7 — Recommendation",
        "",
    ]

    # Synthesise recommendation based on both domains
    fa2 = next(a for a in fastapi_analyses if a.seed == 2)
    ri2 = next(a for a in rich_analyses if a.seed == 2)

    fa_gap = fa2.gap_max_second
    ri_gap = ri2.gap_max_second

    fa_fp_survives = fa2.fp_fires_after_holdout
    ri_fp_survives = ri2.fp_fires_after_holdout

    both_outlier = fa_gap > 0.5 and ri_gap > 0.5
    both_dense = fa_gap < 0.2 and ri_gap < 0.2
    both_holdout_eliminates = (fa_fp_survives is False) and (ri_fp_survives is False)
    both_holdout_survive = fa_fp_survives is True and ri_fp_survives is True

    if both_dense or both_holdout_survive:
        rec = (
            "**Keep max(cal).** Both domains show dense-top distribution "
            f"(FastAPI gap={fa_gap:.4f}, rich gap={ri_gap:.4f}) and/or "
            f"FP survives holdout — the threshold is not anchored by a single outlier. "
            "The two FPs are expected boundary noise at 1% FP rate, consistent with VALIDATED."
            " No threshold construction change needed before real-PR validation."
        )
    elif both_outlier or both_holdout_eliminates:
        rec = (
            "**Switch to p99(cal) or trimmed max.** Both domains show outlier-dominated ceiling "
            f"(FastAPI gap={fa_gap:.4f}, rich gap={ri_gap:.4f}) and/or "
            f"FP eliminated by holdout — the threshold is anchored by a single anomalous hunk. "
            "This matches the faker_hunk_0047 construction artifact from exp #2b §6. "
            "Switch to p99(cal) as threshold before real-PR validation;"
            " re-run exp #2c with p99 construction."
        )
    else:
        rec = (
            f"**Mixed signal.** FastAPI: gap={fa_gap:.4f} (fp_survives_holdout={fa_fp_survives}); "
            f"rich: gap={ri_gap:.4f} (fp_survives_holdout={ri_fp_survives}). "
            "Consider switching to p99(cal) as a conservative measure — if it doesn't hurt recall"
            " (all breaks still flagged), it is strictly safer."
            " Validate on exp #2c corpus before committing."
        )

    lines.append(rec)
    lines.append("")

    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written to {_DOCS_OUT}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Diagnostic — Marginal FP inspection", flush=True)

    print("\nEnsuring repos...", flush=True)
    fastapi_repo = _ensure_repo("fastapi", "https://github.com/tiangolo/fastapi")
    rich_repo = _ensure_repo("rich", "https://github.com/Textualize/rich")

    # fp_ctrl_by_seed: seed → ctrl_index of the FP hunk (from exp #2c trace)
    fastapi_fp_by_seed: dict[int, int] = {2: 5}
    rich_fp_by_seed: dict[int, int] = {2: 7}

    print("\nRunning FastAPI (5 seeds)...", flush=True)
    fastapi_analyses = run_domain("FastAPI", fastapi_repo, fastapi_fp_by_seed)

    print("\nRunning rich (5 seeds)...", flush=True)
    rich_analyses = run_domain("rich", rich_repo, rich_fp_by_seed)

    print("\nWriting report...", flush=True)
    _write_report(fastapi_analyses, rich_analyses)


if __name__ == "__main__":
    main()
