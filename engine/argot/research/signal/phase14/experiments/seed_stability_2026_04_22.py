# engine/argot/research/signal/phase14/experiments/seed_stability_2026_04_22.py
"""Phase 14 Prompt R — Seed Stability Experiment.

Tests whether calibration at N=100, seeds {0-4} produces stable thresholds and flag
sets across FastAPI PRs.  If the gate fails, sweeps N={200,500} to find the smallest
stable N.

Strategy: score each PR hunk once (BPE scores are seed-independent; seed only affects
which calibration hunks are drawn, hence the threshold).  For each seed, we re-sample
calibration hunks within the same tmpdir and recompute cal_scores without re-extracting
the git archive or rebuilding model_a.

Outputs:
  real_pr_base_rate_hunks_fix7_seed{0..4}_fastapi.jsonl  (one per seed, N=100)
  docs/research/scoring/signal/phase14/experiments/seed_stability_2026-04-22.md

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/seed_stability_2026_04_22.py
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

import numpy as np

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    _DEFAULT_EXCLUDE_DIRS,
    _is_excluded,
    collect_candidates,
    sample_hunks,
)
from argot.research.signal.phase14.scorers.import_graph_scorer import _imports_from_ast
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
    _blank_prose_lines,
    _compute_threshold,
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
_PRS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_prs_with_sha_2026_04_22.jsonl"

_FASTAPI_REPO = _REPOS_DIR / "fastapi"
_REPO_GH = "tiangolo/fastapi"

_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "seed_stability_2026-04-22.md"
)

# Experiment parameters
_SEEDS = [0, 1, 2, 3, 4]
_N_BASE = 100
_N_SWEEP = [200, 500]

# Stability gates
_THRESH_VAR_GATE = 0.10  # relative variance > 10% on any PR → unstable
_JACCARD_GATE = 0.80  # Jaccard < 80% on any seed pair → unstable

# In-process cache for git show
_git_show_cache: dict[tuple[str, str], str | None] = {}


def _git_show(repo: Path, sha: str, path: str) -> str | None:
    key = (sha, path)
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


def _is_source_hunk(path: str) -> bool:
    return path.startswith("fastapi/") and path.endswith(".py")


def _is_test_hunk(path: str) -> bool:
    return (
        path.startswith("tests/")
        or "test_" in path.split("/")[-1]
        or path.split("/")[-1].endswith("_test.py")
    ) and path.endswith(".py")


def _compute_multi_seed_thresholds(
    scorer: SequentialImportBpeScorer,
    candidates: list[str],
    seeds: list[int],
    n_values: list[int],
) -> dict[tuple[int, int], float]:
    """Return {(seed, n): max_threshold} for all seed × n combinations.

    Uses the already-built scorer's _bpe_score to compute cal_scores without
    re-extracting the archive.  Candidates are pre-collected from the tmpdir.
    """
    result: dict[tuple[int, int], float] = {}
    for n in n_values:
        if len(candidates) < n:
            for seed in seeds:
                result[(seed, n)] = float("nan")
            continue
        for seed in seeds:
            rng = np.random.default_rng(seed)
            indices = rng.choice(len(candidates), size=n, replace=False)
            hunks = [candidates[int(i)] for i in sorted(indices)]
            cal_scores = [
                scorer._bpe_score(
                    _blank_prose_lines(h, scorer._parser.prose_line_ranges(h))
                )
                for h in hunks
            ]
            result[(seed, n)] = _compute_threshold(cal_scores, None)
    return result


def _jaccard(a: set[Any], b: set[Any]) -> float:
    if not a and not b:
        return 1.0
    union = a | b
    if not union:
        return 1.0
    return len(a & b) / len(union)


# ---------------------------------------------------------------------------
# Main scoring loop
# ---------------------------------------------------------------------------


def _run_experiment(
    tokenizer: Any,
    prs: list[dict[str, Any]],
    n_values: list[int],
) -> dict[str, Any]:
    """Score all PRs once; collect per-seed thresholds for all n_values.

    Returns:
        {
            "pr_records": list[dict]  # base hunk records (scored at seed=0, N=n_values[0])
            "per_pr_thresholds": {pr_num: {(seed, n): threshold}}
            "per_pr_meta": {pr_num: {pre_sha, n_cal_actual, cal_n_source_files}}
            "n_diffs_failed": int
        }
    """
    all_records: list[dict[str, Any]] = []
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]] = {}
    per_pr_meta: dict[int, dict[str, Any]] = {}
    n_diffs_failed = 0

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]
        pre_sha_ref = f"{merge_sha}^1"

        try:
            result = subprocess.run(
                ["git", "-C", str(_FASTAPI_REPO), "rev-parse", pre_sha_ref],
                capture_output=True,
                text=True,
                check=True,
                timeout=30,
            )
            pre_sha = result.stdout.strip()
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"  WARN: rev-parse failed for {pre_sha_ref}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)

                archive_proc = subprocess.run(
                    ["git", "-C", str(_FASTAPI_REPO), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    print(f"  WARN: git archive failed for {pre_sha[:8]}, skipping", flush=True)
                    n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                py_files = _collect_source_files(tmppath)
                if not py_files:
                    n_diffs_failed += 1
                    continue

                # Build scorer with seed=0, N=n_values[0] for actual hunk scoring
                cal_hunks_seed0 = sample_hunks(tmppath, n_values[0], 0)
                scorer = SequentialImportBpeScorer(
                    model_a_files=py_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks_seed0,
                    _tokenizer=tokenizer,
                )
                cal_threshold_seed0 = scorer.bpe_threshold
                n_cal_actual = len(cal_hunks_seed0)
                cal_n_source_files = len(py_files)
                repo_modules: frozenset[str] = scorer._import_scorer._repo_modules

                # Collect candidates once for multi-seed threshold sweep
                candidates = collect_candidates(tmppath)
                thresholds = _compute_multi_seed_thresholds(scorer, candidates, _SEEDS, n_values)

        except Exception as exc:
            print(f"  WARN: calibration failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        per_pr_thresholds[pr_num] = thresholds
        per_pr_meta[pr_num] = {
            "pre_sha": pre_sha,
            "cal_n_source_files": cal_n_source_files,
            "n_cal_actual": n_cal_actual,
            "n_candidates": len(candidates),
        }

        # Fetch diff
        try:
            diff_result = subprocess.run(
                ["gh", "pr", "diff", str(pr_num), "--repo", _REPO_GH],
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
        source_hunks = [h for h in all_hunks if _is_source_hunk(h["file"])]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        for is_test, hunks in [(False, source_hunks), (True, test_hunks)]:
            for hi, hunk in enumerate(hunks):
                file_content = _git_show(_FASTAPI_REPO, merge_sha, hunk["file"])
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
                    sorted(_imports_from_ast(hunk_content) - repo_modules)
                    if scored["reason"] == "import"
                    else []
                )
                all_records.append({
                    "pr_number": pr_num,
                    "pr_title": pr["title"],
                    "pr_mergedAt": pr["mergedAt"],
                    "pr_url": pr["url"],
                    "pr_merge_sha": merge_sha,
                    "file_path": hunk["file"],
                    "hunk_index": hi,
                    "hunk_start_line": hunk["start_line"],
                    "hunk_end_line": hunk["end_line"],
                    "is_test": is_test,
                    "bpe_threshold": cal_threshold_seed0,
                    "cal_threshold": cal_threshold_seed0,
                    "cal_n_source_files": cal_n_source_files,
                    "cal_n_hunks_sampled": n_cal_actual,
                    "pre_pr_sha": pre_sha,
                    "foreign_modules": foreign,
                    "diff_content": hunk.get("diff_content", "")[:1000],
                    **scored,
                })

        n_src = sum(1 for r in all_records if r["pr_number"] == pr_num and not r["is_test"])
        n_flagged = sum(
            1 for r in all_records if r["pr_number"] == pr_num and not r["is_test"] and r["flagged"]
        )
        thr_vals = [v for (s, n), v in thresholds.items() if n == n_values[0] and not math.isnan(v)]
        thr_range = f"{min(thr_vals):.4f}–{max(thr_vals):.4f}" if len(thr_vals) > 1 else f"{cal_threshold_seed0:.4f}"
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  {pr['mergedAt'][:10]}  "
            f"pre={pre_sha[:7]}  thr_range={thr_range}  "
            f"src={n_src}  flagged={n_flagged}",
            flush=True,
        )

    return {
        "pr_records": all_records,
        "per_pr_thresholds": per_pr_thresholds,
        "per_pr_meta": per_pr_meta,
        "n_diffs_failed": n_diffs_failed,
    }


def _reflag_records(
    records: list[dict[str, Any]],
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]],
    seed: int,
    n: int,
) -> list[dict[str, Any]]:
    """Return records re-flagged with (seed, n) threshold."""
    out: list[dict[str, Any]] = []
    for r in records:
        if r["is_test"]:
            out.append(r)
            continue
        pn = r["pr_number"]
        thr = per_pr_thresholds.get(pn, {}).get((seed, n))
        if thr is None or math.isnan(thr):
            out.append(r)
            continue
        reason = r.get("reason", "none")
        if reason in ("import", "auto_generated"):
            out.append({**r, "bpe_threshold": thr, "cal_threshold": thr})
            continue
        bpe_score: float = r.get("bpe_score") or 0.0
        new_reason = "bpe" if bpe_score > thr else "none"
        out.append({
            **r,
            "flagged": new_reason != "none",
            "reason": new_reason,
            "bpe_threshold": thr,
            "cal_threshold": thr,
        })
    return out


def _compute_stability_metrics(
    pr_records: list[dict[str, Any]],
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]],
    seeds: list[int],
    n: int,
) -> dict[str, Any]:
    """Compute per-PR threshold variance and flag-set stability."""
    pr_nums = sorted({r["pr_number"] for r in pr_records if not r["is_test"]})

    # Per-seed flag sets
    seed_flags: dict[int, dict[int, set[tuple[str, int]]]] = {}
    for seed in seeds:
        reflagged = _reflag_records(pr_records, per_pr_thresholds, seed, n)
        seed_flags[seed] = {}
        for r in reflagged:
            if not r["is_test"] and r["flagged"]:
                pn = r["pr_number"]
                seed_flags[seed].setdefault(pn, set()).add((r["file_path"], r["hunk_index"]))
        for pn in pr_nums:
            seed_flags[seed].setdefault(pn, set())

    # Per-PR threshold stats
    pr_thr_stats: list[dict[str, Any]] = []
    max_rel_var = 0.0
    for pn in pr_nums:
        thr_vals = [
            per_pr_thresholds[pn][(s, n)]
            for s in seeds
            if not math.isnan(per_pr_thresholds[pn].get((s, n), float("nan")))
        ] if pn in per_pr_thresholds else []
        if not thr_vals:
            continue
        mn = min(thr_vals)
        mx = max(thr_vals)
        mean = sum(thr_vals) / len(thr_vals)
        variance = sum((v - mean) ** 2 for v in thr_vals) / len(thr_vals)
        stdev = math.sqrt(variance)
        rel_var = (mx - mn) / mean if mean > 0 else 0.0
        max_rel_var = max(max_rel_var, rel_var)
        pr_thr_stats.append({
            "pr_number": pn,
            "thresholds": thr_vals,
            "min": mn,
            "max": mx,
            "mean": mean,
            "stdev": stdev,
            "rel_var": rel_var,
        })

    # Per-PR flag set stability
    min_jaccard = 1.0
    pr_flag_stats: list[dict[str, Any]] = []
    for pn in pr_nums:
        flags_per_seed = [seed_flags[s][pn] for s in seeds]
        flags_union: set[tuple[str, int]] = set().union(*flags_per_seed)
        flags_stable = set.intersection(*flags_per_seed) if flags_per_seed else set()
        flags_unstable = flags_union - flags_stable

        jaccards = []
        for s in seeds[1:]:
            j = _jaccard(seed_flags[seeds[0]][pn], seed_flags[s][pn])
            jaccards.append(j)
            min_jaccard = min(min_jaccard, j)

        pr_flag_stats.append({
            "pr_number": pn,
            "stable": flags_stable,
            "unstable": flags_unstable,
            "union": flags_union,
            "jaccards": jaccards,
        })

    # Global counts
    all_stable = sum(len(s["stable"]) for s in pr_flag_stats)
    all_unstable = sum(len(s["unstable"]) for s in pr_flag_stats)
    all_union = sum(len(s["union"]) for s in pr_flag_stats)

    return {
        "pr_thr_stats": pr_thr_stats,
        "pr_flag_stats": pr_flag_stats,
        "max_rel_var": max_rel_var,
        "min_jaccard": min_jaccard,
        "all_stable_flags": all_stable,
        "all_unstable_flags": all_unstable,
        "all_union_flags": all_union,
        "gate_threshold_fails": max_rel_var > _THRESH_VAR_GATE,
        "gate_jaccard_fails": min_jaccard < _JACCARD_GATE,
    }


def _write_report(
    base_metrics: dict[str, Any],
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]],
    sweep_metrics: dict[int, dict[str, Any]] | None,
    pr_records: list[dict[str, Any]],
    seeds: list[int],
) -> None:
    is_stable = not (base_metrics["gate_threshold_fails"] or base_metrics["gate_jaccard_fails"])
    pr_nums = sorted({r["pr_number"] for r in pr_records if not r["is_test"]})

    lines: list[str] = [
        "# Phase 14 Prompt R — Seed Stability",
        "",
        "**Date:** 2026-04-22  ",
        "**Branch:** research/phase-14-import-graph  ",
        "**Why:** All experiments use seed=0, N=100. If different seeds produce materially",
        "different thresholds or flag sets, calibration is under-powered and prior results are wobbly.",
        "",
        "---",
        "",
        "## §0 Headline",
        "",
    ]

    if is_stable:
        lines += [
            "**N=100 is STABLE.** Both stability gates pass across all 5 seeds:",
            f"- Max relative threshold variance: {base_metrics['max_rel_var']:.2%} (gate: >{_THRESH_VAR_GATE:.0%})",
            f"- Min Jaccard similarity (seed-0 vs others): {base_metrics['min_jaccard']:.2%} (gate: <{_JACCARD_GATE:.0%})",
            "",
            "Prior results at N=100 / seed=0 stand. No re-validation required.",
        ]
    else:
        rec_n = "N=100"
        if sweep_metrics:
            for n_val in sorted(sweep_metrics.keys()):
                sm = sweep_metrics[n_val]
                if not sm["gate_threshold_fails"] and not sm["gate_jaccard_fails"]:
                    rec_n = f"N={n_val}"
                    break
            else:
                rec_n = f"N={max(sweep_metrics.keys())} (still not fully stable)"

        lines += [
            f"**N=100 is UNSTABLE.** Recommended N: **{rec_n}**",
            "",
            "Gate failures at N=100:",
        ]
        if base_metrics["gate_threshold_fails"]:
            lines.append(
                f"- Threshold gate: max relative variance = {base_metrics['max_rel_var']:.2%} > {_THRESH_VAR_GATE:.0%}"
            )
        if base_metrics["gate_jaccard_fails"]:
            lines.append(
                f"- Jaccard gate: min Jaccard = {base_metrics['min_jaccard']:.2%} < {_JACCARD_GATE:.0%}"
            )

    lines += ["", "---", "", "## §1 Per-seed Threshold Table (PRs × 5 seeds)", ""]

    # Build header
    seed_hdrs = " | ".join(f"seed{s}" for s in seeds)
    sep = " | ".join(["---"] * (len(seeds) + 2))
    lines.append(f"| PR# | {seed_hdrs} | rel_var |")
    lines.append(f"| {sep} |")

    thr_stats_by_pr = {s["pr_number"]: s for s in base_metrics["pr_thr_stats"]}
    for pn in pr_nums:
        stat = thr_stats_by_pr.get(pn)
        if stat is None:
            continue
        thr_cells = " | ".join(f"{t:.4f}" for t in stat["thresholds"])
        rel_var = f"{stat['rel_var']:.2%}"
        lines.append(f"| {pn} | {thr_cells} | {rel_var} |")

    lines += [
        "",
        "---",
        "",
        "## §2 Per-PR Relative Variance Distribution",
        "",
    ]

    rel_vars = [s["rel_var"] for s in base_metrics["pr_thr_stats"]]
    if rel_vars:
        sorted_rv = sorted(rel_vars)
        n_rv = len(sorted_rv)
        lines += [
            f"- PRs analysed: {n_rv}",
            f"- Max rel_var: {max(rel_vars):.2%}",
            f"- Median rel_var: {sorted_rv[n_rv // 2]:.2%}",
            f"- PRs with rel_var > 10%: {sum(1 for v in rel_vars if v > 0.10)}",
            f"- PRs with rel_var > 5%: {sum(1 for v in rel_vars if v > 0.05)}",
            f"- PRs with rel_var = 0%: {sum(1 for v in rel_vars if v == 0.0)}",
            "",
            "Distribution (buckets):",
            "",
            "| rel_var range | count |",
            "|---|---|",
        ]
        buckets = [(0.0, 0.01), (0.01, 0.02), (0.02, 0.05), (0.05, 0.10), (0.10, 0.20), (0.20, 1.0)]
        for lo, hi in buckets:
            cnt = sum(1 for v in rel_vars if lo <= v < hi)
            lines.append(f"| {lo:.0%}–{hi:.0%} | {cnt} |")
        lines.append("")

    lines += [
        "---",
        "",
        "## §3 Flag Set Diff: Stable / Unstable / Seed-unique",
        "",
        f"- Total stable flag pairs (all 5 seeds agree): {base_metrics['all_stable_flags']}",
        f"- Total unstable flag pairs (some seeds disagree): {base_metrics['all_unstable_flags']}",
        f"- Total unique flags across all seeds (union): {base_metrics['all_union_flags']}",
        f"- Min pairwise Jaccard (seed-0 vs others): {base_metrics['min_jaccard']:.2%}",
        "",
    ]

    # Jaccard table per seed
    lines += [
        "### Pairwise Jaccard (seed-0 vs seed-k)",
        "",
        "| seed | global Jaccard (over all flag pairs) |",
        "|---|---|",
    ]
    for si, s in enumerate(seeds[1:], 1):
        # Aggregate Jaccard: |A∩B| / |A∪B| over pooled flag sets
        a_all: set[tuple[int, str, int]] = set()
        b_all: set[tuple[int, str, int]] = set()
        for fs in base_metrics["pr_flag_stats"]:
            pn = fs["pr_number"]
            # Seed 0 flags for this PR
            for pair in fs["stable"] | fs["unstable"]:
                pass  # can't reconstruct per-seed from stable/unstable alone
        # Recompute global Jaccard from pr_flag_stats jaccards
        all_j = [fs["jaccards"][si - 1] for fs in base_metrics["pr_flag_stats"]]
        mean_j = sum(all_j) / len(all_j) if all_j else 1.0
        min_j = min(all_j) if all_j else 1.0
        lines.append(f"| seed-{s} | mean={mean_j:.2%}, min={min_j:.2%} |")

    lines += [
        "",
        "### PRs with unstable flags",
        "",
        "| PR# | stable | unstable | union | min_jaccard |",
        "|---|---|---|---|---|",
    ]
    unstable_prs = [
        fs for fs in base_metrics["pr_flag_stats"] if fs["unstable"]
    ]
    if unstable_prs:
        for fs in unstable_prs:
            min_j = min(fs["jaccards"]) if fs["jaccards"] else 1.0
            lines.append(
                f"| {fs['pr_number']} | {len(fs['stable'])} "
                f"| {len(fs['unstable'])} | {len(fs['union'])} | {min_j:.2%} |"
            )
    else:
        lines.append("| — | — | — | — | — |")

    lines += [""]

    # §4 N-sweep (if ran)
    if sweep_metrics:
        lines += [
            "---",
            "",
            "## §4 N-sweep Stability vs N",
            "",
            "| N | max_rel_var | min_jaccard | thresh_gate | jaccard_gate | stable? |",
            "|---|---|---|---|---|---|",
        ]
        # Include N=100 as baseline
        m100 = base_metrics
        lines.append(
            f"| 100 | {m100['max_rel_var']:.2%} | {m100['min_jaccard']:.2%}"
            f" | {'FAIL' if m100['gate_threshold_fails'] else 'pass'}"
            f" | {'FAIL' if m100['gate_jaccard_fails'] else 'pass'}"
            f" | {'NO' if (m100['gate_threshold_fails'] or m100['gate_jaccard_fails']) else 'YES'} |"
        )
        for n_val in sorted(sweep_metrics.keys()):
            sm = sweep_metrics[n_val]
            stable = not (sm["gate_threshold_fails"] or sm["gate_jaccard_fails"])
            lines.append(
                f"| {n_val} | {sm['max_rel_var']:.2%} | {sm['min_jaccard']:.2%}"
                f" | {'FAIL' if sm['gate_threshold_fails'] else 'pass'}"
                f" | {'FAIL' if sm['gate_jaccard_fails'] else 'pass'}"
                f" | {'YES' if stable else 'NO'} |"
            )
        lines += [""]

        # Recommended N
        rec_n_int: int | None = None
        for n_val in sorted(sweep_metrics.keys()):
            sm = sweep_metrics[n_val]
            if not sm["gate_threshold_fails"] and not sm["gate_jaccard_fails"]:
                rec_n_int = n_val
                break
        if rec_n_int is not None:
            lines += [f"**Recommended N: {rec_n_int}**", ""]
        else:
            lines += ["**No fully stable N found in sweep.** Use largest N and accept residual variance.", ""]

    lines += [
        "---",
        "",
        "## §5 Implication",
        "",
    ]

    if is_stable:
        lines += [
            "N=100 / seed=0 is stable. All prior fix7 experiments (FastAPI, Rich, threshold sweep)",
            "used the same configuration and produce results that would not materially change under",
            "a different seed. No re-validation is required.",
            "",
            "The `_CAL_SEED = 0` and `_N_CAL = 100` defaults are confirmed sound.",
        ]
    else:
        lines += [
            "N=100 is seed-sensitive. The following prior experiments used N=100 / seed=0 and",
            "should be re-validated at the recommended N:",
            "",
            "- `score_pr_hunks_fix7_2026_04_22.py` (FastAPI base rate)",
            "- `score_pr_hunks_fix7_rich_2026_04_22.py` (Rich cross-corpus)",
            "- `threshold_sweep_2026_04_22.py` (threshold comparison)",
            "",
            "Update `_N_CAL` in each script and re-run before proceeding to the PR campaign.",
        ]

    lines += [""]

    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {_DOCS_OUT}", flush=True)


def main() -> None:
    print("Phase 14 Prompt R — Seed Stability", flush=True)
    print("Loading shared tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped,unused-ignore]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    print("Tokenizer loaded.", flush=True)

    # Load PRs
    prs: list[dict[str, Any]] = []
    with _PRS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"Loaded {len(prs)} PRs", flush=True)

    # Step 1: Score all PRs, collecting thresholds for N=100 + sweep Ns
    all_n_values = [_N_BASE] + _N_SWEEP
    print(f"\n=== Step 1: Scoring {len(prs)} PRs (N values: {all_n_values}) ===", flush=True)
    exp = _run_experiment(tokenizer, prs, all_n_values)
    pr_records = exp["pr_records"]
    per_pr_thresholds = exp["per_pr_thresholds"]

    # Step 2: Write per-seed JONSLs (N=100)
    print("\n=== Step 2: Writing per-seed JONSLs ===", flush=True)
    for seed in _SEEDS:
        reflagged = _reflag_records(pr_records, per_pr_thresholds, seed, _N_BASE)
        out_path = _SCRIPT_DIR / f"real_pr_base_rate_hunks_fix7_seed{seed}_fastapi.jsonl"
        with out_path.open("w", encoding="utf-8") as fh:
            for r in reflagged:
                fh.write(json.dumps(r) + "\n")
        n_src = sum(1 for r in reflagged if not r["is_test"])
        n_flagged = sum(1 for r in reflagged if not r["is_test"] and r["flagged"])
        print(
            f"  seed={seed}: {n_src} src hunks, {n_flagged} flagged"
            f" ({n_flagged / n_src:.1%}) → {out_path.name}",
            flush=True,
        )

    # Step 3: Compute stability metrics at N=100
    print("\n=== Step 3: Stability metrics (N=100) ===", flush=True)
    base_metrics = _compute_stability_metrics(pr_records, per_pr_thresholds, _SEEDS, _N_BASE)
    print(f"  Max rel_var: {base_metrics['max_rel_var']:.2%}", flush=True)
    print(f"  Min Jaccard: {base_metrics['min_jaccard']:.2%}", flush=True)
    print(f"  Threshold gate {'FAIL' if base_metrics['gate_threshold_fails'] else 'PASS'}", flush=True)
    print(f"  Jaccard gate  {'FAIL' if base_metrics['gate_jaccard_fails'] else 'PASS'}", flush=True)

    # Step 4: N-sweep if needed
    sweep_metrics: dict[int, dict[str, Any]] | None = None
    if base_metrics["gate_threshold_fails"] or base_metrics["gate_jaccard_fails"]:
        print("\n=== Step 4: N-sweep ===", flush=True)
        sweep_metrics = {}
        for n_val in _N_SWEEP:
            print(f"  N={n_val}:", flush=True)
            sm = _compute_stability_metrics(pr_records, per_pr_thresholds, _SEEDS, n_val)
            sweep_metrics[n_val] = sm
            print(f"    Max rel_var: {sm['max_rel_var']:.2%}", flush=True)
            print(f"    Min Jaccard: {sm['min_jaccard']:.2%}", flush=True)
            print(
                f"    Threshold gate {'FAIL' if sm['gate_threshold_fails'] else 'PASS'}  "
                f"Jaccard gate {'FAIL' if sm['gate_jaccard_fails'] else 'PASS'}",
                flush=True,
            )
    else:
        print("  Gates pass — skipping N-sweep.", flush=True)

    # Step 5: Write report
    print("\n=== Step 5: Writing report ===", flush=True)
    _write_report(base_metrics, per_pr_thresholds, sweep_metrics, pr_records, _SEEDS)


if __name__ == "__main__":
    main()
