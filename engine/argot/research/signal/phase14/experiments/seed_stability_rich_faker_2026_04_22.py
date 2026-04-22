# engine/argot/research/signal/phase14/experiments/seed_stability_rich_faker_2026_04_22.py
"""Phase 14 Task #2 — Seed Stability probe for Rich and Faker corpora.

Runs a 5-seed probe at each corpus's fix9 N (Rich N=230, Faker N=250) to verify
that calibration thresholds and flag sets are reproducible.  If a corpus fails the
stability gates, an N-sweep is run above the current N to find the stable point.

Gates (same as FastAPI probe in seed_stability_2026_04_22.py):
  - Threshold gate: max per-PR relative variance ≤ 10%
  - Jaccard gate: min pairwise Jaccard (seed-0 vs seed-k) ≥ 80%

Output:
  docs/research/scoring/signal/phase14/experiments/seed_stability_rich_faker_2026-04-22.md

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/seed_stability_rich_faker_2026_04_22.py
"""

from __future__ import annotations

import io
import json
import math
import re
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass, field
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

_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "seed_stability_rich_faker_2026-04-22.md"
)

# ---------------------------------------------------------------------------
# Corpus definitions
# ---------------------------------------------------------------------------

_SEEDS = [0, 1, 2, 3, 4]
_THRESH_VAR_GATE = 0.10
_JACCARD_GATE = 0.80


@dataclass
class CorpusConfig:
    name: str
    repo_dir: Path
    repo_gh: str
    prs_jsonl: Path
    n_base: int
    n_sweep: list[int]
    source_prefix: str  # hunk path prefix for source files (e.g. "rich/")


_CORPORA: list[CorpusConfig] = [
    CorpusConfig(
        name="rich",
        repo_dir=_REPOS_DIR / "rich",
        repo_gh="Textualize/rich",
        prs_jsonl=_SCRIPT_DIR / "rich_real_pr_base_rate_prs_2026_04_22.jsonl",
        n_base=230,
        # Rich ceiling is ~234-238 hunks; probe just above current N
        n_sweep=[232, 234, 238],
        source_prefix="rich/",
    ),
    CorpusConfig(
        name="faker",
        repo_dir=_REPOS_DIR / "faker",
        repo_gh="joke2k/faker",
        prs_jsonl=_SCRIPT_DIR / "faker_real_pr_base_rate_prs_2026_04_22.jsonl",
        n_base=250,
        n_sweep=[350, 500],
        source_prefix="faker/",
    ),
]

# ---------------------------------------------------------------------------
# Cache
# ---------------------------------------------------------------------------

_git_show_cache: dict[tuple[Path, str, str], str | None] = {}


def _git_show(repo: Path, sha: str, path: str) -> str | None:
    key = (repo, sha, path)
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


def _is_test_hunk(path: str) -> bool:
    return (
        path.startswith("tests/")
        or "test_" in path.split("/")[-1]
        or path.split("/")[-1].endswith("_test.py")
    ) and path.endswith(".py")


# ---------------------------------------------------------------------------
# Multi-seed threshold sweep (seed-independent BPE scores)
# ---------------------------------------------------------------------------


def _compute_multi_seed_thresholds(
    scorer: SequentialImportBpeScorer,
    candidates: list[str],
    seeds: list[int],
    n_values: list[int],
) -> dict[tuple[int, int], float]:
    """Return {(seed, n): threshold} for all seed × n combinations."""
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


# ---------------------------------------------------------------------------
# Jaccard helpers
# ---------------------------------------------------------------------------


def _jaccard(a: set[Any], b: set[Any]) -> float:
    if not a and not b:
        return 1.0
    union = a | b
    if not union:
        return 1.0
    return len(a & b) / len(union)


# ---------------------------------------------------------------------------
# Per-corpus scoring run
# ---------------------------------------------------------------------------


@dataclass
class CorpusResult:
    name: str
    n_base: int
    n_sweep: list[int]
    pr_records: list[dict[str, Any]] = field(default_factory=list)
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]] = field(default_factory=dict)
    n_diffs_failed: int = 0


def _run_corpus(
    cfg: CorpusConfig,
    tokenizer: Any,
    prs: list[dict[str, Any]],
) -> CorpusResult:
    all_n_values = [cfg.n_base] + cfg.n_sweep
    result = CorpusResult(name=cfg.name, n_base=cfg.n_base, n_sweep=cfg.n_sweep)

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]
        pre_sha_ref = f"{merge_sha}^1"

        try:
            rev = subprocess.run(
                ["git", "-C", str(cfg.repo_dir), "rev-parse", pre_sha_ref],
                capture_output=True,
                text=True,
                check=True,
                timeout=30,
            )
            pre_sha = rev.stdout.strip()
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"  WARN rev-parse failed for {pre_sha_ref}: {exc}", flush=True)
            result.n_diffs_failed += 1
            continue

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)

                archive_proc = subprocess.run(
                    ["git", "-C", str(cfg.repo_dir), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    print(f"  WARN git archive failed for {pre_sha[:8]}, skipping", flush=True)
                    result.n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                py_files = _collect_source_files(tmppath)
                if not py_files:
                    result.n_diffs_failed += 1
                    continue

                cal_hunks_seed0 = sample_hunks(tmppath, cfg.n_base, 0)
                scorer = SequentialImportBpeScorer(
                    model_a_files=py_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks_seed0,
                    _tokenizer=tokenizer,
                    exclude_data_dominant=True,
                )
                cal_threshold_seed0 = scorer.bpe_threshold
                n_cal_actual = len(cal_hunks_seed0)
                cal_n_source_files = len(py_files)
                repo_modules: frozenset[str] = scorer._import_scorer._repo_modules

                candidates = collect_candidates(tmppath)
                thresholds = _compute_multi_seed_thresholds(scorer, candidates, _SEEDS, all_n_values)

        except Exception as exc:
            print(f"  WARN calibration failed for PR #{pr_num}: {exc}", flush=True)
            result.n_diffs_failed += 1
            continue

        result.per_pr_thresholds[pr_num] = thresholds

        try:
            diff_result = subprocess.run(
                ["gh", "pr", "diff", str(pr_num), "--repo", cfg.repo_gh],
                capture_output=True,
                text=True,
                check=True,
                timeout=60,
            )
            diff_text = diff_result.stdout
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"    WARN diff failed for #{pr_num}: {exc}", flush=True)
            result.n_diffs_failed += 1
            continue

        if not diff_text.strip():
            continue

        all_hunks = _parse_diff_hunks(diff_text)
        source_hunks = [h for h in all_hunks if h["file"].startswith(cfg.source_prefix) and h["file"].endswith(".py")]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        for is_test, hunks in [(False, source_hunks), (True, test_hunks)]:
            for hi, hunk in enumerate(hunks):
                file_content = _git_show(cfg.repo_dir, merge_sha, hunk["file"])
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
                result.pr_records.append({
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

        n_src = sum(1 for r in result.pr_records if r["pr_number"] == pr_num and not r["is_test"])
        n_flagged = sum(
            1 for r in result.pr_records
            if r["pr_number"] == pr_num and not r["is_test"] and r["flagged"]
        )
        thr_vals = [
            v for (s, n), v in thresholds.items()
            if n == cfg.n_base and not math.isnan(v)
        ]
        thr_range = (
            f"{min(thr_vals):.4f}–{max(thr_vals):.4f}" if len(thr_vals) > 1
            else f"{cal_threshold_seed0:.4f}"
        )
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  {pr['mergedAt'][:10]}  "
            f"pre={pre_sha[:7]}  thr_range={thr_range}  "
            f"src={n_src}  flagged={n_flagged}",
            flush=True,
        )

    return result


# ---------------------------------------------------------------------------
# Stability metrics
# ---------------------------------------------------------------------------


def _reflag_records(
    records: list[dict[str, Any]],
    per_pr_thresholds: dict[int, dict[tuple[int, int], float]],
    seed: int,
    n: int,
) -> list[dict[str, Any]]:
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
    pr_nums = sorted({r["pr_number"] for r in pr_records if not r["is_test"]})

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

    pr_thr_stats: list[dict[str, Any]] = []
    max_rel_var = 0.0
    for pn in pr_nums:
        thr_vals = [
            per_pr_thresholds[pn][(s, n)]
            for s in seeds
            if pn in per_pr_thresholds and not math.isnan(per_pr_thresholds[pn].get((s, n), float("nan")))
        ] if pn in per_pr_thresholds else []
        if not thr_vals:
            continue
        mn = min(thr_vals)
        mx = max(thr_vals)
        mean = sum(thr_vals) / len(thr_vals)
        rel_var = (mx - mn) / mean if mean > 0 else 0.0
        max_rel_var = max(max_rel_var, rel_var)
        pr_thr_stats.append({
            "pr_number": pn,
            "thresholds": thr_vals,
            "min": mn,
            "max": mx,
            "mean": mean,
            "rel_var": rel_var,
        })

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


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------


def _format_corpus_section(
    cfg: CorpusConfig,
    base_metrics: dict[str, Any],
    sweep_metrics: dict[int, dict[str, Any]] | None,
    pr_records: list[dict[str, Any]],
    seeds: list[int],
    section_offset: int,
) -> list[str]:
    """Return markdown lines for one corpus's sections (§1–§5 renumbered by offset)."""
    pr_nums = sorted({r["pr_number"] for r in pr_records if not r["is_test"]})
    is_stable = not (base_metrics["gate_threshold_fails"] or base_metrics["gate_jaccard_fails"])

    rec_n_label = f"N={cfg.n_base}"
    if not is_stable and sweep_metrics:
        for n_val in sorted(sweep_metrics.keys()):
            sm = sweep_metrics[n_val]
            if not sm["gate_threshold_fails"] and not sm["gate_jaccard_fails"]:
                rec_n_label = f"N={n_val}"
                break
        else:
            rec_n_label = f"N={max(sweep_metrics.keys())} (still not fully stable)"

    corpus_upper = cfg.name.upper()
    lines: list[str] = [
        f"## {corpus_upper} corpus",
        "",
    ]

    if is_stable:
        lines += [
            f"**N={cfg.n_base} is STABLE** — both gates pass across all 5 seeds.",
            f"- Max threshold rel_var: {base_metrics['max_rel_var']:.2%} (gate: ≤{_THRESH_VAR_GATE:.0%})",
            f"- Min pairwise Jaccard: {base_metrics['min_jaccard']:.2%} (gate: ≥{_JACCARD_GATE:.0%})",
        ]
    else:
        lines += [
            f"**N={cfg.n_base} is UNSTABLE** — gate failure(s):",
        ]
        if base_metrics["gate_threshold_fails"]:
            lines.append(
                f"- Threshold gate FAIL: max rel_var = {base_metrics['max_rel_var']:.2%} > {_THRESH_VAR_GATE:.0%}"
            )
        if base_metrics["gate_jaccard_fails"]:
            lines.append(
                f"- Jaccard gate FAIL: min Jaccard = {base_metrics['min_jaccard']:.2%} < {_JACCARD_GATE:.0%}"
            )
        if sweep_metrics:
            stable_found = any(
                not sm["gate_threshold_fails"] and not sm["gate_jaccard_fails"]
                for sm in sweep_metrics.values()
            )
            if stable_found:
                lines.append(f"- Recommended N: **{rec_n_label}**")
            else:
                lines.append(f"- No stable N found in sweep (tested up to N={max(sweep_metrics.keys())})")

    lines += [""]

    # §A: Per-seed threshold table
    s1 = section_offset + 1
    lines += [
        f"### §{s1} Per-seed Threshold Table",
        "",
        "| PR# | " + " | ".join(f"seed{s}" for s in seeds) + " | rel_var |",
        "| " + " | ".join(["---"] * (len(seeds) + 2)) + " |",
    ]
    thr_stats_by_pr = {s["pr_number"]: s for s in base_metrics["pr_thr_stats"]}
    for pn in pr_nums:
        stat = thr_stats_by_pr.get(pn)
        if stat is None:
            continue
        thr_cells = " | ".join(f"{t:.4f}" for t in stat["thresholds"])
        lines.append(f"| {pn} | {thr_cells} | {stat['rel_var']:.2%} |")
    lines.append("")

    # §B: rel_var distribution
    s2 = section_offset + 2
    rel_vars = [s["rel_var"] for s in base_metrics["pr_thr_stats"]]
    sorted_rv = sorted(rel_vars)
    n_rv = len(sorted_rv)
    lines += [
        f"### §{s2} Relative Variance Distribution",
        "",
        f"- PRs analysed: {n_rv}",
        f"- Max rel_var: {max(rel_vars):.2%}" if rel_vars else "- No PRs",
        f"- Median rel_var: {sorted_rv[n_rv // 2]:.2%}" if rel_vars else "",
        f"- PRs with rel_var > 10%: {sum(1 for v in rel_vars if v > 0.10)}",
        f"- PRs with rel_var > 5%: {sum(1 for v in rel_vars if v > 0.05)}",
        f"- PRs with rel_var = 0%: {sum(1 for v in rel_vars if v == 0.0)}",
        "",
        "| rel_var range | count |",
        "|---|---|",
    ]
    for lo, hi in [(0.0, 0.01), (0.01, 0.02), (0.02, 0.05), (0.05, 0.10), (0.10, 0.20), (0.20, 1.0)]:
        cnt = sum(1 for v in rel_vars if lo <= v < hi)
        lines.append(f"| {lo:.0%}–{hi:.0%} | {cnt} |")
    lines.append("")

    # §C: Flag set stability
    s3 = section_offset + 3
    lines += [
        f"### §{s3} Flag Set Stability",
        "",
        f"- Total stable flag pairs (all 5 seeds agree): {base_metrics['all_stable_flags']}",
        f"- Total unstable flag pairs (some seeds disagree): {base_metrics['all_unstable_flags']}",
        f"- Total unique flags across all seeds (union): {base_metrics['all_union_flags']}",
        f"- Min pairwise Jaccard (seed-0 vs others): {base_metrics['min_jaccard']:.2%}",
        "",
        "#### Pairwise Jaccard (seed-0 vs seed-k)",
        "",
        "| seed | mean Jaccard | min Jaccard |",
        "|---|---|---|",
    ]
    for si, s in enumerate(seeds[1:], 1):
        all_j = [fs["jaccards"][si - 1] for fs in base_metrics["pr_flag_stats"]]
        mean_j = sum(all_j) / len(all_j) if all_j else 1.0
        min_j = min(all_j) if all_j else 1.0
        lines.append(f"| seed-{s} | {mean_j:.2%} | {min_j:.2%} |")
    lines.append("")

    unstable_prs = [fs for fs in base_metrics["pr_flag_stats"] if fs["unstable"]]
    lines += [
        "#### PRs with unstable flags",
        "",
        "| PR# | stable | unstable | union | min_jaccard |",
        "|---|---|---|---|---|",
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
    lines.append("")

    # §D: N-sweep (if ran)
    s4 = section_offset + 4
    if sweep_metrics:
        lines += [
            f"### §{s4} N-sweep",
            "",
            "| N | max_rel_var | min_jaccard | thresh_gate | jaccard_gate | stable? |",
            "|---|---|---|---|---|---|",
        ]
        m0 = base_metrics
        lines.append(
            f"| {cfg.n_base} | {m0['max_rel_var']:.2%} | {m0['min_jaccard']:.2%}"
            f" | {'FAIL' if m0['gate_threshold_fails'] else 'pass'}"
            f" | {'FAIL' if m0['gate_jaccard_fails'] else 'pass'}"
            f" | {'NO' if (m0['gate_threshold_fails'] or m0['gate_jaccard_fails']) else 'YES'} |"
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
        lines.append("")

    return lines


def _write_report(
    rich_cfg: CorpusConfig,
    rich_base: dict[str, Any],
    rich_sweep: dict[int, dict[str, Any]] | None,
    rich_records: list[dict[str, Any]],
    faker_cfg: CorpusConfig,
    faker_base: dict[str, Any],
    faker_sweep: dict[int, dict[str, Any]] | None,
    faker_records: list[dict[str, Any]],
    seeds: list[int],
    faker_per_pr_thresholds: dict[int, dict[tuple[int, int], float]],
) -> None:
    rich_stable = not (rich_base["gate_threshold_fails"] or rich_base["gate_jaccard_fails"])
    faker_stable = not (faker_base["gate_threshold_fails"] or faker_base["gate_jaccard_fails"])

    def _rec_n(cfg: CorpusConfig, base: dict[str, Any], sweep: dict[int, dict[str, Any]] | None) -> str:
        if not (base["gate_threshold_fails"] or base["gate_jaccard_fails"]):
            return f"N={cfg.n_base} (stable)"
        if sweep:
            for n_val in sorted(sweep.keys()):
                sm = sweep[n_val]
                if not sm["gate_threshold_fails"] and not sm["gate_jaccard_fails"]:
                    return f"N={n_val}"
            return f"N={max(sweep.keys())} (unstable — intrinsic floor)"
        return f"N={cfg.n_base} (unstable, no sweep)"

    lines: list[str] = [
        "# Phase 14 Task #2 — Seed Stability: Rich and Faker",
        "",
        "**Date:** 2026-04-22  ",
        "**Branch:** research/phase-14-import-graph  ",
        "**Why:** post-fix9, Rich uses N=230 and Faker uses N=250. Only FastAPI has been",
        "directly probed for seed stability (N=500 confirmed stable). This probe verifies",
        "Rich and Faker at their fix9 N values.",
        "",
        "---",
        "",
        "## §0 Headline",
        "",
        f"- **RICH** N=230: {'STABLE' if rich_stable else 'UNSTABLE'} → recommended {_rec_n(rich_cfg, rich_base, rich_sweep)}",
        f"- **FAKER** N=250: {'STABLE' if faker_stable else 'UNSTABLE'} → recommended {_rec_n(faker_cfg, faker_base, faker_sweep)}",
        "",
        "---",
        "",
    ]

    # Rich sections (§1–§4 as A-D)
    rich_lines = _format_corpus_section(rich_cfg, rich_base, rich_sweep, rich_records, seeds, 0)
    # Re-number to §1, §2, §3, §4
    rich_out: list[str] = []
    sec_num = 1
    for line in rich_lines:
        if line.startswith("### §"):
            # Replace the offset-based number with actual §N
            line = re.sub(r"### §\d+", f"### §{sec_num}", line)
            sec_num += 1
        rich_out.append(line)

    lines += rich_out
    lines += ["---", ""]

    # Faker sections (§5–§8)
    faker_lines = _format_corpus_section(faker_cfg, faker_base, faker_sweep, faker_records, seeds, 4)
    faker_out: list[str] = []
    sec_num = 5
    for line in faker_lines:
        if line.startswith("### §"):
            line = re.sub(r"### §\d+", f"### §{sec_num}", line)
            sec_num += 1
        faker_out.append(line)

    lines += faker_out
    lines += ["---", ""]

    # §9 Faker borderline flag stability
    lines += [
        "## §9 Faker Borderline Flag Stability",
        "",
        "Checking whether the 4 borderline flags from fix9 (bank/__init__.py, pyfloat,",
        "address/en_GB, dataclass_migration) are stable across seeds.",
        "",
    ]

    borderline_keys = [
        "faker/providers/bank/__init__.py",
        "faker/providers/python/__init__.py",  # pyfloat lives here
        "faker/providers/address/en_GB/__init__.py",
        "tests/",  # dataclass_migration is in tests
    ]

    # Collect flags per seed for the specific borderline hunks
    borderline_table: list[dict[str, Any]] = []
    # Identify borderline hunk records from seed-0 base
    borderline_records = [
        r for r in faker_records
        if not r["is_test"] and any(
            r["file_path"].startswith(k.rstrip("/")) or r["file_path"] == k
            for k in borderline_keys[:3]
        )
    ]
    # Also dataclass_migration fixture (in tests)
    fixture_records = [
        r for r in faker_records
        if r["is_test"] and "dataclass" in r.get("file_path", "").lower()
    ]

    if borderline_records or fixture_records:
        lines += [
            "### Borderline source flags",
            "",
            "| file | hunk_idx | seed0_flagged | seed1_flagged | seed2_flagged | seed3_flagged | seed4_flagged | stable? |",
            "|---|---|---|---|---|---|---|---|",
        ]
        for r in borderline_records:
            pn = r["pr_number"]
            hi = r["hunk_index"]
            seed_flagged: list[str] = []
            for seed in seeds:
                thr = faker_per_pr_thresholds.get(pn, {}).get((seed, faker_cfg.n_base))
                if thr is None or math.isnan(thr):
                    seed_flagged.append("?")
                else:
                    reason = r.get("reason", "none")
                    if reason in ("import", "auto_generated"):
                        seed_flagged.append("Y(import)")
                    else:
                        bpe = r.get("bpe_score") or 0.0
                        seed_flagged.append("Y" if bpe > thr else "N")
            all_same = len(set(seed_flagged)) == 1
            stable_str = "YES" if all_same else "NO"
            lines.append(
                f"| {r['file_path']} | {hi} | "
                + " | ".join(seed_flagged)
                + f" | {stable_str} |"
            )
        lines.append("")

        if fixture_records:
            lines += [
                "### Fixture: dataclass_migration",
                "",
                "| file | hunk_idx | bpe_score | seed0_thr | seed0_margin | seed1_thr | seed2_thr | seed3_thr | seed4_thr | crosses_in_any_seed? |",
                "|---|---|---|---|---|---|---|---|---|---|",
            ]
            for r in fixture_records:
                pn = r["pr_number"]
                hi = r["hunk_index"]
                bpe = r.get("bpe_score") or 0.0
                thr_vals = [
                    faker_per_pr_thresholds.get(pn, {}).get((s, faker_cfg.n_base), float("nan"))
                    for s in seeds
                ]
                margin0 = bpe - thr_vals[0] if not math.isnan(thr_vals[0]) else float("nan")
                crosses = any(
                    not math.isnan(t) and bpe > t for t in thr_vals
                )
                thr_strs = " | ".join(f"{t:.4f}" if not math.isnan(t) else "NaN" for t in thr_vals)
                lines.append(
                    f"| {r['file_path']} | {hi} | {bpe:.4f} | {thr_vals[0]:.4f} | {margin0:.4f} | "
                    + " | ".join(f"{t:.4f}" if not math.isnan(t) else "NaN" for t in thr_vals[1:])
                    + f" | {'YES — FP risk' if crosses else 'NO — safe'} |"
                )
            lines.append("")
    else:
        lines += [
            "No borderline flag records matched. These may appear in different PRs or",
            "file paths — check §5 flag tables for manual cross-reference.",
            "",
        ]

    # §10 Implication
    lines += [
        "---",
        "",
        "## §10 Implication",
        "",
    ]

    if rich_stable and faker_stable:
        lines += [
            "Both corpora are stable at their current N. The fix9 base-rate results",
            "for Rich (N=230) and Faker (N=250) are reproducible under different seeds.",
            "No re-validation is required.",
            "",
            "The fix9 flag counts and thresholds reported in the fix9 validation docs",
            "are trustworthy reference points for V0.",
        ]
    else:
        if not rich_stable:
            rich_rec = _rec_n(rich_cfg, rich_base, rich_sweep)
            lines += [
                f"**RICH**: N=230 is seed-fragile. Recommended N: {rich_rec}.",
                "The fix9 Rich validation (2 Stage-1 flags, 0 Stage-2 flags) may be non-reproducible.",
                "If Rich N cannot be raised (ceiling constraint), document as a known instability",
                "and treat Rich results as approximate.",
                "",
            ]
        if not faker_stable:
            faker_rec = _rec_n(faker_cfg, faker_base, faker_sweep)
            lines += [
                f"**FAKER**: N=250 is seed-fragile. Recommended N: {faker_rec}.",
                "The borderline flags (bank/__init__.py +0.09, pyfloat +0.37, address/en_GB +0.41)",
                "may flip on/off across seeds. This is a real V0 concern: faker's 18-flag base rate",
                "is on a knife-edge if N=250 is unstable.",
                "",
            ]

    lines += [""]

    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {_DOCS_OUT}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Task #2 — Seed Stability: Rich and Faker", flush=True)
    print("Loading tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped,unused-ignore]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    print("Tokenizer loaded.", flush=True)

    corpus_results: dict[str, CorpusResult] = {}

    for cfg in _CORPORA:
        prs: list[dict[str, Any]] = []
        with cfg.prs_jsonl.open(encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line:
                    prs.append(json.loads(line))
        print(f"\n=== {cfg.name.upper()} ({len(prs)} PRs, N={cfg.n_base}) ===", flush=True)

        corpus_result = _run_corpus(cfg, tokenizer, prs)
        corpus_results[cfg.name] = corpus_result
        print(f"  diffs_failed={corpus_result.n_diffs_failed}", flush=True)

    # Compute stability metrics for each corpus
    print("\n=== Computing stability metrics ===", flush=True)
    sweep_results: dict[str, dict[int, dict[str, Any]] | None] = {}

    for cfg in _CORPORA:
        cr = corpus_results[cfg.name]
        all_n_values = [cfg.n_base] + cfg.n_sweep

        base_metrics = _compute_stability_metrics(
            cr.pr_records, cr.per_pr_thresholds, _SEEDS, cfg.n_base
        )
        print(
            f"  {cfg.name.upper()} N={cfg.n_base}: "
            f"max_rel_var={base_metrics['max_rel_var']:.2%}  "
            f"min_jaccard={base_metrics['min_jaccard']:.2%}  "
            f"thresh={'FAIL' if base_metrics['gate_threshold_fails'] else 'PASS'}  "
            f"jaccard={'FAIL' if base_metrics['gate_jaccard_fails'] else 'PASS'}",
            flush=True,
        )

        corpus_results[cfg.name].__dict__.setdefault("base_metrics", base_metrics)
        # Store base_metrics as attribute for report generation
        setattr(cr, "base_metrics", base_metrics)

        sweep_metrics: dict[int, dict[str, Any]] | None = None
        if base_metrics["gate_threshold_fails"] or base_metrics["gate_jaccard_fails"]:
            print(f"  Gates FAIL for {cfg.name.upper()} — running N-sweep", flush=True)
            sweep_metrics = {}
            for n_val in cfg.n_sweep:
                sm = _compute_stability_metrics(cr.pr_records, cr.per_pr_thresholds, _SEEDS, n_val)
                sweep_metrics[n_val] = sm
                stable = not (sm["gate_threshold_fails"] or sm["gate_jaccard_fails"])
                print(
                    f"    N={n_val}: max_rel_var={sm['max_rel_var']:.2%}  "
                    f"min_jaccard={sm['min_jaccard']:.2%}  "
                    f"{'STABLE' if stable else 'UNSTABLE'}",
                    flush=True,
                )
        else:
            print(f"  {cfg.name.upper()} gates PASS — skipping N-sweep", flush=True)

        sweep_results[cfg.name] = sweep_metrics

    # Write report
    print("\n=== Writing report ===", flush=True)
    rich_cr = corpus_results["rich"]
    faker_cr = corpus_results["faker"]
    rich_cfg = next(c for c in _CORPORA if c.name == "rich")
    faker_cfg = next(c for c in _CORPORA if c.name == "faker")

    _write_report(
        rich_cfg=rich_cfg,
        rich_base=rich_cr.base_metrics,  # type: ignore[attr-defined]
        rich_sweep=sweep_results["rich"],
        rich_records=rich_cr.pr_records,
        faker_cfg=faker_cfg,
        faker_base=faker_cr.base_metrics,  # type: ignore[attr-defined]
        faker_sweep=sweep_results["faker"],
        faker_records=faker_cr.pr_records,
        seeds=_SEEDS,
        faker_per_pr_thresholds=faker_cr.per_pr_thresholds,
    )


if __name__ == "__main__":
    main()
