# engine/argot/research/signal/phase14/experiments/revalidate_rich_post_glob_fix_2026_04_22.py
"""Part B re-validation: prove the adapter refactor (ast → tree-sitter) is behavior-preserving.

Re-runs the fix10 Rich validation against the current scorer code (post-glob-alias fix)
and diffs the flag set against the fix10 baseline JSONL.

The glob fix is TS-only — PythonAdapter.prefixes is always empty — so flag counts on
Rich must be exactly 23 with the same (pr, file, hunk_start) keys.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/revalidate_rich_post_glob_fix_2026_04_22.py
    uv run python engine/argot/research/signal/phase14/experiments/revalidate_rich_post_glob_fix_2026_04_22.py --sanity-check
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
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
)

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"

_SCRIPT_DIR = Path(__file__).parent
_PRS_JSONL = _SCRIPT_DIR / "rich_real_pr_base_rate_prs_2026_04_22.jsonl"
_BASELINE_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix10_rich_2026_04_22.jsonl"
_OUT_JSONL = _SCRIPT_DIR / "revalidate_rich_post_glob_fix_2026_04_22.jsonl"

_RICH_REPO = _REPOS_DIR / "rich"
_REPO_GH = "Textualize/rich"
_N_CAL = 230
_CAL_SEED = 0

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
    return path.startswith("rich/") and path.endswith(".py")


def _is_test_hunk(path: str) -> bool:
    return (
        path.startswith("tests/")
        or "test_" in path.split("/")[-1]
        or path.split("/")[-1].endswith("_test.py")
    ) and path.endswith(".py")


def _hunk_key(r: dict[str, Any]) -> tuple[int, str, int]:
    return (r["pr_number"], r["file_path"], r["hunk_start_line"])


def main(sanity_check: bool = False) -> None:
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    print("Loading shared tokenizer...", flush=True)
    shared_tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    print("Tokenizer loaded.", flush=True)

    prs: list[dict[str, Any]] = []
    with _PRS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                prs.append(json.loads(line))
    print(f"Loaded {len(prs)} PRs", flush=True)

    if sanity_check:
        prs = prs[:1]
        print(f"SANITY CHECK MODE — running PR #{prs[0]['number']} only", flush=True)

    all_records: list[dict[str, Any]] = []
    n_diffs_failed = 0

    for i, pr in enumerate(prs):
        pr_num = pr["number"]
        merge_sha = pr["mergeCommit"]["oid"]
        pre_sha_ref = f"{merge_sha}^1"

        try:
            result = subprocess.run(
                ["git", "-C", str(_RICH_REPO), "rev-parse", pre_sha_ref],
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
                    ["git", "-C", str(_RICH_REPO), "archive", pre_sha],
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

                cal_hunks = sample_hunks(tmppath, _N_CAL, _CAL_SEED)

                scorer = SequentialImportBpeScorer(
                    model_a_files=py_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks,
                    _tokenizer=shared_tokenizer,
                )

                cal_threshold = scorer.bpe_threshold

        except Exception as exc:
            print(f"  WARN: calibration failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

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
            continue

        all_hunks = _parse_diff_hunks(diff_text)
        source_hunks = [h for h in all_hunks if _is_source_hunk(h["file"])]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        def _score_hunks(
            hunks: list[dict[str, Any]],
            is_test: bool,
            pr_rec: dict[str, Any],
        ) -> list[dict[str, Any]]:
            records: list[dict[str, Any]] = []
            for hunk in hunks:
                pr_merge_sha = pr_rec["mergeCommit"]["oid"]
                file_content = _git_show(_RICH_REPO, pr_merge_sha, hunk["file"])
                if file_content is None:
                    continue
                lines = file_content.splitlines()
                lo = max(0, hunk["start_line"] - 1)
                hi_idx = min(len(lines), hunk["end_line"])
                hunk_content = "\n".join(lines[lo:hi_idx])
                file_text = file_content
                if not hunk_content.strip():
                    continue
                scored = scorer.score_hunk(
                    hunk_content,
                    file_source=file_text,
                    hunk_start_line=hunk["start_line"],
                    hunk_end_line=hunk["end_line"],
                )
                records.append(
                    {
                        "pr_number": pr_rec["number"],
                        "file_path": hunk["file"],
                        "hunk_start_line": hunk["start_line"],
                        "hunk_end_line": hunk["end_line"],
                        "is_test": is_test,
                        "import_score": scored["import_score"],
                        "bpe_score": scored["bpe_score"],
                        "flagged": scored["flagged"],
                        "reason": scored.get("reason"),
                        "bpe_threshold": cal_threshold,
                    }
                )
            return records

        src_records = _score_hunks(source_hunks, is_test=False, pr_rec=pr)
        tst_records = _score_hunks(test_hunks, is_test=True, pr_rec=pr)
        all_records.extend(src_records)
        all_records.extend(tst_records)

        n_flagged = sum(1 for r in src_records if r["flagged"])
        print(
            f"  [{i + 1:2d}/{len(prs)}] PR #{pr_num}  flagged={n_flagged}",
            flush=True,
        )

    if sanity_check:
        print("Sanity check complete — no output written.", flush=True)
        return

    with _OUT_JSONL.open("w", encoding="utf-8") as fh:
        for rec in all_records:
            fh.write(json.dumps(rec) + "\n")

    # --- diff against fix10 baseline ---
    baseline_records = [
        json.loads(l)
        for l in _BASELINE_JSONL.read_text(encoding="utf-8").splitlines()
        if l.strip()
    ]

    baseline_src = [r for r in baseline_records if not r["is_test"]]
    new_src = [r for r in all_records if not r["is_test"]]

    baseline_flagged = {_hunk_key(r) for r in baseline_src if r["flagged"]}
    new_flagged = {_hunk_key(r) for r in new_src if r["flagged"]}

    added = new_flagged - baseline_flagged
    removed = baseline_flagged - new_flagged

    print(f"\n=== Re-validation summary ===")
    print(f"Baseline source hunks:  {len(baseline_src)}")
    print(f"Re-run  source hunks:   {len(new_src)}")
    print(f"Baseline flags: {len(baseline_flagged)}")
    print(f"Re-run  flags:  {len(new_flagged)}")

    if not added and not removed:
        print("\nZERO DELTA — refactor is behavior-preserving on Python (Rich corpus).")
    else:
        if added:
            print(f"\nNEW flags ({len(added)}):")
            for key in sorted(added):
                print(f"  PR#{key[0]} {key[1]}:{key[2]}")
        if removed:
            print(f"\nDROPPED flags ({len(removed)}):")
            for key in sorted(removed):
                print(f"  PR#{key[0]} {key[1]}:{key[2]}")

    print(f"\nWritten → {_OUT_JSONL}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sanity-check", action="store_true")
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
