# engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_ink_2026_04_22.py
"""Phase 14 — TS corpus validation: Ink (vadimdemedes/ink).

Scores 5 hand-picked merged PRs against SequentialImportBpeScorer with a
TypeScript calibration corpus sampled from the pre-merge snapshot of each PR.

Selected PRs (merged Nov 2025 – Apr 2026, no mass refactors, ≥1 .ts/.tsx file):
  #937  fix: Respect disableFocus() when handling Escape
  #906  feat: add border background color support for Box component
  #925  feat: add wrap="hard" option to Text component
  #910  fix: incremental rendering for trailing newline
  #879  fix: mark text node dirty on insertBefore (stale layout)

N_CAL = 500, seed = 0.  No seed-stability probe (Ink pool well above 500 per
eligibility audit). Stability probe triggered only if pool < 400 after filters.

Usage:
    uv run python engine/.../experiments/score_pr_hunks_ts_ink_2026_04_22.py
    uv run python engine/.../experiments/score_pr_hunks_ts_ink_2026_04_22.py --sanity-check
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

from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter
from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    collect_candidates,
    sample_hunks,
)
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
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
_OUT_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_ts_ink_2026_04_22.jsonl"

_INK_REPO = _REPOS_DIR / "ink"
_REPO_GH = "vadimdemedes/ink"
_N_CAL = 500
_CAL_SEED = 0

# Pool-cap safety: if pool < _N_CAL use pool - 5 (stability probe threshold is 400)
_POOL_STABILITY_THRESHOLD = 400

# 5 hand-picked PRs — (pr_number, merge_sha)
# All verified to have ≥1 src .ts/.tsx file touched.
_SELECTED_PRS: list[tuple[int, str]] = [
    (937, "cb6687322886ec245a17fd75999c69b95114ec90"),
    (906, "d3c6d146bb6cf537430382dbf0653e1e50b2f76e"),
    (925, "2b1e3a6c58e3321f2e6a1feeb3cdff64aecfcdd3"),
    (910, "c32da0b3066590df08da5cef8351a7b863081c1b"),
    (879, "1761c3ae42b647a32132a7b34ce75ef143677cf9"),
]

_DEFAULT_EXCLUDE_DIRS: frozenset[str] = frozenset(
    {
        "test",
        "tests",
        "doc",
        "docs",
        "examples",
        "example",
        "migrations",
        "migration",
        "benchmarks",
        "benchmark",
        "fixtures",
        "scripts",
        "build",
        "dist",
        "__pycache__",
        ".git",
    }
)

_git_show_cache: dict[tuple[str, str], str | None] = {}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


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


def _collect_ts_source_files(repo_dir: Path) -> list[Path]:
    """Collect non-test .ts/.tsx files for model A corpus."""
    result: list[Path] = []
    for ext in (".ts", ".tsx"):
        for p in sorted(repo_dir.rglob(f"*{ext}")):
            rel = p.relative_to(repo_dir)
            name = rel.name
            if (
                name.endswith(".test.ts")
                or name.endswith(".test.tsx")
                or name.endswith(".spec.ts")
                or name.endswith(".spec.tsx")
            ):
                continue
            skip = False
            for part in rel.parts[:-1]:
                if part in _DEFAULT_EXCLUDE_DIRS or part.startswith("."):
                    skip = True
                    break
            if skip:
                continue
            result.append(p)
    return result


def _is_ts_file(path: str) -> bool:
    return path.endswith(".ts") or path.endswith(".tsx")


def _is_test_hunk(path: str) -> bool:
    """Return True if this path is a test file.

    Ink places tests in a top-level ``test/`` directory without ``.test.tsx``
    extension convention (e.g. ``test/borders.tsx``), so we also match paths
    that start with ``test/`` or contain a ``/test/`` segment.
    """
    name = path.split("/")[-1]
    parts = path.split("/")
    return (
        name.endswith(".test.ts")
        or name.endswith(".test.tsx")
        or name.endswith(".spec.ts")
        or name.endswith(".spec.tsx")
        or "/test/" in path
        or "/tests/" in path
        or "/__tests__/" in path
        # Ink-style: top-level test/ dir (path starts with "test/")
        or (len(parts) >= 2 and parts[0] == "test")
    )


def _is_source_hunk(path: str) -> bool:
    return _is_ts_file(path) and not _is_test_hunk(path)


def _parse_diff_hunks(diff_text: str) -> list[dict[str, Any]]:
    hunks: list[dict[str, Any]] = []
    current_file: str | None = None
    active_hunk: dict[str, Any] | None = None
    hunk_lines: list[str] = []

    def _flush() -> None:
        if active_hunk is not None:
            active_hunk["diff_content"] = "\n".join(hunk_lines)

    for line in diff_text.splitlines():
        if line.startswith("diff --git "):
            _flush()
            active_hunk = None
            hunk_lines = []
            current_file = None
        elif line.startswith("+++ b/"):
            current_file = line[6:].strip()
        elif line.startswith("+++ /dev/null"):
            current_file = None
        elif line.startswith("@@ ") and current_file is not None:
            _flush()
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

    _flush()
    return hunks


def _run_stability_probe(
    candidates: list[str],
    n_cal: int,
    bpe_model_b_path: Path,
    ts_files: list[Path],
    adapter: TypeScriptAdapter,
    tokenizer: Any,
) -> list[float]:
    """Run 3-seed stability probe at n_cal and return thresholds."""
    import numpy as np  # noqa: PLC0415

    thresholds: list[float] = []
    for probe_seed in [0, 1, 2]:
        rng = np.random.default_rng(probe_seed)
        indices = rng.choice(len(candidates), size=n_cal, replace=False)
        probe_hunks = [candidates[int(i)] for i in sorted(indices)]
        probe_scorer = SequentialImportBpeScorer(
            model_a_files=ts_files,
            bpe_model_b_path=bpe_model_b_path,
            calibration_hunks=probe_hunks,
            adapter=adapter,
            _tokenizer=tokenizer,
        )
        thresholds.append(probe_scorer.bpe_threshold)
    return thresholds


# ---------------------------------------------------------------------------
# Sampleable range coverage report (Check 2)
# ---------------------------------------------------------------------------


def _report_sampleable_ranges(
    ts_files: list[Path], adapter: TypeScriptAdapter
) -> dict[str, int]:
    """Return {relative_path: sampleable_range_count} for all TSX files."""
    tsx_coverage: dict[str, int] = {}
    for p in ts_files:
        if p.suffix != ".tsx":
            continue
        try:
            source = p.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        ranges = adapter.enumerate_sampleable_ranges(source)
        tsx_coverage[p.name] = len(ranges)
    return tsx_coverage


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main(sanity_check: bool = False) -> None:
    from transformers import AutoTokenizer  # noqa: PLC0415

    print("Loading shared tokenizer...", flush=True)
    shared_tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    print("Tokenizer loaded.", flush=True)

    adapter = TypeScriptAdapter()
    prs = _SELECTED_PRS[:1] if sanity_check else _SELECTED_PRS
    if sanity_check:
        print(f"SANITY CHECK MODE — running PR #{prs[0][0]} only", flush=True)

    all_records: list[dict[str, Any]] = []
    n_diffs_failed = 0

    for i, (pr_num, merge_sha) in enumerate(prs):
        pre_sha_ref = f"{merge_sha}^1"
        try:
            result = subprocess.run(
                ["git", "-C", str(_INK_REPO), "rev-parse", pre_sha_ref],
                capture_output=True,
                text=True,
                check=True,
                timeout=30,
            )
            pre_sha = result.stdout.strip()
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"  WARN: rev-parse failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        print(f"  [{i + 1}/{len(prs)}] PR #{pr_num}  pre_sha={pre_sha[:8]}", flush=True)

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)

                archive_proc = subprocess.run(
                    ["git", "-C", str(_INK_REPO), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    print("    WARN: git archive failed, skipping", flush=True)
                    n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                ts_files = _collect_ts_source_files(tmppath)
                print(f"    model_A: {len(ts_files)} .ts/.tsx files", flush=True)
                if not ts_files:
                    print("    WARN: no TS source files, skipping", flush=True)
                    n_diffs_failed += 1
                    continue

                # Check 2 — TSX sampleable range coverage
                tsx_cov = _report_sampleable_ranges(ts_files, adapter)
                zero_tsx = [fname for fname, cnt in tsx_cov.items() if cnt == 0]
                print(
                    f"    TSX sampleable coverage: {len(tsx_cov)} files, "
                    f"zero-range={len(zero_tsx)}",
                    flush=True,
                )
                if zero_tsx:
                    print(f"    WARN zero-range TSX files: {zero_tsx}", flush=True)

                # Data-dominance and auto-gen report (Check 5 / Check 2)
                n_data_dom = sum(
                    1
                    for p in ts_files
                    if adapter.is_data_dominant(
                        p.read_text(encoding="utf-8", errors="replace")
                    )
                )
                n_auto_gen = sum(
                    1
                    for p in ts_files
                    if adapter.is_auto_generated(
                        p.read_text(encoding="utf-8", errors="replace")
                    )
                )
                print(
                    f"    filters: data_dominant={n_data_dom}, auto_generated={n_auto_gen}",
                    flush=True,
                )

                # Collect full candidate pool before sampling (for stability probe check)
                candidates = collect_candidates(
                    tmppath,
                    exclude_dirs=_DEFAULT_EXCLUDE_DIRS,
                    exclude_auto_generated=True,
                    exclude_data_dominant=True,
                    adapter=adapter,
                )
                pool_size = len(candidates)
                print(f"    calibration pool: {pool_size} candidates", flush=True)

                # Determine actual N_CAL (pool-capped)
                n_cal_actual = min(_N_CAL, pool_size)
                if pool_size < _N_CAL:
                    print(
                        f"    WARN: pool {pool_size} < N_CAL {_N_CAL}, "
                        f"capping to {n_cal_actual}",
                        flush=True,
                    )

                # Stability probe if pool < threshold
                if pool_size < _POOL_STABILITY_THRESHOLD:
                    print(
                        f"    Pool {pool_size} < {_POOL_STABILITY_THRESHOLD} — "
                        "running 3-seed stability probe...",
                        flush=True,
                    )
                    probe_thresholds = _run_stability_probe(
                        candidates,
                        n_cal_actual,
                        _BPE_MODEL_B_PATH,
                        ts_files,
                        adapter,
                        shared_tokenizer,
                    )
                    print(
                        f"    Stability probe thresholds (seeds 0,1,2): {probe_thresholds}",
                        flush=True,
                    )
                else:
                    probe_thresholds = []

                cal_hunks = sample_hunks(
                    tmppath,
                    n_cal_actual,
                    _CAL_SEED,
                    exclude_dirs=_DEFAULT_EXCLUDE_DIRS,
                    exclude_auto_generated=True,
                    exclude_data_dominant=True,
                    adapter=adapter,
                )
                print(
                    f"    calibration: {len(cal_hunks)} hunks sampled (N_CAL={n_cal_actual})",
                    flush=True,
                )

                scorer = SequentialImportBpeScorer(
                    model_a_files=ts_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks,
                    adapter=adapter,
                    repo_root=tmppath,
                    _tokenizer=shared_tokenizer,
                )
                cal_threshold = scorer.bpe_threshold
                print(f"    bpe_threshold={cal_threshold:.4f}", flush=True)

        except Exception as exc:
            print(f"    WARN: calibration failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        try:
            diff_result = subprocess.run(
                ["gh", "pr", "diff", str(pr_num), "--repo", _REPO_GH],
                capture_output=True,
                text=True,
                check=True,
                timeout=120,
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

        # Check 3 — glob alias FP check: look for '@/' imports in diff
        alias_imports = [
            line
            for line in diff_text.splitlines()
            if line.startswith("+") and "from '@/" in line
        ]
        if alias_imports:
            print(
                f"    CHECK3: alias imports found in diff for PR #{pr_num}: {alias_imports[:3]}",
                flush=True,
            )

        def _score_hunk_list(
            hunks: list[dict[str, Any]],
            is_test: bool,
            *,
            _pr_num: int = pr_num,
            _merge_sha: str = merge_sha,
            _scorer: SequentialImportBpeScorer = scorer,
            _cal_threshold: float = cal_threshold,
            _pool_size: int = pool_size,
            _n_cal_actual: int = n_cal_actual,
            _probe_thresholds: list[float] = probe_thresholds,
        ) -> list[dict[str, Any]]:
            records: list[dict[str, Any]] = []
            for hunk in hunks:
                file_content = _git_show(_INK_REPO, _merge_sha, hunk["file"])
                if file_content is None:
                    continue
                lines = file_content.splitlines()
                lo = max(0, hunk["start_line"] - 1)
                hi = min(len(lines), hunk["end_line"])
                hunk_content = "\n".join(lines[lo:hi])
                if not hunk_content.strip():
                    continue
                scored = _scorer.score_hunk(
                    hunk_content,
                    file_source=file_content,
                    hunk_start_line=hunk["start_line"],
                    hunk_end_line=hunk["end_line"],
                )
                records.append(
                    {
                        "pr_number": _pr_num,
                        "file_path": hunk["file"],
                        "hunk_start_line": hunk["start_line"],
                        "hunk_end_line": hunk["end_line"],
                        "is_test": is_test,
                        "import_score": scored["import_score"],
                        "bpe_score": scored["bpe_score"],
                        "flagged": scored["flagged"],
                        "reason": scored.get("reason"),
                        "bpe_threshold": _cal_threshold,
                        "pool_size": _pool_size,
                        "n_cal": _n_cal_actual,
                        "stability_probe_thresholds": _probe_thresholds,
                    }
                )
            return records

        src_records = _score_hunk_list(source_hunks, is_test=False)
        tst_records = _score_hunk_list(test_hunks, is_test=True)
        all_records.extend(src_records)
        all_records.extend(tst_records)

        n_src_flagged = sum(1 for r in src_records if r["flagged"])
        n_tst_flagged = sum(1 for r in tst_records if r["flagged"])
        print(
            f"    scored src={len(src_records)} (flagged={n_src_flagged}), "
            f"test={len(tst_records)} (flagged={n_tst_flagged})",
            flush=True,
        )

    if sanity_check:
        print("\nSanity check complete — no output written.", flush=True)
        _print_summary(all_records)
        return

    _OUT_JSONL.parent.mkdir(parents=True, exist_ok=True)
    with _OUT_JSONL.open("w", encoding="utf-8") as fh:
        for rec in all_records:
            fh.write(json.dumps(rec) + "\n")

    _print_summary(all_records)
    print(f"\nWritten → {_OUT_JSONL}", flush=True)


def _print_summary(records: list[dict[str, Any]]) -> None:
    src = [r for r in records if not r["is_test"]]
    tst = [r for r in records if r["is_test"]]
    src_flagged = [r for r in src if r["flagged"]]
    tst_flagged = [r for r in tst if r["flagged"]]

    print("\n=== Summary ===")
    print(f"Total hunks:   src={len(src)}, test={len(tst)}")
    print(f"Total flagged: src={len(src_flagged)}, test={len(tst_flagged)}")
    if src:
        print(f"Flag rate (src): {len(src_flagged)/len(src)*100:.1f}%")

    stage1 = [r for r in src_flagged if r.get("reason") == "import"]
    stage2 = [r for r in src_flagged if r.get("reason") == "bpe"]
    print(f"Stage1 (import): {len(stage1)}, Stage2 (bpe): {len(stage2)}")

    if src_flagged:
        print("\nFlagged source hunks:")
        for r in src_flagged:
            loc = f"{r['file_path']}:{r['hunk_start_line']}-{r['hunk_end_line']}"
            print(
                f"  PR#{r['pr_number']} {loc}"
                f"  reason={r['reason']}  import={r['import_score']:.3f}"
                f"  bpe={r['bpe_score']:.4f}  thr={r['bpe_threshold']:.4f}"
            )

    print("\nPer-PR breakdown:")
    by_pr: dict[int, dict[str, Any]] = {}
    for r in src:
        pr = r["pr_number"]
        if pr not in by_pr:
            by_pr[pr] = {
                "total": 0,
                "flagged": 0,
                "threshold": r["bpe_threshold"],
                "pool_size": r["pool_size"],
            }
        by_pr[pr]["total"] += 1
        if r["flagged"]:
            by_pr[pr]["flagged"] += 1
    for pr_num, stats in sorted(by_pr.items()):
        print(
            f"  PR#{pr_num}: {stats['total']} src hunks, {stats['flagged']} flagged"
            f"  thr={stats['threshold']:.4f}  pool={stats['pool_size']}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sanity-check", action="store_true")
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
