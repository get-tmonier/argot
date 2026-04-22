# engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix6_2026_04_22.py
"""Phase 14 Exp #7 Step 9 — Score PR hunks with fix6 (prose masking).

Reads:  real_pr_base_rate_prs_with_sha_2026_04_22.jsonl   (cached, with mergeCommit SHAs)
Writes: real_pr_base_rate_hunks_fix6_2026_04_22.jsonl

Builds on fix5 (per-PR recalibration) and adds prose masking: docstrings and
comments are blanked before BPE scoring in both calibration and scoring paths,
so the BPE signal reflects code tokens only.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix6_2026_04_22.py
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix6_2026_04_22.py --sanity-check
"""

from __future__ import annotations

import argparse
import io
import json
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
    SequentialImportBpeScorer,
)

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"

_SCRIPT_DIR = Path(__file__).parent
_PRS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_prs_with_sha_2026_04_22.jsonl"
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix6_2026_04_22.jsonl"

_FASTAPI_REPO = _REPOS_DIR / "fastapi"
_REPO_GH = "tiangolo/fastapi"
_N_CAL = 100
_CAL_SEED = 0

# In-process cache for git show results: (sha, path) -> content or None
_git_show_cache: dict[tuple[str, str], str | None] = {}


def _git_show(repo: Path, sha: str, path: str) -> str | None:
    """Return file content at <sha>:<path>, or None if path doesn't exist at that commit."""
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
    """Parse unified diff into per-hunk records with diff content."""
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
            current_file = None  # deleted file — skip
        elif line.startswith("@@ ") and current_file is not None:
            _flush_hunk()
            hunk_lines = []
            m = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
            if m:
                new_start = int(m.group(1))
                new_count = int(m.group(2)) if m.group(2) is not None else 1
                if new_count > 0:  # skip pure deletions
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


def _extract_hunk(file_path: Path, start_line: int, end_line: int) -> str | None:
    """Extract just the hunk lines from start_line to end_line (1-based, inclusive).

    Note: Not called in the main fix5 pipeline (replaced by inline merge-commit extraction).
    Retained for compatibility / potential reuse.
    """
    if not file_path.exists():
        return None
    lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _get_foreign_modules(hunk_source: str, repo_modules: frozenset[str]) -> list[str]:
    return sorted(_imports_from_ast(hunk_source) - repo_modules)


def main(sanity_check: bool = False) -> None:
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    print("Loading shared tokenizer...", flush=True)
    shared_tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    print("Tokenizer loaded.", flush=True)

    # Load PR list (includes mergeCommit SHAs)
    prs: list[dict[str, Any]] = []
    with _PRS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"Loaded {len(prs)} PRs", flush=True)

    if sanity_check:
        target = next((p for p in prs if p["number"] == 14564), None)
        if target is None:
            print("ERROR: PR #14564 not found in JSONL", flush=True)
            return
        prs = [target]
        print("SANITY CHECK MODE — running PR #14564 only", flush=True)

    all_records: list[dict[str, Any]] = []
    n_diffs_failed = 0
    n_files_missing = 0
    per_pr_thresholds: list[float] = []

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]
        pre_sha_ref = f"{merge_sha}^1"

        # Resolve pre_sha to full SHA
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

        # Per-PR calibration: extract repo state at pre_sha via git archive
        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)

                archive_proc = subprocess.run(
                    ["git", "-C", str(_FASTAPI_REPO), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    print(
                        f"  WARN: git archive failed for {pre_sha[:8]}, skipping",
                        flush=True,
                    )
                    n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                py_files = _collect_source_files(tmppath)
                if not py_files:
                    print(
                        f"  WARN: no .py files in archive for pre_sha {pre_sha[:8]}, skipping",
                        flush=True,
                    )
                    n_diffs_failed += 1
                    continue

                cal_hunks = sample_hunks(tmppath, _N_CAL, _CAL_SEED)
                n_cal_actual = len(cal_hunks)

                scorer = SequentialImportBpeScorer(
                    model_a_files=py_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks,
                    _tokenizer=shared_tokenizer,
                )

                cal_threshold = scorer.bpe_threshold
                cal_n_source_files = len(py_files)
                repo_modules: frozenset[str] = scorer._import_scorer._repo_modules

        except Exception as exc:
            print(f"  WARN: calibration failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        per_pr_thresholds.append(cal_threshold)

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
            print(f"    WARN diff failed: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        if not diff_text.strip():
            print("    WARN empty diff", flush=True)
            continue

        all_hunks = _parse_diff_hunks(diff_text)
        source_hunks = [h for h in all_hunks if _is_source_hunk(h["file"])]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        def _score_hunks(
            hunks: list[dict[str, Any]],
            is_test: bool,
            pr_rec: dict[str, Any],
        ) -> tuple[list[dict[str, Any]], int]:
            records: list[dict[str, Any]] = []
            n_missing = 0
            for hi, hunk in enumerate(hunks):
                # fix4: read file content at the PR's merge commit, not current HEAD
                pr_merge_sha = pr_rec["mergeCommit"]["oid"]
                file_content = _git_show(_FASTAPI_REPO, pr_merge_sha, hunk["file"])
                if file_content is None:
                    n_missing += 1
                    continue

                lines = file_content.splitlines()
                lo = max(0, hunk["start_line"] - 1)
                hi_idx = min(len(lines), hunk["end_line"])
                hunk_content = "\n".join(lines[lo:hi_idx])
                file_text = file_content  # full file at merge-commit, for Stage 1

                if not hunk_content.strip():
                    continue

                scored = scorer.score_hunk(
                    hunk_content,
                    file_source=file_text,
                    hunk_start_line=hunk["start_line"],
                    hunk_end_line=hunk["end_line"],
                )
                foreign = (
                    _get_foreign_modules(hunk_content, repo_modules)
                    if scored["reason"] == "import"
                    else []
                )
                records.append(
                    {
                        "pr_number": pr_rec["number"],
                        "pr_title": pr_rec["title"],
                        "pr_mergedAt": pr_rec["mergedAt"],
                        "pr_url": pr_rec["url"],
                        "pr_merge_sha": pr_merge_sha,
                        "file_path": hunk["file"],
                        "hunk_index": hi,
                        "hunk_start_line": hunk["start_line"],
                        "hunk_end_line": hunk["end_line"],
                        "is_test": is_test,
                        "bpe_threshold": cal_threshold,
                        "foreign_modules": foreign,
                        "diff_content": hunk.get("diff_content", "")[:1000],
                        "cal_threshold": cal_threshold,
                        "cal_n_source_files": cal_n_source_files,
                        "cal_n_hunks_sampled": n_cal_actual,
                        "pre_pr_sha": pre_sha,
                        **scored,
                    }
                )
            return records, n_missing

        src_records, src_missing = _score_hunks(source_hunks, is_test=False, pr_rec=pr)
        tst_records, tst_missing = _score_hunks(test_hunks, is_test=True, pr_rec=pr)
        all_records.extend(src_records)
        all_records.extend(tst_records)
        n_files_missing += src_missing + tst_missing

        n_src_hunks = len(src_records)
        n_flagged = sum(1 for r in src_records if r["flagged"])
        n_st1 = sum(1 for r in src_records if r.get("reason") == "import")
        n_st2 = sum(1 for r in src_records if r.get("reason") == "bpe")

        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  {pr['mergedAt'][:10]}  "
            f"pre={pre_sha[:7]}  cal_thr={cal_threshold:.4f}  "
            f"src={n_src_hunks}  flagged={n_flagged}  st1={n_st1}  st2={n_st2}",
            flush=True,
        )

        if sanity_check and src_records:
            bpe_scores_first3 = [
                r.get("bpe_score") for r in src_records[:3] if r.get("bpe_score") is not None
            ]
            print("\n--- SANITY CHECK DETAIL ---", flush=True)
            print(f"cal_threshold: {cal_threshold:.6f}", flush=True)
            print(f"Source hunks flagged: {n_flagged}", flush=True)
            print(f"First 3 BPE scores: {bpe_scores_first3}", flush=True)
            print("--- END SANITY CHECK ---\n", flush=True)

    if sanity_check:
        print("Sanity check complete — no output written.", flush=True)
        return

    # Write output
    with _HUNKS_JSONL.open("w", encoding="utf-8") as fh:
        for rec in all_records:
            fh.write(json.dumps(rec) + "\n")

    src_total = sum(1 for r in all_records if not r["is_test"])
    src_flagged = sum(1 for r in all_records if not r["is_test"] and r["flagged"])
    print(f"\nDiffs failed: {n_diffs_failed}", flush=True)
    print(f"Files missing at merge commits: {n_files_missing}", flush=True)
    print(f"Source hunks scored: {src_total}", flush=True)
    print(
        f"Source hunks flagged: {src_flagged} ({src_flagged / src_total:.1%})"
        if src_total
        else "Source hunks flagged: 0",
        flush=True,
    )

    # PR-level flag summary
    pr_flags: dict[int, int] = {}
    pr_totals: dict[int, int] = {}
    for r in all_records:
        if not r["is_test"]:
            pn = r["pr_number"]
            pr_totals[pn] = pr_totals.get(pn, 0) + 1
            if r["flagged"]:
                pr_flags[pn] = pr_flags.get(pn, 0) + 1
    prs_with_flags = sum(1 for pn, cnt in pr_flags.items() if cnt > 0)
    print(f"PRs with ≥1 source flag: {prs_with_flags}/{len(pr_totals)}", flush=True)

    # Stage breakdown
    st1 = sum(1 for r in all_records if not r["is_test"] and r.get("reason") == "import")
    st2 = sum(1 for r in all_records if not r["is_test"] and r.get("reason") == "bpe")
    print(f"  Stage 1 (import): {st1}", flush=True)
    print(f"  Stage 2 (bpe):    {st2}", flush=True)

    # Flagged files
    flagged_files: dict[str, int] = {}
    for r in all_records:
        if not r["is_test"] and r["flagged"]:
            fp = r["file_path"]
            flagged_files[fp] = flagged_files.get(fp, 0) + 1
    if flagged_files:
        print("Flagged files:", flush=True)
        for fp, cnt in sorted(flagged_files.items(), key=lambda x: -x[1]):
            print(f"  {fp}: {cnt}", flush=True)

    # BPE score distribution on source hunks
    bpe_scores = [
        r.get("bpe_score", 0.0)
        for r in all_records
        if not r["is_test"] and r.get("bpe_score") is not None
    ]
    if bpe_scores:
        bpe_scores_sorted = sorted(bpe_scores, reverse=True)
        print(f"\nBPE score distribution (source hunks, n={len(bpe_scores)}):", flush=True)
        print(f"  max={bpe_scores_sorted[0]:.4f}", flush=True)
        print(f"  p99={bpe_scores_sorted[int(len(bpe_scores_sorted) * 0.01)]:.4f}", flush=True)
        print(f"  p95={bpe_scores_sorted[int(len(bpe_scores_sorted) * 0.05)]:.4f}", flush=True)
        print(f"  median={bpe_scores_sorted[len(bpe_scores_sorted) // 2]:.4f}", flush=True)

    # Per-PR threshold distribution
    if per_pr_thresholds:
        thr_sorted = sorted(per_pr_thresholds)
        n_thr = len(thr_sorted)
        thr_median = thr_sorted[n_thr // 2]
        thr_p90 = thr_sorted[int(n_thr * 0.90)]
        print(f"\nPer-PR threshold distribution (n={n_thr}):", flush=True)
        print(f"  min={thr_sorted[0]:.4f}", flush=True)
        print(f"  median={thr_median:.4f}", flush=True)
        print(f"  p90={thr_p90:.4f}", flush=True)
        print(f"  max={thr_sorted[-1]:.4f}", flush=True)

    print(f"\nWritten → {_HUNKS_JSONL}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--sanity-check",
        action="store_true",
        help="Run on PR #14564 only and print calibration threshold, flagged count, and first 3 BPE scores",
    )
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
