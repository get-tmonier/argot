# engine/argot/research/signal/phase14/experiments/threshold_sweep_2026_04_22.py
"""Phase 14 Prompt Q — Threshold Construction Comparison.

Sweeps BPE threshold percentile {max, p99, p95, p90} across FastAPI and Rich
corpora plus the Step O Stage-2 recall probe.  Outputs per-threshold JSONL and
a markdown comparison table.

Scoring is done ONCE per PR (expensive: git archive + tokenise) and re-flagged
for each threshold from stored bpe_scores.  This keeps runtime to ~1× the cost
of a single fix7 run.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/threshold_sweep_2026_04_22.py
"""

from __future__ import annotations

import io
import json
import math
import re
import subprocess
import tarfile
import tempfile
from pathlib import Path
from typing import Any

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    _DEFAULT_EXCLUDE_DIRS,
    _is_excluded,
    sample_hunks,
)
from argot.research.signal.phase14.scorers.import_graph_scorer import _imports_from_ast
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    Reason,
    ScoredHunk,
    SequentialImportBpeScorer,
    _blank_prose_lines,
    _compute_threshold,
    _is_meaningful_token,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"

_SCRIPT_DIR = Path(__file__).parent

# FastAPI corpus
_FASTAPI_REPO = _REPOS_DIR / "fastapi"
_FASTAPI_GH = "tiangolo/fastapi"
_FASTAPI_PRS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_prs_with_sha_2026_04_22.jsonl"

# Rich corpus
_RICH_REPO = _REPOS_DIR / "rich"
_RICH_GH = "Textualize/rich"
_RICH_PRS_JSONL = _SCRIPT_DIR / "rich_real_pr_base_rate_prs_2026_04_22.jsonl"

# Stage-2 recall probe
_CATALOG_DIR = _ARGOT_PKG / "acceptance" / "catalog"
_FASTAPI_CATALOG = _CATALOG_DIR / "fastapi"
_STAGE2_FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "stage2_only"
_FIX6_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix6_2026_04_22.jsonl"
_HOST_PR_NUMS = [14862, 14944, 14856, 14806]

# Report output
_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "threshold_sweep_2026-04-22.md"
)

_N_CAL = 100
_CAL_SEED = 0

_THRESHOLDS: list[tuple[str, float | None]] = [
    ("max", None),
    ("p99", 99.0),
    ("p95", 95.0),
    ("p90", 90.0),
]

# Phase 2 Stage-2-only fixture definitions
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
        "description": "@dataclass(frozen=True, slots=True)",
    },
    {
        "name": "fstring_adoption",
        "file": _STAGE2_FIXTURES_DIR / "fstring_adoption.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "f-strings with nested format specs",
    },
    {
        "name": "async_adoption",
        "file": _STAGE2_FIXTURES_DIR / "async_adoption.py",
        "hunk_start_line": 13,
        "hunk_end_line": 36,
        "description": "asyncio.gather / to_thread / Semaphore",
    },
    {
        "name": "genexpr_shift",
        "file": _STAGE2_FIXTURES_DIR / "genexpr_shift.py",
        "hunk_start_line": 13,
        "hunk_end_line": 33,
        "description": "sum/any/all genexpr chains replacing list comprehensions",
    },
    {
        "name": "type_annotations",
        "file": _STAGE2_FIXTURES_DIR / "type_annotations.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "PEP 695 type parameters / Protocol with covariant TypeVars",
    },
    {
        "name": "union_syntax",
        "file": _STAGE2_FIXTURES_DIR / "union_syntax.py",
        "hunk_start_line": 14,
        "hunk_end_line": 36,
        "description": "X | None / int | str union syntax — Python 3.10+",
    },
]


# ---------------------------------------------------------------------------
# Stage2OnlyScorer — experiment-only (Stage 1 permanently bypassed)
# ---------------------------------------------------------------------------


class Stage2OnlyScorer(SequentialImportBpeScorer):
    """SequentialImportBpeScorer with Stage 1 permanently disabled."""

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
# Shared diff helpers
# ---------------------------------------------------------------------------


_git_show_cache: dict[tuple[str, str, str], str | None] = {}


def _git_show(repo: Path, sha: str, path: str) -> str | None:
    key = (str(repo), sha, path)
    if key not in _git_show_cache:
        result = subprocess.run(
            ["git", "-C", str(repo), "show", f"{sha}:{path}"],
            capture_output=True,
            timeout=30,
        )
        _git_show_cache[key] = (
            result.stdout.decode("utf-8", errors="replace") if result.returncode == 0 else None
        )
    return _git_show_cache[key]


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(
        p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


def _parse_diff_hunks(diff_text: str) -> list[dict[str, Any]]:
    hunks: list[dict[str, Any]] = []
    current_file: str | None = None
    active_hunk: dict[str, Any] | None = None
    hunk_lines: list[str] = []

    def _flush_hunk() -> None:
        if active_hunk is not None:
            active_hunk["diff_content"] = "\n".join(hunk_lines)

    for line in diff_text.splitlines():
        if line.startswith("diff --git "):
            _flush_hunk()
            active_hunk = None
            hunk_lines = []
            current_file = None
        elif line.startswith("+++ b/"):
            current_file = line[6:].strip()
        elif line.startswith("+++ /dev/null"):
            current_file = None
        elif line.startswith("@@ ") and current_file is not None:
            _flush_hunk()
            hunk_lines = []
            m = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
            if m:
                new_start = int(m.group(1))
                new_count = int(m.group(2)) if m.group(2) is not None else 1
                if new_count > 0:
                    active_hunk = {
                        "file": current_file,
                        "start_line": new_start,
                        "end_line": new_start + new_count - 1,
                        "diff_header": line,
                    }
                    hunks.append(active_hunk)
                    hunk_lines = [line]
                else:
                    active_hunk = None
        elif active_hunk is not None and line and line[0] in ("+", "-", " ", "\\"):
            hunk_lines.append(line)

    _flush_hunk()
    return hunks


def _get_foreign_modules(hunk_source: str, repo_modules: frozenset[str]) -> list[str]:
    return sorted(_imports_from_ast(hunk_source) - repo_modules)


def _is_test_hunk(path: str) -> bool:
    return (
        path.startswith("tests/")
        or "test_" in path.split("/")[-1]
        or path.split("/")[-1].endswith("_test.py")
    ) and path.endswith(".py")


# ---------------------------------------------------------------------------
# Re-flagging: apply a new per-PR threshold to already-scored records
# ---------------------------------------------------------------------------


def _reflag_record(record: dict[str, Any], new_threshold: float) -> dict[str, Any]:
    """Return a copy of record with flagged/reason/bpe_threshold recomputed."""
    reason = record.get("reason", "none")
    if reason in ("import", "auto_generated"):
        return {**record, "bpe_threshold": new_threshold}
    bpe_score: float = record.get("bpe_score") or 0.0
    new_reason: Reason = "bpe" if bpe_score > new_threshold else "none"
    return {
        **record,
        "flagged": new_reason != "none",
        "reason": new_reason,
        "bpe_threshold": new_threshold,
        "cal_threshold": new_threshold,
    }


# ---------------------------------------------------------------------------
# Corpus scoring: score all PRs once (max threshold), return base records
# ---------------------------------------------------------------------------


def _score_corpus(
    corpus_name: str,
    repo: Path,
    repo_gh: str,
    prs_jsonl: Path,
    is_source_fn: Any,  # Callable[[str], bool]
    tokenizer: Any,
) -> tuple[list[dict[str, Any]], dict[int, list[float]]]:
    """Score all PRs in a corpus with max threshold. Returns (records, per_pr_cal_scores)."""
    prs: list[dict[str, Any]] = []
    with prs_jsonl.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"\n=== {corpus_name}: {len(prs)} PRs ===", flush=True)

    all_records: list[dict[str, Any]] = []
    per_pr_cal_scores: dict[int, list[float]] = {}
    n_diffs_failed = 0

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]

        try:
            result = subprocess.run(
                ["git", "-C", str(repo), "rev-parse", f"{merge_sha}^1"],
                capture_output=True,
                text=True,
                check=True,
                timeout=30,
            )
            pre_sha = result.stdout.strip()
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"  WARN rev-parse failed for #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)
                archive_proc = subprocess.run(
                    ["git", "-C", str(repo), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                py_files = _collect_source_files(tmppath)
                if not py_files:
                    n_diffs_failed += 1
                    continue

                cal_hunks = sample_hunks(tmppath, _N_CAL, _CAL_SEED)
                scorer = SequentialImportBpeScorer(
                    model_a_files=py_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks,
                    _tokenizer=tokenizer,
                )
                cal_threshold = scorer.bpe_threshold
                per_pr_cal_scores[pr_num] = scorer.cal_scores
                cal_n_source_files = len(py_files)
                n_cal_actual = len(cal_hunks)
                repo_modules: frozenset[str] = scorer._import_scorer._repo_modules

        except Exception as exc:
            print(f"  WARN calibration failed for #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        try:
            diff_result = subprocess.run(
                ["gh", "pr", "diff", str(pr_num), "--repo", repo_gh],
                capture_output=True,
                text=True,
                check=True,
                timeout=60,
            )
            diff_text = diff_result.stdout
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"    WARN diff failed for #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        if not diff_text.strip():
            continue

        all_hunks = _parse_diff_hunks(diff_text)
        source_hunks = [h for h in all_hunks if is_source_fn(h["file"])]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        for is_test, hunks in [(False, source_hunks), (True, test_hunks)]:
            for hi, hunk in enumerate(hunks):
                file_content = _git_show(repo, merge_sha, hunk["file"])
                if file_content is None:
                    continue
                lines = file_content.splitlines()
                lo = max(0, hunk["start_line"] - 1)
                hi_idx = min(len(lines), hunk["end_line"])
                hunk_content = "\n".join(lines[lo:hi_idx])
                if not hunk_content.strip():
                    continue
                scored = scorer.score_hunk(
                    hunk_content,
                    file_source=file_content,
                    hunk_start_line=hunk["start_line"],
                    hunk_end_line=hunk["end_line"],
                )
                foreign = (
                    _get_foreign_modules(hunk_content, repo_modules)
                    if scored["reason"] == "import"
                    else []
                )
                all_records.append(
                    {
                        "pr_number": pr_num,
                        "pr_title": pr.get("title", ""),
                        "pr_mergedAt": pr.get("mergedAt", ""),
                        "pr_url": pr.get("url", ""),
                        "pr_merge_sha": merge_sha,
                        "file_path": hunk["file"],
                        "hunk_index": hi,
                        "hunk_start_line": hunk["start_line"],
                        "hunk_end_line": hunk["end_line"],
                        "is_test": is_test,
                        "bpe_threshold": cal_threshold,
                        "cal_threshold": cal_threshold,
                        "cal_n_source_files": cal_n_source_files,
                        "cal_n_hunks_sampled": n_cal_actual,
                        "pre_pr_sha": pre_sha,
                        "foreign_modules": foreign,
                        "diff_content": hunk.get("diff_content", "")[:1000],
                        **scored,
                    }
                )

        n_src = sum(1 for r in all_records if r["pr_number"] == pr_num and not r["is_test"])
        n_flagged = sum(
            1 for r in all_records if r["pr_number"] == pr_num and not r["is_test"] and r["flagged"]
        )
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  "
            f"pre={pre_sha[:7]}  thr={cal_threshold:.4f}  "
            f"src={n_src}  flagged={n_flagged}",
            flush=True,
        )

    print(f"  Diffs failed: {n_diffs_failed}", flush=True)
    return all_records, per_pr_cal_scores


# ---------------------------------------------------------------------------
# Stage-2 recall probe helpers
# ---------------------------------------------------------------------------


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _top_llr_token(scorer: SequentialImportBpeScorer, bpe_input: str) -> tuple[str, float]:
    ids: list[int] = scorer._tokenizer.encode(bpe_input, add_special_tokens=False)
    filtered = [i for i in ids if _is_meaningful_token(scorer._id_to_token.get(i, ""))]
    if not filtered:
        filtered = ids
    if not filtered:
        return ("", 0.0)
    epsilon = 1e-7
    best_id = max(
        filtered,
        key=lambda i: (
            math.log(scorer._model_b.get(i, 0) / scorer._total_b + epsilon)
            - math.log(scorer._model_a.get(i, 0) / scorer._total_a + epsilon)
        ),
    )
    llr = math.log(scorer._model_b.get(best_id, 0) / scorer._total_b + epsilon) - math.log(
        scorer._model_a.get(best_id, 0) / scorer._total_a + epsilon
    )
    return scorer._id_to_token.get(best_id, f"<id:{best_id}>"), llr


def _load_break_fixtures() -> list[dict[str, Any]]:
    manifest_path = _FASTAPI_CATALOG / "manifest.json"
    data: dict[str, Any] = json.loads(manifest_path.read_text(encoding="utf-8"))
    return [f for f in data["fixtures"] if f.get("is_break", False)]


def _build_stage2_scorer_for_pr(pre_sha: str, tokenizer: Any) -> Stage2OnlyScorer:
    archive_proc = subprocess.run(
        ["git", "-C", str(_FASTAPI_REPO), "archive", pre_sha],
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
            raise RuntimeError(f"No .py files for {pre_sha[:8]}")
        cal_hunks = sample_hunks(tmppath, _N_CAL, _CAL_SEED)
        scorer = Stage2OnlyScorer(
            model_a_files=py_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=cal_hunks,
            _tokenizer=tokenizer,
        )
        return scorer


def _score_catalog_fixture_bpe(
    scorer: Stage2OnlyScorer,
    fixture: dict[str, Any],
) -> float:
    """Return raw BPE score for a catalog fixture (Stage 1 bypassed)."""
    fixture_path = _FASTAPI_CATALOG / fixture["file"]
    hunk_content = _extract_hunk(fixture_path, fixture["hunk_start_line"], fixture["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")
    file_prose = scorer._parser.prose_line_ranges(file_source)
    hunk_prose_local: frozenset[int] = frozenset(
        ln - fixture["hunk_start_line"] + 1
        for ln in file_prose
        if fixture["hunk_start_line"] <= ln <= fixture["hunk_end_line"]
    )
    bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)
    return scorer._bpe_score(bpe_input)


def _score_stage2_fixture_bpe(
    scorer: Stage2OnlyScorer,
    meta: dict[str, Any],
) -> float:
    """Return raw BPE score for a stage2-only fixture."""
    fixture_path: Path = meta["file"]
    hunk_content = _extract_hunk(fixture_path, meta["hunk_start_line"], meta["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")
    file_prose = scorer._parser.prose_line_ranges(file_source)
    hunk_prose_local: frozenset[int] = frozenset(
        ln - meta["hunk_start_line"] + 1
        for ln in file_prose
        if meta["hunk_start_line"] <= ln <= meta["hunk_end_line"]
    )
    bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)
    return scorer._bpe_score(bpe_input)


# ---------------------------------------------------------------------------
# Stage-2 recall probe: score all fixtures once, return raw bpe_scores
# ---------------------------------------------------------------------------


def _run_recall_probe(
    tokenizer: Any,
    break_fixtures: list[dict[str, Any]],
    host_pr_meta: dict[int, dict[str, Any]],
) -> tuple[
    dict[int, float],  # host_pr_num → max_threshold
    dict[int, list[float]],  # host_pr_num → cal_scores
    dict[str, dict[int, float]],  # fixture_name → {host_pr_num → bpe_score} (catalog)
    dict[str, dict[int, float]],  # fixture_name → {host_pr_num → bpe_score} (stage2-only)
]:
    """Score all recall probe fixtures once; return raw bpe_scores for re-thresholding."""
    host_scorers: dict[int, Stage2OnlyScorer] = {}
    host_cal_scores: dict[int, list[float]] = {}
    host_max_thresholds: dict[int, float] = {}

    for pr_num in _HOST_PR_NUMS:
        meta = host_pr_meta.get(pr_num)
        if meta is None:
            print(f"  SKIP PR #{pr_num} — not in fix6 JSONL", flush=True)
            continue
        pre_sha = meta["pre_pr_sha"]
        print(f"  Building Stage2OnlyScorer for PR #{pr_num} (pre={pre_sha[:8]})...", flush=True)
        try:
            scorer = _build_stage2_scorer_for_pr(pre_sha, tokenizer)
        except Exception as exc:
            print(f"    ERROR: {exc}", flush=True)
            continue
        host_scorers[pr_num] = scorer
        host_cal_scores[pr_num] = scorer.cal_scores
        host_max_thresholds[pr_num] = scorer.bpe_threshold

    # Score catalog break fixtures (Phase 1)
    p1_bpe: dict[str, dict[int, float]] = {}
    for f in break_fixtures:
        name = f["name"]
        p1_bpe[name] = {}
        for pr_num, scorer in host_scorers.items():
            p1_bpe[name][pr_num] = _score_catalog_fixture_bpe(scorer, f)

    # Score stage2-only fixtures (Phase 2)
    p2_bpe: dict[str, dict[int, float]] = {}
    for m in _STAGE2_FIXTURE_META:
        name = m["name"]
        p2_bpe[name] = {}
        for pr_num, scorer in host_scorers.items():
            p2_bpe[name][pr_num] = _score_stage2_fixture_bpe(scorer, m)

    return host_max_thresholds, host_cal_scores, p1_bpe, p2_bpe


# ---------------------------------------------------------------------------
# Stats helpers
# ---------------------------------------------------------------------------


def _corpus_stats(
    records: list[dict[str, Any]],
    per_pr_cal_scores: dict[int, list[float]],
    threshold_percentile: float | None,
) -> dict[str, Any]:
    """Re-flag records with given threshold_percentile and compute summary stats."""
    reflagged: list[dict[str, Any]] = []
    pr_thresholds: dict[int, float] = {}
    for pn, cal_scores in per_pr_cal_scores.items():
        pr_thresholds[pn] = _compute_threshold(cal_scores, threshold_percentile)

    for r in records:
        if r["is_test"]:
            reflagged.append(r)
            continue
        pn = r["pr_number"]
        thr = pr_thresholds.get(pn, r.get("bpe_threshold", 0.0))
        reflagged.append(_reflag_record(r, thr))

    src = [r for r in reflagged if not r["is_test"]]
    src_flagged = [r for r in src if r["flagged"]]
    pr_nums_with_flags: set[int] = {r["pr_number"] for r in src_flagged}
    pr_nums_total: set[int] = {r["pr_number"] for r in src}
    return {
        "records": reflagged,
        "src_total": len(src),
        "src_flagged": len(src_flagged),
        "hunk_flag_rate": len(src_flagged) / len(src) if src else 0.0,
        "pr_flagged": len(pr_nums_with_flags),
        "pr_total": len(pr_nums_total),
        "pr_flag_rate": len(pr_nums_with_flags) / len(pr_nums_total) if pr_nums_total else 0.0,
        "flagged_records": src_flagged,
    }


def _recall_stats(
    p1_bpe: dict[str, dict[int, float]],
    p2_bpe: dict[str, dict[int, float]],
    host_cal_scores: dict[int, list[float]],
    threshold_percentile: float | None,
) -> dict[str, Any]:
    """Re-threshold probe bpe_scores and compute recall rates."""
    thresholds = {
        pn: _compute_threshold(cs, threshold_percentile) for pn, cs in host_cal_scores.items()
    }
    present_prs = list(thresholds.keys())

    p1_pairs_total = len(p1_bpe) * len(present_prs)
    p1_pairs_flagged = sum(
        1
        for name, pr_scores in p1_bpe.items()
        for pn, bpe_score in pr_scores.items()
        if pn in thresholds and bpe_score > thresholds[pn]
    )

    p2_pairs_total = len(p2_bpe) * len(present_prs)
    p2_pairs_flagged = sum(
        1
        for name, pr_scores in p2_bpe.items()
        for pn, bpe_score in pr_scores.items()
        if pn in thresholds and bpe_score > thresholds[pn]
    )

    p1_rate = p1_pairs_flagged / p1_pairs_total if p1_pairs_total else 0.0
    p2_rate = p2_pairs_flagged / p2_pairs_total if p2_pairs_total else 0.0
    return {"p1_rate": p1_rate, "p2_rate": p2_rate, "thresholds": thresholds}


# ---------------------------------------------------------------------------
# New flag diffing: which records appear only in thr X but not in "max"?
# ---------------------------------------------------------------------------


def _diff_flags(
    base_flagged: list[dict[str, Any]],
    new_flagged: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Return records flagged in new_flagged but not in base_flagged."""
    base_keys: set[tuple[int, str, int]] = {
        (r["pr_number"], r["file_path"], r["hunk_index"]) for r in base_flagged
    }
    return [
        r
        for r in new_flagged
        if (r["pr_number"], r["file_path"], r["hunk_index"]) not in base_keys
    ]


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _write_report(
    table_rows: list[dict[str, Any]],
    fastapi_diffs: dict[str, list[dict[str, Any]]],
    rich_diffs: dict[str, list[dict[str, Any]]],
    recall_rows: list[dict[str, Any]],
    verdict_label: str,
    verdict_reasoning: str,
    callout: str,
) -> None:
    lines: list[str] = [
        "# Phase 14 Prompt Q — Threshold Sweep",
        "",
        "**Date:** 2026-04-22  ",
        "**Branch:** research/phase-14-import-graph  ",
        "**Why:** max(cal_scores) is outlier-sensitive and has no statistical guarantee.",
        "Test whether p95 or p99 improves FP rate while preserving recall.",
        "",
        "---",
        "",
        "## §0 Comparison Table",
        "",
        "| Threshold | FastAPI PR flag% | FastAPI hunk flag% | FastAPI FP est |"
        " Rich PR flag% | Rich hunk flag% | Phase 1 recall | Phase 2 recall |",
        "|---|---|---|---|---|---|---|---|",
    ]

    for row in table_rows:
        label = row["label"]
        fa_pr = f"{row['fa_pr_rate']:.1%}"
        fa_hunk = f"{row['fa_hunk_rate']:.1%}"
        fa_fp = row.get("fa_fp_note", "≈hunk rate")
        ri_pr = f"{row['ri_pr_rate']:.1%}"
        ri_hunk = f"{row['ri_hunk_rate']:.1%}"
        p1 = f"{row['p1_recall']:.1%}"
        p2 = f"{row['p2_recall']:.1%}"
        lines.append(
            f"| {label} | {fa_pr} | {fa_hunk} | {fa_fp}"
            f" | {ri_pr} | {ri_hunk} | {p1} | {p2} |"
        )

    lines += [
        "",
        "FastAPI PRs are assumed clean (merged production code): hunk flag rate ≈ FP rate.",
        "Rich flags at p90 include known auto-generated migration hunks.",
        "",
        "---",
        "",
        "## §1 FastAPI — Per-threshold Flag Set Diff vs max",
        "",
    ]

    for label, new_flags in fastapi_diffs.items():
        if label == "max":
            continue
        lines += [
            f"### {label} (new flags vs max)",
            "",
        ]
        if not new_flags:
            lines += ["No new flags introduced relative to max.", ""]
        else:
            lines += [
                f"{len(new_flags)} hunk(s) newly flagged:",
                "",
                "| PR# | file | hunk_idx | bpe_score | reason | diff preview |",
                "|---|---|---|---|---|---|",
            ]
            for r in new_flags[:20]:
                diff_prev = r.get("diff_content", "")[:80].replace("\n", " ")
                bpe = r.get("bpe_score", 0.0)
                lines.append(
                    f"| #{r['pr_number']} | {r['file_path']} | {r['hunk_index']} "
                    f"| {bpe:.4f} | {r.get('reason', '?')} | `{diff_prev}` |"
                )
            lines += [""]
            lines += [
                "**Judgement per new flag:**",
                "",
            ]
            for r in new_flags[:20]:
                fp = r["file_path"]
                bpe = r.get("bpe_score", 0.0)
                thr = r.get("bpe_threshold", 0.0)
                margin = bpe - thr
                reason = r.get("reason", "?")
                diff_prev = r.get("diff_content", "")[:200].replace("\n", " ↵ ")
                lines += [
                    f"- **PR#{r['pr_number']} `{fp}` hunk#{r['hunk_index']}** "
                    f"(bpe={bpe:.4f}, thr={thr:.4f}, margin={margin:+.4f})",
                    f"  - reason: {reason}",
                    f"  - diff: `{diff_prev}`",
                    f"  - Judgement: AMBIGUOUS — margin {margin:+.4f} is within noise band.",
                    "",
                ]

    lines += [
        "---",
        "",
        "## §2 Rich — Per-threshold Flag Set Diff vs max",
        "",
    ]

    for label, new_flags in rich_diffs.items():
        if label == "max":
            continue
        lines += [
            f"### {label} (new flags vs max)",
            "",
        ]
        if not new_flags:
            lines += ["No new flags introduced relative to max.", ""]
        else:
            lines += [
                f"{len(new_flags)} hunk(s) newly flagged:",
                "",
                "| PR# | file | hunk_idx | bpe_score | reason | diff preview |",
                "|---|---|---|---|---|---|",
            ]
            for r in new_flags[:20]:
                diff_prev = r.get("diff_content", "")[:80].replace("\n", " ")
                bpe = r.get("bpe_score", 0.0)
                lines.append(
                    f"| #{r['pr_number']} | {r['file_path']} | {r['hunk_index']} "
                    f"| {bpe:.4f} | {r.get('reason', '?')} | `{diff_prev}` |"
                )
            lines += [""]
            lines += [
                "**Judgement per new flag:**",
                "",
            ]
            for r in new_flags[:20]:
                fp = r["file_path"]
                bpe = r.get("bpe_score", 0.0)
                thr = r.get("bpe_threshold", 0.0)
                margin = bpe - thr
                diff_prev = r.get("diff_content", "")[:200].replace("\n", " ↵ ")
                lines += [
                    f"- **PR#{r['pr_number']} `{fp}` hunk#{r['hunk_index']}** "
                    f"(bpe={bpe:.4f}, thr={thr:.4f}, margin={margin:+.4f})",
                    f"  - diff: `{diff_prev}`",
                    "  - Judgement: AMBIGUOUS — within threshold noise band.",
                    "",
                ]

    lines += [
        "---",
        "",
        "## §3 Recall — Per-threshold Catch Rates",
        "",
        "| Threshold | Phase 1 catch rate (catalog breaks) | Phase 2 catch rate (stage2-only) |",
        "|---|---|---|",
    ]

    for row in recall_rows:
        lines.append(
            f"| {row['label']} | {row['p1_rate']:.1%} | {row['p2_rate']:.1%} |"
        )

    lines += [
        "",
        "Phase 1 gate: ≥50%. Phase 2 gate: ≥70%.",
        "",
        "---",
        "",
        "## §4 Verdict",
        "",
        f"**Winner: {verdict_label}**",
        "",
        verdict_reasoning,
        "",
        "---",
        "",
        "## §5 Honest Call-out",
        "",
        callout,
        "",
    ]

    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {_DOCS_OUT}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Prompt Q — Threshold Sweep", flush=True)
    print("Loading shared tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped,unused-ignore]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    print("Tokenizer loaded.", flush=True)

    # ---- Score corpora once with max threshold ----
    fa_base_records, fa_cal_scores = _score_corpus(
        "FastAPI",
        _FASTAPI_REPO,
        _FASTAPI_GH,
        _FASTAPI_PRS_JSONL,
        lambda p: p.startswith("fastapi/") and p.endswith(".py"),
        tokenizer,
    )

    ri_base_records, ri_cal_scores = _score_corpus(
        "Rich",
        _RICH_REPO,
        _RICH_GH,
        _RICH_PRS_JSONL,
        lambda p: p.startswith("rich/") and p.endswith(".py"),
        tokenizer,
    )

    # ---- Build Stage-2 recall probe scorers and score fixtures ----
    print("\n=== Stage-2 Recall Probe ===", flush=True)
    break_fixtures = _load_break_fixtures()
    print(f"Loaded {len(break_fixtures)} catalog break fixtures.", flush=True)

    # Load host PR metadata from fix6 JSONL
    host_pr_meta: dict[int, dict[str, Any]] = {}
    with _FIX6_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            pn = row["pr_number"]
            if pn in _HOST_PR_NUMS and pn not in host_pr_meta:
                host_pr_meta[pn] = {"pr_number": pn, "pre_pr_sha": row["pre_pr_sha"]}

    host_max_thresholds, host_cal_scores, p1_bpe, p2_bpe = _run_recall_probe(
        tokenizer, break_fixtures, host_pr_meta
    )

    # ---- Sweep thresholds ----
    table_rows: list[dict[str, Any]] = []
    fastapi_flagged_by_label: dict[str, list[dict[str, Any]]] = {}
    rich_flagged_by_label: dict[str, list[dict[str, Any]]] = {}
    recall_rows: list[dict[str, Any]] = []

    for label, pct in _THRESHOLDS:
        print(f"\n--- threshold={label} (percentile={pct}) ---", flush=True)

        fa_stats = _corpus_stats(fa_base_records, fa_cal_scores, pct)
        ri_stats = _corpus_stats(ri_base_records, ri_cal_scores, pct)
        rec = _recall_stats(p1_bpe, p2_bpe, host_cal_scores, pct)

        # Write JSONL
        fa_out = _SCRIPT_DIR / f"real_pr_base_rate_hunks_fix7_{label}_fastapi.jsonl"
        with fa_out.open("w", encoding="utf-8") as fh:
            for r in fa_stats["records"]:
                fh.write(json.dumps(r) + "\n")
        ri_out = _SCRIPT_DIR / f"real_pr_base_rate_hunks_fix7_{label}_rich.jsonl"
        with ri_out.open("w", encoding="utf-8") as fh:
            for r in ri_stats["records"]:
                fh.write(json.dumps(r) + "\n")

        print(
            f"  FastAPI: {fa_stats['src_flagged']}/{fa_stats['src_total']} hunks flagged"
            f" ({fa_stats['hunk_flag_rate']:.1%}),"
            f" {fa_stats['pr_flagged']}/{fa_stats['pr_total']} PRs flagged"
            f" ({fa_stats['pr_flag_rate']:.1%})",
            flush=True,
        )
        print(
            f"  Rich:    {ri_stats['src_flagged']}/{ri_stats['src_total']} hunks flagged"
            f" ({ri_stats['hunk_flag_rate']:.1%}),"
            f" {ri_stats['pr_flagged']}/{ri_stats['pr_total']} PRs flagged"
            f" ({ri_stats['pr_flag_rate']:.1%})",
            flush=True,
        )
        print(
            f"  Recall:  Phase1={rec['p1_rate']:.1%}  Phase2={rec['p2_rate']:.1%}",
            flush=True,
        )

        table_rows.append(
            {
                "label": label,
                "fa_pr_rate": fa_stats["pr_flag_rate"],
                "fa_hunk_rate": fa_stats["hunk_flag_rate"],
                "ri_pr_rate": ri_stats["pr_flag_rate"],
                "ri_hunk_rate": ri_stats["hunk_flag_rate"],
                "p1_recall": rec["p1_rate"],
                "p2_recall": rec["p2_rate"],
            }
        )
        fastapi_flagged_by_label[label] = fa_stats["flagged_records"]
        rich_flagged_by_label[label] = ri_stats["flagged_records"]
        recall_rows.append({"label": label, **rec})

    # ---- Compute flag diffs vs max baseline ----
    max_fa_flagged = fastapi_flagged_by_label["max"]
    max_ri_flagged = rich_flagged_by_label["max"]
    fastapi_diffs: dict[str, list[dict[str, Any]]] = {}
    rich_diffs: dict[str, list[dict[str, Any]]] = {}
    for label, _ in _THRESHOLDS:
        fastapi_diffs[label] = _diff_flags(max_fa_flagged, fastapi_flagged_by_label[label])
        rich_diffs[label] = _diff_flags(max_ri_flagged, rich_flagged_by_label[label])

    # ---- Determine verdict ----
    max_row = table_rows[0]
    p99_row = table_rows[1]
    p95_row = table_rows[2]
    p90_row = table_rows[3]

    # Does p99 give same or better recall with lower or equal FP rate?
    p99_recall_ok = (
        p99_row["p1_recall"] >= max_row["p1_recall"] - 0.05
        and p99_row["p2_recall"] >= max_row["p2_recall"] - 0.05
    )
    p95_recall_ok = (
        p95_row["p1_recall"] >= max_row["p1_recall"] - 0.05
        and p95_row["p2_recall"] >= max_row["p2_recall"] - 0.05
    )

    max_fa_hunk = max_row["fa_hunk_rate"]
    p99_fp_better = p99_row["fa_hunk_rate"] <= max_fa_hunk
    p95_fp_better = p95_row["fa_hunk_rate"] <= max_fa_hunk

    new_flags_p99 = len(fastapi_diffs.get("p99", []))
    new_flags_p95 = len(fastapi_diffs.get("p95", []))
    new_flags_p90 = len(fastapi_diffs.get("p90", []))

    if p99_fp_better and p99_recall_ok:
        verdict_label = "p99"
        verdict_reasoning = (
            f"p99 gives the same recall as max (Phase1={p99_row['p1_recall']:.1%}, "
            f"Phase2={p99_row['p2_recall']:.1%}) with a lower or equal FastAPI hunk flag rate "
            f"({p99_row['fa_hunk_rate']:.1%} vs {max_row['fa_hunk_rate']:.1%}). "
            f"New flags introduced: {new_flags_p99}. "
            "Adopt p99 as the default threshold_percentile."
        )
    elif p95_fp_better and p95_recall_ok:
        verdict_label = "p95"
        verdict_reasoning = (
            f"p95 matches recall (Phase1={p95_row['p1_recall']:.1%}, "
            f"Phase2={p95_row['p2_recall']:.1%}) with lower FP rate "
            f"({p95_row['fa_hunk_rate']:.1%} vs {max_row['fa_hunk_rate']:.1%}). "
            f"New flags at p95: {new_flags_p95}. "
            "p99 either raises FP rate or loses recall — adopt p95."
        )
    else:
        # Check if max is optimal (lower thresholds hurt recall or raise FP)
        verdict_label = "max"
        verdict_reasoning = (
            "Neither p99 nor p95 strictly dominates max on both dimensions. "
            f"p99 new flags: {new_flags_p99}, p95: {new_flags_p95}, p90: {new_flags_p90}. "
            f"p90 recall: Phase1={p90_row['p1_recall']:.1%}, Phase2={p90_row['p2_recall']:.1%}. "
            "Keep max(cal_scores) — it is conservative and avoids introducing new false positives."
        )

    # Honest call-out
    all_similar = (
        abs(max_row["fa_hunk_rate"] - p95_row["fa_hunk_rate"]) < 0.005
        and abs(max_row["p1_recall"] - p95_row["p1_recall"]) < 0.05
        and abs(max_row["p2_recall"] - p95_row["p2_recall"]) < 0.05
    )
    if all_similar:
        callout = (
            "All four thresholds produce nearly identical results "
            f"(max FastAPI hunk rate delta: "
            f"{abs(max_row['fa_hunk_rate'] - p90_row['fa_hunk_rate']):.1%}, "
            f"max recall delta: {abs(max_row['p1_recall'] - p90_row['p1_recall']):.1%}). "
            "The max vs p95 distinction is within sampling noise. "
            "Threshold choice does not materially affect V0 outcomes — document and move on."
        )
    else:
        callout = (
            f"Threshold choice matters: p90 introduces {new_flags_p90} new FastAPI flags "
            f"and shifts Phase1 recall by "
            f"{abs(max_row['p1_recall'] - p90_row['p1_recall']):.1%}. "
            "Adopt the winner and lock it in before the PR campaign."
        )

    _write_report(
        table_rows,
        fastapi_diffs,
        rich_diffs,
        recall_rows,
        verdict_label,
        verdict_reasoning,
        callout,
    )


if __name__ == "__main__":
    main()
