# engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix1.py
"""Phase 14 Exp #6 Step 3 — Score PR hunks with fix1 (Stage 1 regex fallback removed).

Reads:  real_pr_base_rate_prs_2026_04_22.jsonl   (cached — NOT re-mined)
Writes: real_pr_base_rate_hunks_fix1_2026_04_22.jsonl

Calibration: seed 0, n=100 hunks from .argot/research/repos/fastapi source.
Scoring:     SequentialImportBpeScorer with fix1 applied (SyntaxError → set(), no regex fallback).
Extraction:  file-start-to-hunk-end on current HEAD of cached repo.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_fix1.py
"""

from __future__ import annotations

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
_PRS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_prs_2026_04_22.jsonl"
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix1_2026_04_22.jsonl"

_FASTAPI_REPO = _REPOS_DIR / "fastapi"
_REPO_GH = "tiangolo/fastapi"
_N_CAL = 100
_CAL_SEED = 0


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


def _extract_file_to_hunk_end(file_path: Path, end_line: int) -> str | None:
    if not file_path.exists():
        return None
    lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
    hi = min(len(lines), end_line)
    return "\n".join(lines[:hi])


def _get_foreign_modules(hunk_source: str, repo_modules: frozenset[str]) -> list[str]:
    return sorted(_imports_from_ast(hunk_source) - repo_modules)


def main() -> None:
    # Load PR list
    prs: list[dict[str, Any]] = []
    with _PRS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"Loaded {len(prs)} PRs", flush=True)

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
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  {pr['mergedAt'][:10]}  {pr['title'][:55]}",
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
                fp = _FASTAPI_REPO / hunk["file"]
                content = _extract_file_to_hunk_end(fp, hunk["end_line"])
                if content is None:
                    n_missing += 1
                    continue
                if not content.strip():
                    continue
                scored = scorer.score_hunk(content)
                foreign = (
                    _get_foreign_modules(content, repo_modules)
                    if scored["reason"] == "import"
                    else []
                )
                records.append(
                    {
                        "pr_number": pr_rec["number"],
                        "pr_title": pr_rec["title"],
                        "pr_mergedAt": pr_rec["mergedAt"],
                        "pr_url": pr_rec["url"],
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

    # Write output
    with _HUNKS_JSONL.open("w", encoding="utf-8") as fh:
        for rec in all_records:
            fh.write(json.dumps(rec) + "\n")

    src_total = sum(1 for r in all_records if not r["is_test"])
    src_flagged = sum(1 for r in all_records if not r["is_test"] and r["flagged"])
    print(f"\nDiffs failed: {n_diffs_failed}", flush=True)
    print(f"Files missing from HEAD: {n_files_missing}", flush=True)
    print(f"Source hunks scored: {src_total}", flush=True)
    print(
        f"Source hunks flagged: {src_flagged} " f"({src_flagged / src_total:.1%})"
        if src_total
        else "Source hunks flagged: 0",
        flush=True,
    )
    print(f"Written → {_HUNKS_JSONL}", flush=True)


if __name__ == "__main__":
    main()
