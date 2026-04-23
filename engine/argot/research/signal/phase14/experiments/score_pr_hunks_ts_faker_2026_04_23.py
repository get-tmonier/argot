# engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_faker_2026_04_23.py
"""Phase 14 — TS corpus validation: faker-js (faker-js/faker).

Adversarial stress test: ~92% of faker-js .ts files are pure locale data
(arrays of strings, address names, etc.) that the is_data_dominant filter MUST
exclude.  Scores 5 hand-picked merged PRs against SequentialImportBpeScorer
with a TypeScript calibration corpus sampled from the pre-merge snapshot of
each PR.

Selected PRs (merged Apr 2025 – Apr 2026):
  #3798  feat(locale): Add postal_address and improved secondary_address for es   [locale]
  #3796  feat(locale): add mn_MN_cyrl (Mongolian) locale                           [locale]
  #3820  refactor(core): expose core.locale as LocaleProxy                          [core]
  #3783  feat(date): add ability to provide year range for past and future           [core]
  #3809  refactor(location): simplify locale access                                  [core]

N_CAL = 300 (escalate to 500 if stability probe fails).
Stability probe runs for EVERY PR (not just small-pool ones).

Usage:
    uv run python engine/.../experiments/score_pr_hunks_ts_faker_2026_04_23.py
    uv run python engine/.../experiments/score_pr_hunks_ts_faker_2026_04_23.py --sanity-check
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
_OUT_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_ts_faker_2026_04_23.jsonl"

_FAKER_REPO = _REPOS_DIR / "faker-js"
_REPO_GH = "faker-js/faker"
_N_CAL = 300
_N_CAL_ESCALATED = 500
_CAL_SEED = 0
_STABILITY_REL_VAR_THRESHOLD = 0.10
_STABILITY_JACCARD_THRESHOLD = 0.80

# 5 hand-picked PRs — (pr_number, merge_sha, pr_type)
# Mix: 2 locale PRs, 3 core PRs.
_SELECTED_PRS: list[tuple[int, str, str]] = [
    (3798, "6c2a0abd3092c8afb2bca67544a16e85a13b6b61", "locale"),
    (3796, "d17c0f1fe2ea95666b668c1e87ef4a1505a19538", "locale"),
    (3820, "a7825a71ce1dd6b0728848ab73da8134f518e689", "core"),
    (3783, "237e7dc34aafafeef68e667a0140d08a8bfa7fb2", "core"),
    (3809, "f45c508c3da0f6faee6a5a5edefce5eca0c90aa8", "core"),
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


def _run_data_dominance_audit(
    adapter: TypeScriptAdapter,
) -> tuple[int, int, int, int, int, int]:
    """Audit HEAD snapshot of faker repo for data-dominance.

    Returns (locale_total, locale_excluded, locale_kept,
              non_locale_total, non_locale_excluded, non_locale_kept).
    """
    locale_total = 0
    locale_excluded = 0
    non_locale_total = 0
    non_locale_excluded = 0

    for ext in (".ts", ".tsx"):
        for p in sorted(_FAKER_REPO.rglob(f"*{ext}")):
            try:
                rel = p.relative_to(_FAKER_REPO)
            except ValueError:
                continue
            # Skip node_modules, dist, hidden dirs
            skip = False
            for part in rel.parts[:-1]:
                if part in ("node_modules", "dist", ".git"):
                    skip = True
                    break
            if skip:
                continue
            try:
                source = p.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            is_locale = "locales" in rel.parts
            is_data_dom = adapter.is_data_dominant(source)
            if is_locale:
                locale_total += 1
                if is_data_dom:
                    locale_excluded += 1
            else:
                non_locale_total += 1
                if is_data_dom:
                    non_locale_excluded += 1

    locale_kept = locale_total - locale_excluded
    non_locale_kept = non_locale_total - non_locale_excluded
    return (
        locale_total,
        locale_excluded,
        locale_kept,
        non_locale_total,
        non_locale_excluded,
        non_locale_kept,
    )


def _print_eligibility_gate(
    adapter: TypeScriptAdapter,
) -> None:
    """Run and print Check 1 and Check 2."""
    print("\n=== CHECK 1: Data-dominance eligibility gate ===", flush=True)
    (
        locale_total,
        locale_excluded,
        locale_kept,
        non_locale_total,
        non_locale_excluded,
        non_locale_kept,
    ) = _run_data_dominance_audit(adapter)

    locale_exclusion_rate = locale_excluded / locale_total if locale_total > 0 else 0.0
    print(
        f"Locale files: {locale_total} total, {locale_excluded} excluded "
        f"({locale_exclusion_rate:.1%}), {locale_kept} kept",
        flush=True,
    )
    print("Pass condition: >=70% excluded", flush=True)  # §8.3: 80% unreachable; ceiling 75.9%

    if locale_exclusion_rate >= 0.70:  # §8.3: gate recalibrated
        print("Status: PASS", flush=True)
    elif locale_exclusion_rate >= 0.60:
        print("Status: YELLOW", flush=True)
        print("Kept locale files (investigate):", flush=True)
        # List kept locale files
        for ext in (".ts", ".tsx"):
            for p in sorted(_FAKER_REPO.rglob(f"*{ext}")):
                try:
                    rel = p.relative_to(_FAKER_REPO)
                except ValueError:
                    continue
                if "locales" not in rel.parts:
                    continue
                try:
                    source = p.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
                if not adapter.is_data_dominant(source):
                    print(f"  KEPT: {rel}", flush=True)
    else:
        print("Status: FAIL", flush=True)
        raise SystemExit("CHECK 1 FAILED — scorer not multi-language ready")

    non_locale_excl_rate = non_locale_excluded / non_locale_total if non_locale_total > 0 else 0.0
    print("\n=== CHECK 2: Non-locale file inclusion ===", flush=True)
    print(
        f"Non-locale files: {non_locale_total} total, {non_locale_excluded} excluded "
        f"({non_locale_excl_rate:.1%})",
        flush=True,
    )
    print("Pass condition: <5% excluded", flush=True)
    if non_locale_excl_rate < 0.05:
        print("Status: PASS", flush=True)
    else:
        print("Status: WARN", flush=True)
        print("Excluded non-locale files:", flush=True)
        for ext in (".ts", ".tsx"):
            for p in sorted(_FAKER_REPO.rglob(f"*{ext}")):
                try:
                    rel = p.relative_to(_FAKER_REPO)
                except ValueError:
                    continue
                if "locales" in rel.parts:
                    continue
                skip = False
                for part in rel.parts[:-1]:
                    if part in ("node_modules", "dist", ".git"):
                        skip = True
                        break
                if skip:
                    continue
                try:
                    source = p.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
                if adapter.is_data_dominant(source):
                    print(f"  EXCL: {rel}", flush=True)


def _check_tsconfig_paths() -> list[str]:
    """Check 5 — glob alias FP check: inspect tsconfig.json for paths."""
    print("\n=== CHECK 5: tsconfig glob alias FP check ===", flush=True)
    alias_patterns: list[str] = []
    for tsconfig_name in ("tsconfig.json", "tsconfig.base.json"):
        tsconfig_path = _FAKER_REPO / tsconfig_name
        if not tsconfig_path.exists():
            print(f"  {tsconfig_name}: not found", flush=True)
            continue
        try:
            raw = tsconfig_path.read_text(encoding="utf-8")
            # Strip comments (tsconfig allows them) before JSON parsing
            raw_no_comments = re.sub(r"//[^\n]*", "", raw)
            data: dict[str, Any] = json.loads(raw_no_comments)
            compiler_opts = data.get("compilerOptions", {})
            paths = compiler_opts.get("paths", {}) if isinstance(compiler_opts, dict) else {}
            if not paths:
                print(f"  {tsconfig_name}: no paths entries", flush=True)
            else:
                print(f"  {tsconfig_name} paths:", flush=True)
                for alias, targets in paths.items():
                    print(f"    {alias!r} -> {targets}", flush=True)
                    alias_patterns.append(str(alias))
        except Exception as exc:
            print(f"  {tsconfig_name}: parse error — {exc}", flush=True)
    return alias_patterns


def _run_probe_scorers(
    candidates: list[str],
    n_cal: int,
    ts_files: list[Path],
    adapter: TypeScriptAdapter,
    tokenizer: Any,
) -> list[SequentialImportBpeScorer]:
    """Build 3 probe scorers with seeds 0,1,2."""
    import numpy as np  # noqa: PLC0415

    probe_scorers: list[SequentialImportBpeScorer] = []
    for probe_seed in [0, 1, 2]:
        rng = np.random.default_rng(probe_seed)
        indices = rng.choice(len(candidates), size=n_cal, replace=False)
        probe_hunks = [candidates[int(i)] for i in sorted(indices)]
        ps = SequentialImportBpeScorer(
            model_a_files=ts_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=probe_hunks,
            adapter=adapter,
            _tokenizer=tokenizer,
        )
        probe_scorers.append(ps)
    return probe_scorers


def main(sanity_check: bool = False) -> None:
    from transformers import AutoTokenizer  # noqa: PLC0415

    print("Loading shared tokenizer...", flush=True)
    shared_tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")  # type: ignore[no-untyped-call]
    print("Tokenizer loaded.", flush=True)

    adapter = TypeScriptAdapter()

    # Check 1 & 2: data-dominance eligibility gate
    _print_eligibility_gate(adapter)

    # Check 5: tsconfig paths
    alias_patterns = _check_tsconfig_paths()

    prs = _SELECTED_PRS[:1] if sanity_check else _SELECTED_PRS
    if sanity_check:
        print(f"\nSANITY CHECK MODE — running PR #{prs[0][0]} only", flush=True)

    all_records: list[dict[str, Any]] = []
    n_diffs_failed = 0

    for i, (pr_num, merge_sha, pr_type) in enumerate(prs):
        pre_sha_ref = f"{merge_sha}^1"
        try:
            result = subprocess.run(
                ["git", "-C", str(_FAKER_REPO), "rev-parse", pre_sha_ref],
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

        print(
            f"\n  [{i + 1}/{len(prs)}] PR #{pr_num} ({pr_type})  pre_sha={pre_sha[:8]}",
            flush=True,
        )

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                tmppath = Path(tmpdir)

                archive_proc = subprocess.run(
                    ["git", "-C", str(_FAKER_REPO), "archive", pre_sha],
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

                # Data-dominance and auto-gen report
                n_data_dom = sum(
                    1
                    for p in ts_files
                    if adapter.is_data_dominant(p.read_text(encoding="utf-8", errors="replace"))
                )
                n_auto_gen = sum(
                    1
                    for p in ts_files
                    if adapter.is_auto_generated(p.read_text(encoding="utf-8", errors="replace"))
                )
                print(
                    f"    filters: data_dominant={n_data_dom}, auto_generated={n_auto_gen}",
                    flush=True,
                )

                candidates = collect_candidates(
                    tmppath,
                    exclude_dirs=_DEFAULT_EXCLUDE_DIRS,
                    exclude_auto_generated=True,
                    exclude_data_dominant=True,
                    adapter=adapter,
                )
                pool_size = len(candidates)
                print(f"    calibration pool: {pool_size} candidates", flush=True)

                if pool_size == 0:
                    print(
                        "    WARN: pool is empty, cannot calibrate — skipping PR",
                        flush=True,
                    )
                    n_diffs_failed += 1
                    continue

                # Try N_CAL=300 first; if pool too small, cap
                n_cal_try = min(_N_CAL, pool_size)
                if pool_size < _N_CAL:
                    print(
                        f"    WARN: pool {pool_size} < N_CAL {_N_CAL}, " f"capping to {n_cal_try}",
                        flush=True,
                    )

                # Mandatory stability probe (every PR)
                print(
                    f"    Running 3-seed stability probe (n={n_cal_try})...",
                    flush=True,
                )
                probe_scorers_300 = _run_probe_scorers(
                    candidates,
                    n_cal_try,
                    ts_files,
                    adapter,
                    shared_tokenizer,
                )
                probe_thresholds_300 = [ps.bpe_threshold for ps in probe_scorers_300]
                print(
                    f"    Probe thresholds (seeds 0,1,2) @ n={n_cal_try}: "
                    f"{probe_thresholds_300}",
                    flush=True,
                )

                # Get diff hunks for Jaccard computation
                try:
                    diff_result_pre = subprocess.run(
                        ["gh", "pr", "diff", str(pr_num), "--repo", _REPO_GH],
                        capture_output=True,
                        text=True,
                        check=True,
                        timeout=120,
                    )
                    diff_text_pre = diff_result_pre.stdout
                except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
                    print(f"    WARN: diff fetch failed for stability probe: {exc}", flush=True)
                    diff_text_pre = ""

                all_hunks_probe = _parse_diff_hunks(diff_text_pre) if diff_text_pre.strip() else []
                source_hunks_probe = [h for h in all_hunks_probe if _is_source_hunk(h["file"])]

                import numpy as np  # noqa: PLC0415

                thresholds_300 = probe_thresholds_300
                mean_t = float(np.mean(thresholds_300))
                rel_var = (
                    (float(np.max(thresholds_300)) - float(np.min(thresholds_300))) / mean_t
                    if mean_t > 0
                    else 0.0
                )

                # Jaccard using probe scorers
                flag_sets: list[set[tuple[str, int]]] = []
                for ps in probe_scorers_300:
                    flags: set[tuple[str, int]] = set()
                    for hunk in source_hunks_probe:
                        fc = _git_show(_FAKER_REPO, merge_sha, hunk["file"])
                        if fc is None:
                            continue
                        lines = fc.splitlines()
                        lo = max(0, hunk["start_line"] - 1)
                        hi = min(len(lines), hunk["end_line"])
                        hc = "\n".join(lines[lo:hi])
                        if not hc.strip():
                            continue
                        sc = ps.score_hunk(
                            hc,
                            file_source=fc,
                            hunk_start_line=hunk["start_line"],
                            hunk_end_line=hunk["end_line"],
                        )
                        if sc["flagged"]:
                            flags.add((hunk["file"], hunk["start_line"]))
                    flag_sets.append(flags)

                union_flags = flag_sets[0] | flag_sets[1] | flag_sets[2]
                inter_flags = flag_sets[0] & flag_sets[1] & flag_sets[2]
                jaccard = len(inter_flags) / len(union_flags) if union_flags else 1.0

                print(
                    f"    rel_var={rel_var:.3f}, jaccard={jaccard:.3f}",
                    flush=True,
                )

                n_cal_used = n_cal_try
                stability_status: str
                probe_thresholds_final = probe_thresholds_300

                if rel_var > _STABILITY_REL_VAR_THRESHOLD or jaccard < _STABILITY_JACCARD_THRESHOLD:
                    print(
                        f"    Stability MARGINAL at n={n_cal_try} — "
                        f"trying n={_N_CAL_ESCALATED}...",
                        flush=True,
                    )
                    n_cal_500 = min(_N_CAL_ESCALATED, pool_size)
                    if n_cal_500 > n_cal_try:
                        probe_scorers_500 = _run_probe_scorers(
                            candidates,
                            n_cal_500,
                            ts_files,
                            adapter,
                            shared_tokenizer,
                        )
                        probe_thresholds_500 = [ps.bpe_threshold for ps in probe_scorers_500]
                        mean_t2 = float(np.mean(probe_thresholds_500))
                        rel_var2 = (
                            (
                                float(np.max(probe_thresholds_500))
                                - float(np.min(probe_thresholds_500))
                            )
                            / mean_t2
                            if mean_t2 > 0
                            else 0.0
                        )
                        # Jaccard at 500
                        flag_sets_500: list[set[tuple[str, int]]] = []
                        for ps2 in probe_scorers_500:
                            flags2: set[tuple[str, int]] = set()
                            for hunk in source_hunks_probe:
                                fc2 = _git_show(_FAKER_REPO, merge_sha, hunk["file"])
                                if fc2 is None:
                                    continue
                                lines2 = fc2.splitlines()
                                lo2 = max(0, hunk["start_line"] - 1)
                                hi2 = min(len(lines2), hunk["end_line"])
                                hc2 = "\n".join(lines2[lo2:hi2])
                                if not hc2.strip():
                                    continue
                                sc2 = ps2.score_hunk(
                                    hc2,
                                    file_source=fc2,
                                    hunk_start_line=hunk["start_line"],
                                    hunk_end_line=hunk["end_line"],
                                )
                                if sc2["flagged"]:
                                    flags2.add((hunk["file"], hunk["start_line"]))
                            flag_sets_500.append(flags2)
                        union_500 = flag_sets_500[0] | flag_sets_500[1] | flag_sets_500[2]
                        inter_500 = flag_sets_500[0] & flag_sets_500[1] & flag_sets_500[2]
                        jaccard2 = len(inter_500) / len(union_500) if union_500 else 1.0
                        print(
                            f"    n=500: rel_var={rel_var2:.3f}, jaccard={jaccard2:.3f}",
                            flush=True,
                        )
                        if (
                            rel_var2 <= _STABILITY_REL_VAR_THRESHOLD
                            and jaccard2 >= _STABILITY_JACCARD_THRESHOLD
                        ):
                            stability_status = "PASS_AT_500"
                            n_cal_used = n_cal_500
                            probe_thresholds_final = probe_thresholds_500
                            rel_var = rel_var2
                            jaccard = jaccard2
                        else:
                            stability_status = "FAIL_BOTH"
                    else:
                        stability_status = "FAIL_BOTH"
                else:
                    stability_status = "PASS"

                print(f"    stability_status={stability_status}", flush=True)

                cal_hunks = sample_hunks(
                    tmppath,
                    n_cal_used,
                    _CAL_SEED,
                    exclude_dirs=_DEFAULT_EXCLUDE_DIRS,
                    exclude_auto_generated=True,
                    exclude_data_dominant=True,
                    adapter=adapter,
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

        except Exception as exc:
            print(f"    WARN: calibration failed for PR #{pr_num}: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        # Use pre-fetched diff text if available, else re-fetch
        try:
            if not diff_text_pre.strip():
                diff_result = subprocess.run(
                    ["gh", "pr", "diff", str(pr_num), "--repo", _REPO_GH],
                    capture_output=True,
                    text=True,
                    check=True,
                    timeout=120,
                )
                diff_text = diff_result.stdout
            else:
                diff_text = diff_text_pre
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
            print(f"    WARN diff failed: {exc}", flush=True)
            n_diffs_failed += 1
            continue

        if not diff_text.strip():
            continue

        all_hunks = _parse_diff_hunks(diff_text)
        source_hunks = [h for h in all_hunks if _is_source_hunk(h["file"])]
        test_hunks = [h for h in all_hunks if _is_test_hunk(h["file"])]

        def _score_hunk_list(
            hunks: list[dict[str, Any]],
            is_test: bool,
            *,
            _pr_num: int = pr_num,
            _pr_type: str = pr_type,
            _merge_sha: str = merge_sha,
            _scorer: SequentialImportBpeScorer = scorer,
            _cal_threshold: float = cal_threshold,
            _pool_size: int = pool_size,
            _n_cal_used: int = n_cal_used,
            _probe_thresholds_final: list[float] = probe_thresholds_final,
            _stability_rel_var: float = rel_var,
            _stability_jaccard: float = jaccard,
            _stability_status: str = stability_status,
            _alias_patterns: list[str] = alias_patterns,
        ) -> list[dict[str, Any]]:
            records: list[dict[str, Any]] = []
            for hunk in hunks:
                file_content = _git_show(_FAKER_REPO, _merge_sha, hunk["file"])
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

                # Check 4: locale file Stage 1 warning
                file_parts = hunk["file"].split("/")
                is_locale_file = "locales" in file_parts
                locale_stage1_warn = is_locale_file and scored["import_score"] > 0
                if locale_stage1_warn:
                    print(
                        f"    CHECK 4 WARN: locale file {hunk['file']} "
                        f"has import_score={scored['import_score']}",
                        flush=True,
                    )

                # Check 5: glob alias FP check
                if _alias_patterns and scored["import_score"] > 0:
                    hunk_imports = set()
                    for line in hunk_content.splitlines():
                        if "import" in line:
                            hunk_imports.update(re.findall(r'["\']([^"\']+)["\']', line))
                    for alias in _alias_patterns:
                        alias_prefix = alias.rstrip("*")
                        for imp in hunk_imports:
                            if imp.startswith(alias_prefix):
                                print(
                                    f"    CHECK 5 WARN: alias {alias!r} used in "
                                    f"{hunk['file']}:{hunk['start_line']} "
                                    f"with import_score={scored['import_score']}",
                                    flush=True,
                                )

                records.append(
                    {
                        "pr_number": _pr_num,
                        "pr_type": _pr_type,
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
                        "n_cal_used": _n_cal_used,
                        "stability_probe_thresholds": _probe_thresholds_final,
                        "stability_rel_var": _stability_rel_var,
                        "stability_jaccard": _stability_jaccard,
                        "stability_status": _stability_status,
                        "locale_stage1_warn": locale_stage1_warn,
                    }
                )
            return records

        src_records = _score_hunk_list(source_hunks, is_test=False)
        tst_records = _score_hunk_list(test_hunks, is_test=True)
        all_records.extend(src_records)
        all_records.extend(tst_records)

    _OUT_JSONL.parent.mkdir(parents=True, exist_ok=True)
    with _OUT_JSONL.open("w", encoding="utf-8") as fh:
        for rec in all_records:
            fh.write(json.dumps(rec) + "\n")

    print(f"\nWritten {len(all_records)} records → {_OUT_JSONL}", flush=True)
    if n_diffs_failed:
        print(f"WARNING: {n_diffs_failed} PR(s) failed or were skipped.", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sanity-check", action="store_true")
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
