# engine/argot/research/signal/phase14/experiments/stage2_recall_cross_corpus_fix9_faker_2026_04_22.py
"""Phase 14 Experiment — Stage 2 Recall Cross-Corpus Probe (fix9).

fix9 change: data-dominant files are now excluded from model A's *training* corpus
(SequentialImportBpeScorer default exclude_data_dominant=True).  Expected: faker's
calibration floor drops from ~7.15 toward ~4-5, lifting Stage 2 fixture catch rate
from 38% toward ≥80%.

Design mirrors fix8 (stage2_recall_cross_corpus_fix8_faker_2026_04_22.py).
FastAPI baseline reused from Step O (not re-run).

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/stage2_recall_cross_corpus_fix9_faker_2026_04_22.py
"""

from __future__ import annotations

import io
import json
import math
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    _DEFAULT_EXCLUDE_DIRS,
    _is_excluded,
    sample_hunks,
)
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    ScoredHunk,
    SequentialImportBpeScorer,
    _blank_prose_lines,
    _compute_threshold,
    _is_meaningful_token,
    Reason,
)

# ---------------------------------------------------------------------------
# Stage2OnlyScorer (Stage 1 permanently disabled)
# ---------------------------------------------------------------------------


class Stage2OnlyScorer(SequentialImportBpeScorer):
    """Stage 1 permanently disabled — BPE-only scoring for isolation probe."""

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
    ) -> ScoredHunk:
        import_score: float = 0.0

        bpe_input = hunk_content
        if file_source is not None and hunk_start_line is not None and hunk_end_line is not None:
            file_prose = self._parser.prose_line_ranges(file_source)
            hunk_prose_local: frozenset[int] = frozenset(
                ln - hunk_start_line + 1
                for ln in file_prose
                if hunk_start_line <= ln <= hunk_end_line
            )
            bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)

        bpe_score: float = self._bpe_score(bpe_input)
        reason: Reason = "bpe" if bpe_score > self.bpe_threshold else "none"

        return {
            "import_score": import_score,
            "bpe_score": bpe_score,
            "flagged": reason != "none",
            "reason": reason,
        }


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"

_SCRIPT_DIR = Path(__file__).parent
_FIX7_RICH_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix7_rich_2026_04_22.jsonl"
_FIX7_FAKER_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix7_faker_2026_04_22.jsonl"

_STAGE2_FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "stage2_only"

_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "stage2_recall_cross_corpus_fix9_faker_2026-04-22.md"
)

_CAL_SEED = 0

# FastAPI baseline from Step O (do not re-run)
_FASTAPI_BASELINE: dict[str, Any] = {
    "corpus": "fastapi",
    "host_prs": [14862, 14944, 14856, 14806],
    "n_cal": 100,
    "median_threshold": 4.0601,  # median of [4.1047, 4.0155, 4.1115, 3.2696]
    "thresholds_by_pr": {
        14862: 4.1047,
        14944: 4.0155,
        14856: 4.1115,
        14806: 3.2696,
    },
    # Per-fixture BPE scores from Step O (scores are corpus-independent — same fixture = same score)
    "fixture_bpe_scores": {
        "walrus_operator": 7.671,
        "match_case": 7.482,
        "dataclass_migration": 5.270,
        "fstring_adoption": 7.418,
        "async_adoption": 7.329,  # 7.242 on #14944, 7.329 otherwise
        "genexpr_shift": 7.388,
        "type_annotations": 6.190,
        "union_syntax": 6.887,  # 6.730 on #14944, 6.887 otherwise
    },
    "catch_rate": 1.0,
    "flagged": 32,
    "total": 32,
}

# Rich host PRs — 5 most recent clean PRs in fix7 run
_RICH_HOST_PR_NUMS = [4079, 4077, 4076, 4075, 3941]
_RICH_N_CAL = 200
_RICH_REPO = _REPOS_DIR / "rich"

# Faker host PRs — 5 most recent clean PRs in fix7 run
_FAKER_HOST_PR_NUMS = [2352, 2351, 2350, 2349, 2348]
_FAKER_N_CAL = 250  # data-dominant filter reduces pool to ~275; N=250 is safe max
_FAKER_REPO = _REPOS_DIR / "faker"

# Fixture definitions (same as Step O)
_STAGE2_FIXTURE_META: list[dict[str, Any]] = [
    {
        "name": "walrus_operator",
        "file": _STAGE2_FIXTURES_DIR / "walrus_operator.py",
        "hunk_start_line": 12,
        "hunk_end_line": 36,
        "description": "walrus := in while/if conditions — Python 3.8+",
    },
    {
        "name": "match_case",
        "file": _STAGE2_FIXTURES_DIR / "match_case.py",
        "hunk_start_line": 12,
        "hunk_end_line": 35,
        "description": "match/case structural pattern matching — Python 3.10+",
    },
    {
        "name": "dataclass_migration",
        "file": _STAGE2_FIXTURES_DIR / "dataclass_migration.py",
        "hunk_start_line": 13,
        "hunk_end_line": 37,
        "description": "@dataclass(frozen=True, slots=True) — host uses plain __init__ classes",
    },
    {
        "name": "fstring_adoption",
        "file": _STAGE2_FIXTURES_DIR / "fstring_adoption.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "f-strings with nested {val!r:>10} format specs throughout",
    },
    {
        "name": "async_adoption",
        "file": _STAGE2_FIXTURES_DIR / "async_adoption.py",
        "hunk_start_line": 13,
        "hunk_end_line": 36,
        "description": "asyncio.gather / to_thread / Semaphore concurrency primitives",
    },
    {
        "name": "genexpr_shift",
        "file": _STAGE2_FIXTURES_DIR / "genexpr_shift.py",
        "hunk_start_line": 13,
        "hunk_end_line": 33,
        "description": "sum/any/all genexpr chains where host uses list comprehensions",
    },
    {
        "name": "type_annotations",
        "file": _STAGE2_FIXTURES_DIR / "type_annotations.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "PEP 695 type parameters def f[T] / Protocol with covariant TypeVars",
    },
    {
        "name": "union_syntax",
        "file": _STAGE2_FIXTURES_DIR / "union_syntax.py",
        "hunk_start_line": 14,
        "hunk_end_line": 36,
        "description": "X | None / int | str union syntax throughout — Python 3.10+",
    },
]


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------


@dataclass
class HostResult:
    pr_number: int
    corpus: str
    pre_sha: str
    threshold_max: float
    threshold_p99: float
    threshold_p95: float
    cal_scores: list[float]
    fixture_results: list[dict[str, Any]]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(
        p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _top_llr_token(scorer: Stage2OnlyScorer, bpe_input: str) -> tuple[str, float]:
    ids: list[int] = scorer._tokenizer.encode(bpe_input, add_special_tokens=False)
    filtered = [i for i in ids if _is_meaningful_token(scorer._id_to_token.get(i, ""))]
    if not filtered:
        filtered = ids
    if not filtered:
        return ("", 0.0)
    epsilon = 1e-7
    total_b = scorer._total_b
    total_a = scorer._total_a
    best_id = max(
        filtered,
        key=lambda i: (
            math.log(scorer._model_b.get(i, 0) / total_b + epsilon)
            - math.log(scorer._model_a.get(i, 0) / total_a + epsilon)
        ),
    )
    llr = math.log(scorer._model_b.get(best_id, 0) / total_b + epsilon) - math.log(
        scorer._model_a.get(best_id, 0) / total_a + epsilon
    )
    return scorer._id_to_token.get(best_id, f"<id:{best_id}>"), llr


def _load_pre_shas_from_jsonl(jsonl_path: Path, pr_nums: list[int]) -> dict[int, str]:
    wanted = set(pr_nums)
    result: dict[int, str] = {}
    with jsonl_path.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            pn = r.get("pr_number")
            if pn in wanted and pn not in result and "pre_pr_sha" in r:
                result[pn] = r["pre_pr_sha"]
            if len(result) == len(wanted):
                break
    return result


def _build_scorer_for_pr(
    repo: Path,
    pre_sha: str,
    n_cal: int,
    tokenizer: Any,
) -> tuple[Stage2OnlyScorer, float, float, list[float]]:
    """Build Stage2OnlyScorer; return (scorer, threshold_max, threshold_p99, cal_scores)."""
    archive_proc = subprocess.run(
        ["git", "-C", str(repo), "archive", pre_sha],
        capture_output=True,
        timeout=120,
    )
    if archive_proc.returncode != 0:
        raise RuntimeError(f"git archive failed for {pre_sha[:8]}")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmppath = Path(tmpdir)
        with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
            tf.extractall(tmppath)

        py_files = _collect_source_files(tmppath)
        if not py_files:
            raise RuntimeError(f"No .py files in archive for {pre_sha[:8]}")

        cal_hunks = sample_hunks(tmppath, n_cal, _CAL_SEED)
        scorer = Stage2OnlyScorer(
            model_a_files=py_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=cal_hunks,
            _tokenizer=tokenizer,
        )
        cal_scores = scorer.cal_scores
        threshold_max = _compute_threshold(cal_scores, None)
        threshold_p99 = _compute_threshold(cal_scores, 99)
        threshold_p95 = _compute_threshold(cal_scores, 95)
        return scorer, threshold_max, threshold_p99, threshold_p95, cal_scores


def _score_fixture(
    scorer: Stage2OnlyScorer,
    meta: dict[str, Any],
    threshold_p99: float,
    threshold_p95: float,
) -> dict[str, Any]:
    from argot.research.signal.phase14.parsers import PythonTreeSitterParser

    fixture_path: Path = meta["file"]
    hunk_content = _extract_hunk(fixture_path, meta["hunk_start_line"], meta["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")

    result = scorer.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=meta["hunk_start_line"],
        hunk_end_line=meta["hunk_end_line"],
    )

    parser = PythonTreeSitterParser()
    file_prose = parser.prose_line_ranges(file_source)
    hunk_prose_local: frozenset[int] = frozenset(
        ln - meta["hunk_start_line"] + 1
        for ln in file_prose
        if meta["hunk_start_line"] <= ln <= meta["hunk_end_line"]
    )
    bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)
    top_token, top_llr = _top_llr_token(scorer, bpe_input)

    bpe_score = result["bpe_score"]
    return {
        "fixture_name": meta["name"],
        "description": meta.get("description", ""),
        "bpe_score": bpe_score,
        "flagged_max": result["flagged"],
        "flagged_p99": bpe_score > threshold_p99,
        "flagged_p95": bpe_score > threshold_p95,
        "margin_max": bpe_score - scorer.bpe_threshold,
        "margin_p99": bpe_score - threshold_p99,
        "margin_p95": bpe_score - threshold_p95,
        "top_token": top_token,
        "top_llr": top_llr,
    }


def _run_corpus(
    corpus: str,
    repo: Path,
    host_pr_nums: list[int],
    n_cal: int,
    jsonl_path: Path,
    tokenizer: Any,
) -> list[HostResult]:
    print(f"\n{'=' * 60}", flush=True)
    print(f"Corpus: {corpus}  N_CAL={n_cal}  host PRs={host_pr_nums}", flush=True)
    print(f"{'=' * 60}", flush=True)

    pre_shas = _load_pre_shas_from_jsonl(jsonl_path, host_pr_nums)
    missing = set(host_pr_nums) - set(pre_shas.keys())
    if missing:
        print(f"  WARN: pre_sha not found for PRs {missing}", flush=True)

    results: list[HostResult] = []

    for pr_num in host_pr_nums:
        pre_sha = pre_shas.get(pr_num)
        if pre_sha is None:
            print(f"  SKIP PR #{pr_num} — no pre_sha", flush=True)
            continue

        print(f"\nPR #{pr_num} pre_sha={pre_sha[:8]} ...", flush=True)

        try:
            scorer, thr_max, thr_p99, thr_p95, cal_scores = _build_scorer_for_pr(
                repo, pre_sha, n_cal, tokenizer
            )
        except Exception as exc:
            print(f"  ERROR building scorer: {exc}", flush=True)
            continue

        print(
            f"  threshold: max={thr_max:.4f}  p99={thr_p99:.4f}  p95={thr_p95:.4f}  "
            f"n_cal={len(cal_scores)}",
            flush=True,
        )

        fixture_results: list[dict[str, Any]] = []
        n_flagged_max = 0
        n_flagged_p99 = 0

        for meta in _STAGE2_FIXTURE_META:
            fr = _score_fixture(scorer, meta, thr_p99, thr_p95)
            fixture_results.append(fr)
            if fr["flagged_max"]:
                n_flagged_max += 1
            if fr["flagged_p99"]:
                n_flagged_p99 += 1
            marker = "YES" if fr["flagged_max"] else "no "
            p99_marker = "YES" if fr["flagged_p99"] else "no "
            print(
                f"    {fr['fixture_name']}: max={marker} p99={p99_marker}"
                f" bpe={fr['bpe_score']:.4f} margin_max={fr['margin_max']:+.4f}",
                flush=True,
            )

        print(
            f"  -> Flagged: max={n_flagged_max}/{len(_STAGE2_FIXTURE_META)}"
            f"  p99={n_flagged_p99}/{len(_STAGE2_FIXTURE_META)}",
            flush=True,
        )

        results.append(
            HostResult(
                pr_number=pr_num,
                corpus=corpus,
                pre_sha=pre_sha,
                threshold_max=thr_max,
                threshold_p99=thr_p99,
                threshold_p95=thr_p95,
                cal_scores=cal_scores,
                fixture_results=fixture_results,
            )
        )

    return results


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _median(vals: list[float]) -> float:
    if not vals:
        return 0.0
    s = sorted(vals)
    n = len(s)
    if n % 2 == 1:
        return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2.0


def _write_report(
    out: Path,
    rich_results: list[HostResult],
    faker_results: list[HostResult],
) -> None:
    fixture_names = [m["name"] for m in _STAGE2_FIXTURE_META]

    def _median_bpe(results: list[HostResult], fixture_name: str) -> float:
        scores = [
            fr["bpe_score"]
            for hr in results
            for fr in hr.fixture_results
            if fr["fixture_name"] == fixture_name
        ]
        return _median(scores)

    def _median_threshold_max(results: list[HostResult]) -> float:
        return _median([hr.threshold_max for hr in results])

    def _median_threshold_p99(results: list[HostResult]) -> float:
        return _median([hr.threshold_p99 for hr in results])

    def _median_threshold_p95(results: list[HostResult]) -> float:
        return _median([hr.threshold_p95 for hr in results])

    def _catch_rate(results: list[HostResult], threshold_key: str) -> tuple[int, int]:
        flagged_key = f"flagged_{threshold_key}"
        flagged = sum(1 for hr in results for fr in hr.fixture_results if fr[flagged_key])
        total = sum(len(hr.fixture_results) for hr in results)
        return flagged, total

    fa_bpe = _FASTAPI_BASELINE["fixture_bpe_scores"]
    fa_thresholds = list(_FASTAPI_BASELINE["thresholds_by_pr"].values())
    fa_thr_median = _median(fa_thresholds)

    lines: list[str] = [
        "# Phase 14 Stage-2 Recall Cross-Corpus Probe — fix9 (2026-04-22)",
        "",
        "**fix9 change:** Data-dominant files excluded from model A training corpus",
        "(SequentialImportBpeScorer default exclude_data_dominant=True).",
        "Expected: faker calibration floor drops from ~7.15 → ~4-5; catch rate rises from 38% → ≥80%.",
        "",
        "**Method:** `Stage2OnlyScorer` (Stage 1 permanently disabled) calibrated on each host",
        "corpus's pre-merge snapshot. All 8 stage2_only fixtures injected into 4-5 clean host PRs",
        "per corpus. FastAPI baseline reused from Step O (not re-run).",
        "",
        "| Corpus | N_CAL | Host PRs | Note |",
        "|---|---|---|---|",
        f"| FastAPI (baseline) | 100 | #14862, #14944, #14856, #14806 | Step O result reused |",
        f"| Rich | {_RICH_N_CAL} | {', '.join(f'#{p}' for p in _RICH_HOST_PR_NUMS)} |"
        " Regression check — expect same as fix8 |",
        f"| Faker | {_FAKER_N_CAL} | {', '.join(f'#{p}' for p in _FAKER_HOST_PR_NUMS)} |"
        " Primary fix9 result |",
        "",
        "---",
        "",
        "## §0 Three-Host Summary Table",
        "",
        "Median BPE score across host PRs per fixture, with max-threshold flag status.",
        "",
    ]

    header = "| fixture |"
    header += " FastAPI bpe | FastAPI flag |"
    if rich_results:
        header += " Rich bpe | Rich thr | Rich flag |"
    if faker_results:
        header += " Faker bpe | Faker thr | Faker flag |"
    sep = "|---|" + "---|---|" * 3

    lines += [header, sep]

    for fx_name in fixture_names:
        fa_bpe_score = fa_bpe.get(fx_name, 0.0)
        fa_flagged = fa_bpe_score > fa_thr_median
        row = f"| {fx_name} | {fa_bpe_score:.3f} | {'YES' if fa_flagged else 'no'} |"

        if rich_results:
            r_bpe = _median_bpe(rich_results, fx_name)
            r_thr = _median_threshold_max(rich_results)
            r_flag = r_bpe > r_thr
            row += f" {r_bpe:.3f} | {r_thr:.3f} | {'YES' if r_flag else 'no'} |"

        if faker_results:
            f_bpe = _median_bpe(faker_results, fx_name)
            f_thr = _median_threshold_max(faker_results)
            f_flag = f_bpe > f_thr
            row += f" {f_bpe:.3f} | {f_thr:.3f} | {'YES' if f_flag else 'no'} |"

        lines.append(row)

    lines += ["", "---", "", "## §1 Score Homogeneity Check", ""]
    lines.append(
        "For each fixture, are BPE scores roughly comparable across hosts (within ±0.5)?"
    )
    lines += [
        "",
        "| fixture | FastAPI (Step O) | Rich median | Faker median | max gap | homogeneous? |",
        "|---|---|---|---|---|---|",
    ]

    all_homogeneous = True
    for fx_name in fixture_names:
        fa_s = fa_bpe.get(fx_name, 0.0)
        r_s = _median_bpe(rich_results, fx_name) if rich_results else float("nan")
        f_s = _median_bpe(faker_results, fx_name) if faker_results else float("nan")

        valid_scores = [s for s in [fa_s, r_s, f_s] if not math.isnan(s)]
        gap = max(valid_scores) - min(valid_scores) if valid_scores else 0.0
        homogeneous = gap <= 0.5
        if not homogeneous:
            all_homogeneous = False

        r_str = f"{r_s:.3f}" if not math.isnan(r_s) else "N/A"
        f_str = f"{f_s:.3f}" if not math.isnan(f_s) else "N/A"
        lines.append(
            f"| {fx_name} | {fa_s:.3f} | {r_str} | {f_str} | {gap:.3f} |"
            f" {'YES' if homogeneous else '**NO**'} |"
        )

    homogeneity_verdict = (
        "All fixtures are score-homogeneous (gap ≤0.5) across corpora."
        if all_homogeneous
        else "Score divergence detected in ≥1 fixture — content/corpus mismatch may contribute."
    )
    lines += ["", f"**Verdict:** {homogeneity_verdict}", ""]

    lines += [
        "---",
        "",
        "## §2 Threshold Regime Comparison",
        "",
        "| Corpus | N_CAL | Median threshold (max) | Median p99 | Median p95 |",
        "|---|---|---|---|---|",
        f"| FastAPI (Step O) | 100 | {fa_thr_median:.4f} | (not computed in Step O) | — |",
    ]

    if rich_results:
        r_med_max = _median_threshold_max(rich_results)
        r_med_p99 = _median_threshold_p99(rich_results)
        r_med_p95 = _median_threshold_p95(rich_results)
        lines.append(
            f"| Rich | {_RICH_N_CAL} | {r_med_max:.4f} | {r_med_p99:.4f} | {r_med_p95:.4f} |"
        )

    if faker_results:
        f_med_max = _median_threshold_max(faker_results)
        f_med_p99 = _median_threshold_p99(faker_results)
        f_med_p95 = _median_threshold_p95(faker_results)
        lines.append(
            f"| Faker | {_FAKER_N_CAL} | {f_med_max:.4f} | {f_med_p99:.4f} | {f_med_p95:.4f} |"
        )

    lines += [
        "",
        "### Per-host threshold breakdown",
        "",
        "| Corpus | PR | pre_sha | max threshold | p99 | p95 | n_cal |",
        "|---|---|---|---|---|---|---|",
    ]

    for pn, thr in _FASTAPI_BASELINE["thresholds_by_pr"].items():
        lines.append(f"| FastAPI | #{pn} | (Step O) | {thr:.4f} | — | — | 100 |")

    for hr in rich_results:
        lines.append(
            f"| Rich | #{hr.pr_number} | {hr.pre_sha[:8]} |"
            f" {hr.threshold_max:.4f} | {hr.threshold_p99:.4f} | {hr.threshold_p95:.4f} |"
            f" {len(hr.cal_scores)} |"
        )

    for hr in faker_results:
        lines.append(
            f"| Faker | #{hr.pr_number} | {hr.pre_sha[:8]} |"
            f" {hr.threshold_max:.4f} | {hr.threshold_p99:.4f} | {hr.threshold_p95:.4f} |"
            f" {len(hr.cal_scores)} |"
        )

    lines += ["", "---", "", "## §3 Catch Rate per Host", ""]

    fa_catch = _FASTAPI_BASELINE["flagged"]
    fa_total = _FASTAPI_BASELINE["total"]
    lines += [
        "| Corpus | Fixtures flagged | Total pairs | Catch rate |",
        "|---|---|---|---|",
        f"| FastAPI (Step O, max threshold, N=100) | {fa_catch}/{fa_total} |"
        f" {fa_total} | {fa_catch / fa_total:.0%} |",
    ]

    if rich_results:
        r_flagged, r_total = _catch_rate(rich_results, "max")
        lines.append(
            f"| Rich (max threshold, N={_RICH_N_CAL}) | {r_flagged}/{r_total} |"
            f" {r_total} | {r_flagged / r_total:.0%} |"
            if r_total
            else "| Rich | 0/0 | 0 | N/A |"
        )

    if faker_results:
        f_flagged, f_total = _catch_rate(faker_results, "max")
        lines.append(
            f"| Faker (max threshold, N={_FAKER_N_CAL}) | {f_flagged}/{f_total} |"
            f" {f_total} | {f_flagged / f_total:.0%} |"
            if f_total
            else "| Faker | 0/0 | 0 | N/A |"
        )

    lines += [""]

    for label, corpus_results, n_cal in [
        ("Rich", rich_results, _RICH_N_CAL),
        ("Faker", faker_results, _FAKER_N_CAL),
    ]:
        if not corpus_results:
            continue
        lines += [
            f"### {label} — per-fixture × per-host score table (max threshold)",
            "",
            "Cell format: `YES bpe>thr` / `no bpe<thr`.",
            "",
        ]
        pr_nums = [hr.pr_number for hr in corpus_results]
        header2 = "| fixture |" + "".join(f" #{pn} |" for pn in pr_nums) + " median |"
        sep2 = "|---|" + "".join("---|" for _ in pr_nums) + "---|"
        lines += [header2, sep2]

        lookup: dict[str, dict[int, dict[str, Any]]] = {n: {} for n in fixture_names}
        for hr in corpus_results:
            for fr in hr.fixture_results:
                lookup[fr["fixture_name"]][hr.pr_number] = {**fr, "threshold_max": hr.threshold_max}

        for fx_name in fixture_names:
            row = f"| {fx_name} |"
            bpe_vals: list[float] = []
            for hr in corpus_results:
                fr = lookup[fx_name].get(hr.pr_number)
                if fr is None:
                    row += " N/A |"
                elif fr["flagged_max"]:
                    row += f" **YES** {fr['bpe_score']:.3f}>{hr.threshold_max:.3f} |"
                    bpe_vals.append(fr["bpe_score"])
                else:
                    row += f" no {fr['bpe_score']:.3f}<{hr.threshold_max:.3f} |"
                    bpe_vals.append(fr["bpe_score"])
            med = _median(bpe_vals)
            row += f" {med:.3f} |"
            lines.append(row)
        lines.append("")

    lines += ["---", "", "## §4 Counterfactual Threshold Analysis", ""]
    lines.append("Re-compute catch rate at p99 and p95 thresholds.")
    lines += [
        "",
        "| Corpus | Threshold | Catch rate | Fixtures flagged | Total pairs |",
        "|---|---|---|---|---|",
        f"| FastAPI | max (~{fa_thr_median:.2f}) | {fa_catch / fa_total:.0%} |"
        f" {fa_catch}/{fa_total} | {fa_total} |",
    ]

    for label, corpus_results, n_cal in [
        ("Rich", rich_results, _RICH_N_CAL),
        ("Faker", faker_results, _FAKER_N_CAL),
    ]:
        if not corpus_results:
            continue
        for thr_label in ["max", "p99", "p95"]:
            flagged, total = _catch_rate(corpus_results, thr_label)
            med_thr = (
                _median_threshold_max(corpus_results)
                if thr_label == "max"
                else _median_threshold_p99(corpus_results)
                if thr_label == "p99"
                else _median_threshold_p95(corpus_results)
            )
            lines.append(
                f"| {label} | {thr_label} (~{med_thr:.2f}) |"
                f" {flagged / total:.0%} | {flagged}/{total} | {total} |"
                if total
                else f"| {label} | {thr_label} | N/A | 0/0 | 0 |"
            )

    lines += [
        "",
        "### Counterfactual per-fixture detail (Faker)",
        "",
        "| fixture | Faker max margin | Faker p99 margin | Faker p95 margin |",
        "|---|---|---|---|",
    ]

    if faker_results:
        for fx_name in fixture_names:
            margins_max = [
                fr["margin_max"]
                for hr in faker_results
                for fr in hr.fixture_results
                if fr["fixture_name"] == fx_name
            ]
            margins_p99 = [
                fr["margin_p99"]
                for hr in faker_results
                for fr in hr.fixture_results
                if fr["fixture_name"] == fx_name
            ]
            margins_p95 = [
                fr["margin_p95"]
                for hr in faker_results
                for fr in hr.fixture_results
                if fr["fixture_name"] == fx_name
            ]
            med_max = _median(margins_max)
            med_p99 = _median(margins_p99)
            med_p95 = _median(margins_p95)
            flag_max = "YES" if med_max > 0 else "**no**"
            flag_p99 = "YES" if med_p99 > 0 else "**no**"
            flag_p95 = "YES" if med_p95 > 0 else "**no**"
            lines.append(
                f"| {fx_name} | {flag_max} {med_max:+.3f} |"
                f" {flag_p99} {med_p99:+.3f} | {flag_p95} {med_p95:+.3f} |"
            )

    lines += [
        "",
        "---",
        "",
        "## §5 Verdict",
        "",
    ]

    if faker_results:
        f_flagged_max, f_total = _catch_rate(faker_results, "max")
        f_flagged_p99, _ = _catch_rate(faker_results, "p99")
        f_med_max = _median_threshold_max(faker_results)
        f_med_p99 = _median_threshold_p99(faker_results)

        faker_max_catch = f_flagged_max / f_total if f_total else 0.0
        faker_p99_catch = f_flagged_p99 / f_total if f_total else 0.0

        all_gaps: list[float] = []
        for fx_name in fixture_names:
            fa_s = fa_bpe.get(fx_name, 0.0)
            f_s = _median_bpe(faker_results, fx_name)
            all_gaps.append(abs(fa_s - f_s))
        max_gap = max(all_gaps) if all_gaps else 0.0

        if faker_max_catch >= 0.75:
            main_verdict = (
                f"**fix9 CONFIRMED:** Excluding locale files from model A training lifted "
                f"faker catch rate to {faker_max_catch:.0%} (was 38% in fix8). "
                f"Median threshold: {f_med_max:.4f} (was ~7.15). "
                f"The LLR inflation hypothesis is validated."
            )
        elif faker_max_catch > 0.38:
            main_verdict = (
                f"**fix9 PARTIAL:** Catch rate rose from 38% → {faker_max_catch:.0%}. "
                f"Median threshold: {f_med_max:.4f} (was ~7.15). "
                f"Some floor remains — faker non-locale code still uses rare vocabulary "
                f"even after locale exclusion."
            )
        elif faker_max_catch <= 0.38:
            main_verdict = (
                f"**fix9 NO IMPROVEMENT:** Catch rate unchanged at {faker_max_catch:.0%}. "
                f"Median threshold: {f_med_max:.4f} (was ~7.15). "
                f"Model A training exclusion alone is not sufficient — "
                f"the floor is content-driven beyond locale vocabulary."
            )
        else:
            main_verdict = "**Inconclusive** — insufficient results."

        lines += [
            main_verdict,
            "",
            f"fix8 baseline: 38% catch rate, median threshold ~7.15",
            f"fix9 result:   {faker_max_catch:.0%} catch rate, median threshold {f_med_max:.4f}",
            "",
        ]

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Stage-2 Recall Cross-Corpus Probe (fix9)", flush=True)
    print("FastAPI baseline (Step O): reused — not re-run.", flush=True)

    print("\nLoading shared tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    print("Tokenizer loaded.", flush=True)

    rich_results = _run_corpus(
        corpus="rich",
        repo=_RICH_REPO,
        host_pr_nums=_RICH_HOST_PR_NUMS,
        n_cal=_RICH_N_CAL,
        jsonl_path=_FIX7_RICH_JSONL,
        tokenizer=tokenizer,
    )

    faker_results = _run_corpus(
        corpus="faker",
        repo=_FAKER_REPO,
        host_pr_nums=_FAKER_HOST_PR_NUMS,
        n_cal=_FAKER_N_CAL,
        jsonl_path=_FIX7_FAKER_JSONL,
        tokenizer=tokenizer,
    )

    print("\n" + "=" * 60, flush=True)
    print("SUMMARY", flush=True)
    print("=" * 60, flush=True)

    fa_catch = _FASTAPI_BASELINE["flagged"]
    fa_total = _FASTAPI_BASELINE["total"]
    print(f"FastAPI (Step O): {fa_catch}/{fa_total} = {fa_catch / fa_total:.0%}", flush=True)

    if rich_results:
        r_f, r_t = sum(
            1 for hr in rich_results for fr in hr.fixture_results if fr["flagged_max"]
        ), sum(len(hr.fixture_results) for hr in rich_results)
        print(
            f"Rich (max):  {r_f}/{r_t} = {r_f / r_t:.0%}" if r_t else "Rich: no results",
            flush=True,
        )

    if faker_results:
        f_f, f_t = sum(
            1 for hr in faker_results for fr in hr.fixture_results if fr["flagged_max"]
        ), sum(len(hr.fixture_results) for hr in faker_results)
        f_p99, _ = (
            sum(1 for hr in faker_results for fr in hr.fixture_results if fr["flagged_p99"]),
            f_t,
        )
        print(
            f"Faker (max): {f_f}/{f_t} = {f_f / f_t:.0%}" if f_t else "Faker: no results",
            flush=True,
        )
        print(
            f"Faker (p99): {f_p99}/{f_t} = {f_p99 / f_t:.0%}" if f_t else "",
            flush=True,
        )

    _write_report(_DOCS_OUT, rich_results, faker_results)


if __name__ == "__main__":
    main()
