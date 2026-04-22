# engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix4_2026_04_22.py
"""Phase 14 Exp #7 Step 6 — Score PR hunks with fix4 (merge-commit extraction).

Reads:  real_pr_base_rate_prs_with_sha_2026_04_22.jsonl   (cached, with mergeCommit SHAs)
Writes: real_pr_base_rate_hunks_fix4_2026_04_22.jsonl

Calibration: seed 0, n=100 hunks from .argot/research/repos/fastapi source.
Scoring:     SequentialImportBpeScorer with fix4 applied:
             - Stage 1 uses file_source to extract imports from full file context
               plus any imports in the hunk itself (extraction decoupled, as in fix3)
             - Stage 2 scores hunk_content only (not file-start-to-hunk-end)
Extraction:  hunk_content = lines [start_line..end_line] from file at merge commit
             file_source  = full file text at merge commit (via git show <sha>:<path>)

fix3 was INVALID because it read current HEAD for file content; line numbers from diff
headers refer to the file state at the PR's merge commit. fix4 corrects this by using
`git show <merge_sha>:<path>` to retrieve file content at the exact merge point.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix4_2026_04_22.py
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix4_2026_04_22.py --sanity-check
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
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
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix4_2026_04_22.jsonl"

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

    Note: Not called in the main fix4 pipeline (replaced by inline merge-commit extraction).
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
    # Load PR list (includes mergeCommit SHAs)
    prs: list[dict[str, Any]] = []
    with _PRS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"Loaded {len(prs)} PRs", flush=True)

    if sanity_check:
        target = next((p for p in prs if p["number"] == 14609), None)
        if target is None:
            print("ERROR: PR #14609 not found in JSONL", flush=True)
            return
        prs = [target]
        print("SANITY CHECK MODE — running PR #14609 only", flush=True)

    # Initialize scorer
    print("Initializing SequentialImportBpeScorer (seed=0, n_cal=100)...", flush=True)
    model_a_files = _collect_source_files(_FASTAPI_REPO)
    cal_hunks = sample_hunks(_FASTAPI_REPO, _N_CAL, _CAL_SEED)
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  model_a_files={len(model_a_files)}, n_cal={len(cal_hunks)}", flush=True)
    print(f"  bpe_threshold={scorer.bpe_threshold:.4f}", flush=True)

    repo_modules: frozenset[str] = scorer._import_scorer._repo_modules

    all_records: list[dict[str, Any]] = []
    n_diffs_failed = 0
    n_files_missing = 0

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  {pr['mergedAt'][:10]}  "
            f"sha={merge_sha[:12]}  {pr['title'][:50]}",
            flush=True,
        )

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

                scored = scorer.score_hunk(hunk_content, file_source=file_text)
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
                        "bpe_threshold": scorer.bpe_threshold,
                        "foreign_modules": foreign,
                        "diff_content": hunk.get("diff_content", "")[:1000],
                        **scored,
                    }
                )
            return records, n_missing

        src_records, src_missing = _score_hunks(source_hunks, is_test=False, pr_rec=pr)
        tst_records, tst_missing = _score_hunks(test_hunks, is_test=True, pr_rec=pr)
        all_records.extend(src_records)
        all_records.extend(tst_records)
        n_files_missing += src_missing + tst_missing

        n_flagged_src = sum(1 for r in src_records if r["flagged"])
        print(
            f"    source={len(src_records)} hunks, {n_flagged_src} flagged  "
            f"| test={len(tst_records)} hunks",
            flush=True,
        )

        if sanity_check and src_records:
            # Extra sanity output for PR #14609
            flagged_src = [r for r in src_records if r["flagged"]]
            all_src = src_records
            print("\n--- SANITY CHECK DETAIL ---", flush=True)
            print(f"Source hunks scored: {len(all_src)}", flush=True)
            print(f"Source hunks flagged: {len(flagged_src)}", flush=True)
            # Find hunks around lines 431-437 in routing.py
            routing_hunks = [
                r for r in all_src if "routing.py" in r["file_path"]
            ]
            print(f"routing.py hunks: {len(routing_hunks)}", flush=True)
            for rh in routing_hunks[:5]:
                print(
                    f"  lines {rh['hunk_start_line']}-{rh['hunk_end_line']}  "
                    f"bpe={rh.get('bpe_score', 'N/A'):.6f}  flagged={rh['flagged']}",
                    flush=True,
                )
            # Find the hunk at lines 431-437
            target_hunk = next(
                (
                    r
                    for r in all_src
                    if "routing.py" in r["file_path"]
                    and r["hunk_start_line"] == 431
                ),
                None,
            )
            if target_hunk:
                merge_sha_val = target_hunk["pr_merge_sha"]
                hunk_content_at_merge = _git_show(_FASTAPI_REPO, merge_sha_val, "fastapi/routing.py")
                if hunk_content_at_merge:
                    lines = hunk_content_at_merge.splitlines()
                    lo = max(0, 431 - 1)
                    hi_idx = min(len(lines), 437)
                    extracted = "\n".join(lines[lo:hi_idx])
                    print(f"\nExtracted hunk (lines 431-437 at merge SHA {merge_sha_val[:12]}):", flush=True)
                    print(extracted, flush=True)
                    print(f"\n_normalize_errors present: {'_normalize_errors' in extracted}", flush=True)
                    print(f"WebSocketRequestValidationError present: {'WebSocketRequestValidationError' in extracted}", flush=True)
                    print(f"endpoint_ctx present: {'endpoint_ctx' in extracted}", flush=True)
                    print(f"e.pos present: {'e.pos' in extracted}", flush=True)
                    print(f"\nbpe_score: {target_hunk.get('bpe_score', 'N/A')}", flush=True)
                    print(f"flagged: {target_hunk['flagged']}", flush=True)
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
    bpe_scores = [r.get("bpe_score", 0.0) for r in all_records if not r["is_test"] and r.get("bpe_score") is not None]
    if bpe_scores:
        bpe_scores_sorted = sorted(bpe_scores, reverse=True)
        print(f"\nBPE score distribution (source hunks, n={len(bpe_scores)}):", flush=True)
        print(f"  max={bpe_scores_sorted[0]:.4f}", flush=True)
        print(f"  p99={bpe_scores_sorted[int(len(bpe_scores_sorted) * 0.01)]:.4f}", flush=True)
        print(f"  p95={bpe_scores_sorted[int(len(bpe_scores_sorted) * 0.05)]:.4f}", flush=True)
        print(f"  median={bpe_scores_sorted[len(bpe_scores_sorted) // 2]:.4f}", flush=True)

    print(f"\nWritten → {_HUNKS_JSONL}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--sanity-check",
        action="store_true",
        help="Run on PR #14609 only and print hunk content for validation",
    )
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
