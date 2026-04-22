# engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_hono_2026_04_22.py
"""Phase 14 — TS corpus validation: Hono (honojs/hono).

Scores 5 hand-picked merged PRs against SequentialImportBpeScorer with a
TypeScript calibration corpus sampled from the pre-merge snapshot of each PR.

Selected PRs (merged Oct 2025 – Apr 2026, no mass refactors, ≥1 .ts/.tsx file):
  #4883  fix(aws-lambda): handle invalid header names
  #4848  fix(compress): convert strong ETag to weak ETag
  #4750  fix(bearer-auth): escape regex metacharacters
  #4780  feat(jsx-renderer): support function-based options
  #4834  feat(css): add classNameSlug option to createCssContext

N_CAL = 500, seed = 0.  No seed-stability probe (Hono is Green — analog of FastAPI).

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_hono_2026_04_22.py
    uv run python engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_hono_2026_04_22.py --sanity-check
"""

from __future__ import annotations

import argparse
import io
import json
import re
import subprocess
import tarfile
import tempfile
from collections.abc import Generator
from pathlib import Path
from typing import Any

import numpy as np
import tree_sitter_typescript as tstypescript
from tree_sitter import Language, Node
from tree_sitter import Parser as TsParser

from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter
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
_OUT_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_ts_hono_2026_04_22.jsonl"

_HONO_REPO = _REPOS_DIR / "hono"
_REPO_GH = "honojs/hono"
_N_CAL = 485  # pool varies 488-494 across PR snapshots; 485 is safe floor
_CAL_SEED = 0

# 5 hand-picked PRs — (pr_number, merge_sha)
_SELECTED_PRS: list[tuple[int, str]] = [
    (4883, "fa2c74fe5c3ce996d025d9d97bf5670c207bb82e"),
    (4848, "0bce36bf368ca4c1749285aeed537cba36b551bf"),
    (4750, "0c0bf8d789949d69d4e5bea244c468d2c7d9986b"),
    (4780, "58825a72f7cc0a36d08535fc11dc90934ba77aeb"),
    (4834, "f82aba8e8ea45d56199e751cee6ea7c067bcd176"),
]

_TS_LANGUAGE = Language(tstypescript.language_typescript())
_TSX_LANGUAGE = Language(tstypescript.language_tsx())

# Top-level node types we extract as calibration hunks
_TOP_LEVEL_TYPES: frozenset[str] = frozenset(
    {
        "function_declaration",
        "generator_function_declaration",
        "class_declaration",
        "abstract_class_declaration",
        "interface_declaration",
        "type_alias_declaration",
    }
)

# RHS node types in lexical_declaration that count as function bodies
_FUNCTION_VALUE_TYPES: frozenset[str] = frozenset({"arrow_function", "function_expression"})

_MIN_BODY_LINES = 5

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
# TypeScript calibration hunk sampler
# ---------------------------------------------------------------------------


def _walk(node: Node) -> Generator[Node, None, None]:
    yield node
    for child in node.children:
        yield from _walk(child)


def _extract_lexical_arrow_hunks(node: Node, lines: list[str]) -> list[str]:
    """Return hunk strings for lexical_declaration nodes with arrow/function bodies.

    Handles:  const foo = () => { ... }
              const foo = function() { ... }
    Extracts the whole declarator span when the RHS is a function body >= MIN_BODY_LINES.
    """
    hunks: list[str] = []
    for decl_child in node.children:
        if decl_child.type != "variable_declarator":
            continue
        # Find RHS: first named child after "="
        found_eq = False
        rhs: Node | None = None
        for c in decl_child.children:
            if c.type == "=":
                found_eq = True
                continue
            if found_eq and c.is_named:
                rhs = c
                break
        if rhs is None or rhs.type not in _FUNCTION_VALUE_TYPES:
            continue
        start = decl_child.start_point[0]
        end = decl_child.end_point[0]
        if (end - start) < _MIN_BODY_LINES:
            continue
        hunks.append("\n".join(lines[start : end + 1]))
    return hunks


def _collect_ts_candidates(source_dir: Path, adapter: TypeScriptAdapter) -> list[str]:
    """Return all qualifying hunk strings from .ts/.tsx files in source_dir.

    A qualifying hunk is a top-level function/class/interface/type declaration
    OR a const arrow-function assignment, with at least MIN_BODY_LINES lines.
    Auto-generated and data-dominant files are excluded.
    """
    hunks: list[str] = []
    for ext in (".ts", ".tsx"):
        lang = _TSX_LANGUAGE if ext == ".tsx" else _TS_LANGUAGE
        for ts_file in sorted(source_dir.rglob(f"*{ext}")):
            rel = ts_file.relative_to(source_dir)
            # Skip test files
            name = rel.name
            if (
                name.endswith(".test.ts")
                or name.endswith(".test.tsx")
                or name.endswith(".spec.ts")
                or name.endswith(".spec.tsx")
                or "test" in rel.parts
                or "tests" in rel.parts
                or "__tests__" in rel.parts
            ):
                continue
            # Skip excluded dirs
            skip = False
            for part in rel.parts[:-1]:
                if part in _DEFAULT_EXCLUDE_DIRS or part.startswith("."):
                    skip = True
                    break
            if skip:
                continue
            try:
                source = ts_file.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if adapter.is_auto_generated(source):
                continue
            if adapter.is_data_dominant(source):
                continue
            try:
                parser = TsParser(lang)
                tree = parser.parse(source.encode("utf-8"))
            except Exception:
                continue
            lines = source.splitlines()
            root = tree.root_node
            for child in root.children:
                # Unwrap `export` wrapper to get the inner declaration
                inner = child
                if child.type == "export_statement":
                    for sub in child.children:
                        if sub.type in _TOP_LEVEL_TYPES or sub.type in (
                            "lexical_declaration",
                            "variable_declaration",
                        ):
                            inner = sub
                            break

                if inner.type in _TOP_LEVEL_TYPES:
                    start = inner.start_point[0]
                    end = inner.end_point[0]
                    if (end - start) >= _MIN_BODY_LINES:
                        hunks.append("\n".join(lines[start : end + 1]))
                elif inner.type in ("lexical_declaration", "variable_declaration"):
                    hunks.extend(_extract_lexical_arrow_hunks(inner, lines))
    return hunks


def _sample_ts_hunks(source_dir: Path, n: int, seed: int, adapter: TypeScriptAdapter) -> list[str]:
    candidates = _collect_ts_candidates(source_dir, adapter)
    if len(candidates) < n:
        raise ValueError(
            f"Only {len(candidates)} qualifying TS hunks in {source_dir!r}, need {n}."
        )
    rng = np.random.default_rng(seed)
    indices = rng.choice(len(candidates), size=n, replace=False)
    return [candidates[int(i)] for i in sorted(indices)]


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
    return (
        name.endswith(".test.ts")
        or name.endswith(".test.tsx")
        or name.endswith(".spec.ts")
        or name.endswith(".spec.tsx")
        or "/test/" in path
        or "/tests/" in path
        or "/__tests__/" in path
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


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main(sanity_check: bool = False) -> None:
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    print("Loading shared tokenizer...", flush=True)
    shared_tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
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
                ["git", "-C", str(_HONO_REPO), "rev-parse", pre_sha_ref],
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
                    ["git", "-C", str(_HONO_REPO), "archive", pre_sha],
                    capture_output=True,
                    timeout=120,
                )
                if archive_proc.returncode != 0:
                    print(f"    WARN: git archive failed, skipping", flush=True)
                    n_diffs_failed += 1
                    continue

                with tarfile.open(fileobj=io.BytesIO(archive_proc.stdout)) as tf:
                    tf.extractall(tmppath)

                ts_files = _collect_ts_source_files(tmppath)
                print(f"    model_A: {len(ts_files)} .ts/.tsx files", flush=True)
                if not ts_files:
                    print(f"    WARN: no TS source files, skipping", flush=True)
                    n_diffs_failed += 1
                    continue

                # Data-dominance report (inline — before filtering happens inside scorer)
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

                cal_hunks = _sample_ts_hunks(tmppath, _N_CAL, _CAL_SEED, adapter)
                print(f"    calibration: {len(cal_hunks)} hunks sampled (N_CAL={_N_CAL})", flush=True)

                scorer = SequentialImportBpeScorer(
                    model_a_files=ts_files,
                    bpe_model_b_path=_BPE_MODEL_B_PATH,
                    calibration_hunks=cal_hunks,
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

        def _score_hunk_list(
            hunks: list[dict[str, Any]],
            is_test: bool,
        ) -> list[dict[str, Any]]:
            records: list[dict[str, Any]] = []
            for hunk in hunks:
                file_content = _git_show(_HONO_REPO, merge_sha, hunk["file"])
                if file_content is None:
                    continue
                lines = file_content.splitlines()
                lo = max(0, hunk["start_line"] - 1)
                hi = min(len(lines), hunk["end_line"])
                hunk_content = "\n".join(lines[lo:hi])
                if not hunk_content.strip():
                    continue
                scored = scorer.score_hunk(
                    hunk_content,
                    file_source=file_content,
                    hunk_start_line=hunk["start_line"],
                    hunk_end_line=hunk["end_line"],
                )
                records.append(
                    {
                        "pr_number": pr_num,
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
            print(
                f"  PR#{r['pr_number']} {r['file_path']}:{r['hunk_start_line']}-{r['hunk_end_line']}"
                f"  reason={r['reason']}  import={r['import_score']:.3f}  bpe={r['bpe_score']:.4f}"
                f"  thr={r['bpe_threshold']:.4f}"
            )

    print("\nPer-PR breakdown:")
    by_pr: dict[int, dict[str, Any]] = {}
    for r in src:
        pr = r["pr_number"]
        if pr not in by_pr:
            by_pr[pr] = {"total": 0, "flagged": 0, "threshold": r["bpe_threshold"]}
        by_pr[pr]["total"] += 1
        if r["flagged"]:
            by_pr[pr]["flagged"] += 1
    for pr_num, stats in sorted(by_pr.items()):
        print(
            f"  PR#{pr_num}: {stats['total']} src hunks, {stats['flagged']} flagged"
            f"  thr={stats['threshold']:.4f}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sanity-check", action="store_true")
    args = parser.parse_args()
    main(sanity_check=args.sanity_check)
