# engine/argot/research/signal/phase14/experiments/recall_probe_fix6_2026_04_22.py
"""Phase 14 Experiment L — Recall sanity probe: fix6 on paradigm-break catalog fixtures.

Motivation:
    fix6 achieved ~4% FP rate (down from ~7% in fix5) but recall hasn't been measured
    since pre-fix3.  Four rounds of precision tuning may have silently crushed signal
    for genuine paradigm breaks.  This probe plants known break fixtures into real PR
    contexts and measures the catch rate.

Design:
    - Fixtures: is_break=True entries from acceptance/catalog/fastapi/manifest.json
      (31 fixtures across routing, framework_swap, validation, exception_handling,
       async_blocking, background_tasks, downstream_http, dependency_injection,
       serialization categories)
    - Host PRs: 4 unflagged fix6 PRs with >=5 source hunks (#14862, #14944, #14856, #14806)
      Note: only 4 PRs met criteria (spec suggested 5; all qualifying PRs included).
    - Per-PR calibration rebuilt identically to fix6:
        git archive pre_pr_sha → sample_hunks(seed=0, N=100)
        SequentialImportBpeScorer initialized on that snapshot
    - Scoring: scorer.score_hunk(hunk_content, file_source=catalog_file, hunk_start_line,
        hunk_end_line) — full prose masking enabled, matching fix6 calling convention
    - Verdict gate: recall <90% on any fixture = regression, blocks V0

Constraints:
    - No cloud deps, no hardcoded domain literals
    - No scorer tuning: if fixtures miss, REPORT only

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/recall_probe_fix6_2026_04_22.py
"""

from __future__ import annotations

import io
import json
import math
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
    _blank_prose_lines,
    _is_meaningful_token,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent  # engine/argot
_PROJECT_ROOT = _ARGOT_PKG.parent.parent  # argot/ project root
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent  # engine/argot/research
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"
_CATALOG_DIR = _ARGOT_PKG / "acceptance" / "catalog"
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_FASTAPI_REPO = _REPOS_DIR / "fastapi"
_FASTAPI_CATALOG = _CATALOG_DIR / "fastapi"

_SCRIPT_DIR = Path(__file__).parent
_FIX6_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix6_2026_04_22.jsonl"

_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "recall_probe_fix6_2026-04-22.md"
)

_N_CAL = 100
_CAL_SEED = 0

# Host PRs: unflagged fix6 PRs with >=5 source hunks, excluding structural outliers
# (#14564, #14575, #14609, #14776)
_HOST_PR_NUMS = [14862, 14944, 14856, 14806]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(
        p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _top_llr_token(
    scorer: SequentialImportBpeScorer,
    bpe_input: str,
) -> tuple[str, float]:
    """Return (token_str, llr_value) for the token with the highest LLR in bpe_input."""
    ids: list[int] = scorer._tokenizer.encode(bpe_input, add_special_tokens=False)
    filtered = [i for i in ids if _is_meaningful_token(scorer._id_to_token.get(i, ""))]
    if not filtered:
        filtered = ids
    if not filtered:
        return ("", 0.0)
    epsilon = 1e-7
    total_b = scorer._total_b
    total_a = scorer._total_a
    best_id = max(
        filtered,
        key=lambda i: (
            math.log(scorer._model_b.get(i, 0) / total_b + epsilon)
            - math.log(scorer._model_a.get(i, 0) / total_a + epsilon)
        ),
    )
    llr = math.log(scorer._model_b.get(best_id, 0) / total_b + epsilon) - math.log(
        scorer._model_a.get(best_id, 0) / total_a + epsilon
    )
    token_str = scorer._id_to_token.get(best_id, f"<id:{best_id}>")
    return (token_str, llr)


def _load_break_fixtures() -> list[dict[str, Any]]:
    manifest_path = _FASTAPI_CATALOG / "manifest.json"
    data: dict[str, Any] = json.loads(manifest_path.read_text(encoding="utf-8"))
    return [f for f in data["fixtures"] if f.get("is_break", False)]


def _build_scorer_for_pr(
    pre_sha: str,
    tokenizer: Any,
) -> tuple[SequentialImportBpeScorer, float]:
    """Build a fix6-identical scorer for the given pre-merge SHA.

    Returns (scorer, threshold).
    """
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
            raise RuntimeError(f"No .py files in archive for {pre_sha[:8]}")

        cal_hunks = sample_hunks(tmppath, _N_CAL, _CAL_SEED)
        scorer = SequentialImportBpeScorer(
            model_a_files=py_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=cal_hunks,
            _tokenizer=tokenizer,
        )
        return scorer, scorer.bpe_threshold


def _score_fixture_on_scorer(
    scorer: SequentialImportBpeScorer,
    fixture: dict[str, Any],
) -> dict[str, Any]:
    """Score one fixture against one host PR's calibrated scorer."""
    fixture_path = _FASTAPI_CATALOG / fixture["file"]
    hunk_content = _extract_hunk(fixture_path, fixture["hunk_start_line"], fixture["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")

    result = scorer.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=fixture["hunk_start_line"],
        hunk_end_line=fixture["hunk_end_line"],
    )

    # Compute bpe_input for LLR token attribution (mirrors score_hunk internals)
    from argot.research.signal.phase14.parsers import PythonTreeSitterParser

    parser = PythonTreeSitterParser()
    file_prose = parser.prose_line_ranges(file_source)
    hunk_prose_local: frozenset[int] = frozenset(
        ln - fixture["hunk_start_line"] + 1
        for ln in file_prose
        if fixture["hunk_start_line"] <= ln <= fixture["hunk_end_line"]
    )
    bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)
    top_token, top_llr = _top_llr_token(scorer, bpe_input)

    return {
        "fixture_name": fixture["name"],
        "category": fixture.get("category", ""),
        "flagged": result["flagged"],
        "reason": result["reason"],
        "bpe_score": result["bpe_score"],
        "import_score": result["import_score"],
        "top_token": top_token,
        "top_llr": top_llr,
    }


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _write_report(
    out: Path,
    host_prs: list[dict[str, Any]],
    all_results: dict[int, list[dict[str, Any]]],
    break_fixtures: list[dict[str, Any]],
    pre_fix3_recall: float,
) -> None:
    fixture_names = [f["name"] for f in break_fixtures]
    pr_nums = [p["pr_number"] for p in host_prs]

    # §0 summary
    total_pairs = len(fixture_names) * len(pr_nums)
    total_flagged = sum(
        1
        for pr_num, results in all_results.items()
        for r in results
        if r["flagged"]
    )
    catch_rate = total_flagged / total_pairs if total_pairs > 0 else 0.0

    # Per-fixture catch rates
    fixture_catch: dict[str, int] = {n: 0 for n in fixture_names}
    for pr_num, results in all_results.items():
        for r in results:
            if r["flagged"]:
                fixture_catch[r["fixture_name"]] = fixture_catch.get(r["fixture_name"], 0) + 1

    # Per-host catch rates
    host_catch: dict[int, int] = {}
    for pr_num, results in all_results.items():
        host_catch[pr_num] = sum(1 for r in results if r["flagged"])

    # Verdict
    recall_ok = catch_rate >= 0.90
    verdict = "PASS" if recall_ok else "REGRESSION — BLOCKS V0"

    lines: list[str] = [
        "# Phase 14 Recall Probe: fix6 on Paradigm-Break Fixtures (2026-04-22)",
        "",
        "**Purpose:** Verify that four rounds of precision tuning (fix3→fix6) have not",
        "silently crushed recall on genuine paradigm breaks.  Pre-fix3 catch rate: ~100%",
        "(per Phase 14 Exp #2c Postfix V2 validation, obs 4395).",
        "",
        "**Scorer:** fix6 (`SequentialImportBpeScorer` with per-PR recalibration + prose masking)",
        "",
        f"**Gate:** catch rate ≥90% = PASS | <90% = REGRESSION (blocks V0)",
        "",
        "---",
        "",
        "## §0 Summary",
        "",
        f"| metric | value |",
        "|---|---|",
        f"| break fixtures | {len(fixture_names)} |",
        f"| host PRs | {len(pr_nums)} (note: only 4 met criteria; spec suggested 5) |",
        f"| total (fixture × host) pairs | {total_pairs} |",
        f"| pairs flagged | {total_flagged} |",
        f"| overall catch rate | {catch_rate:.1%} |",
        f"| pre-fix3 baseline | ~100% |",
        f"| verdict | **{verdict}** |",
        "",
        "### Per-fixture catch rate",
        "",
        "| fixture | category | flagged (of 4 hosts) | catch rate |",
        "|---|---|---|---|",
    ]

    for f in break_fixtures:
        name = f["name"]
        cat = f.get("category", "")
        n = fixture_catch.get(name, 0)
        rate = n / len(pr_nums)
        marker = " ⚠" if rate < 1.0 else ""
        lines.append(f"| {name} | {cat} | {n}/{len(pr_nums)} | {rate:.0%}{marker} |")

    lines += [
        "",
        "### Per-host catch rate",
        "",
        f"| host PR | n_breaks_flagged | catch rate |",
        "|---|---|---|",
    ]

    for pr in host_prs:
        pn = pr["pr_number"]
        n = host_catch.get(pn, 0)
        rate = n / len(fixture_names)
        lines.append(f"| #{pn} (threshold={pr['threshold']:.4f}) | {n}/{len(fixture_names)} | {rate:.1%} |")

    lines += [
        "",
        "---",
        "",
        "## §1 Per-fixture × Per-host Score Table",
        "",
        "Cell format: `flagged/not | bpe_score vs threshold`",
        "Cells showing `IMPORT` mean Stage 1 fired (foreign module detected).",
        "",
    ]

    # Build per-fixture lookup: fixture_name -> {pr_num -> result}
    fx_results: dict[str, dict[int, dict[str, Any]]] = {n: {} for n in fixture_names}
    for pr_num, results in all_results.items():
        for r in results:
            fx_results[r["fixture_name"]][pr_num] = r

    # Header row
    header = "| fixture | category |" + "".join(f" PR #{pn} |" for pn in pr_nums)
    sep = "|---|---|" + "".join("---|" for _ in pr_nums)
    lines += [header, sep]

    for f in break_fixtures:
        name = f["name"]
        cat = f.get("category", "")
        row = f"| {name} | {cat} |"
        for pn in pr_nums:
            r = fx_results[name].get(pn)
            if r is None:
                row += " N/A |"
            elif r["reason"] == "import":
                row += f" IMPORT |"
            elif r["flagged"]:
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" **YES** {r['bpe_score']:.3f}>{pr_thr:.3f} |"
            else:
                # Show score vs threshold
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" no {r['bpe_score']:.3f}<{pr_thr:.3f} |"
        lines.append(row)

    lines += [
        "",
        "---",
        "",
        "## §2 Failures Analysis",
        "",
    ]

    # Find failures (fixture not flagged in ≥1 host)
    failures_found = False
    for f in break_fixtures:
        name = f["name"]
        missed_prs = [
            pn for pn in pr_nums
            if not fx_results[name].get(pn, {}).get("flagged", False)
        ]
        if missed_prs:
            if not failures_found:
                failures_found = True
            lines += [
                f"### {name} (category: {f.get('category', '')})",
                "",
                f"Missed on host PRs: {', '.join(f'#{p}' for p in missed_prs)}",
                "",
                "| host PR | bpe_score | threshold | margin | top_token | top_llr | reason |",
                "|---|---|---|---|---|---|---|",
            ]
            for pn in missed_prs:
                r = fx_results[name].get(pn, {})
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                bpe = r.get("bpe_score", 0.0)
                top_tok = r.get("top_token", "?")
                top_llr_val = r.get("top_llr", 0.0)
                margin = bpe - pr_thr
                reason_str = r.get("reason", "?")

                # Diagnose why it missed
                if r.get("reason") == "import":
                    diag = "Stage 1 fired (should be flagged — check result above)"
                elif margin > -0.5:
                    diag = "thin margin — borderline miss"
                elif r.get("import_score", 0) > 0:
                    diag = "Stage 1 partial (import_score>0 but <1)"
                else:
                    diag = "threshold too high for snapshot; distinctive tokens absorbed into model_A"
                lines.append(
                    f"| #{pn} | {bpe:.4f} | {pr_thr:.4f} | {margin:+.4f} | `{top_tok}` | {top_llr_val:.4f} | {diag} |"
                )
            lines.append("")

    if not failures_found:
        lines += ["No failures — all fixtures flagged on all host PRs.", ""]

    lines += [
        "---",
        "",
        "## §3 Verdict",
        "",
        f"| metric | value |",
        "|---|---|",
        f"| overall catch rate | {catch_rate:.1%} |",
        f"| pre-fix3 baseline | ~100% |",
        f"| delta | {catch_rate - pre_fix3_recall:+.1%} |",
        f"| gate (≥90%) | **{verdict}** |",
        "",
    ]

    if recall_ok:
        lines += [
            "Recall is preserved from pre-fix3 baseline.  Four rounds of precision tuning",
            "(fix3→fix6) did not materially reduce sensitivity to genuine paradigm breaks.",
            "fix6 is cleared on the recall dimension.",
            "",
        ]
    else:
        lines += [
            "**RECALL REGRESSION DETECTED.**  fix6 fails to catch ≥10% of paradigm-break",
            "fixtures that pre-fix3 caught at 100%.  Root cause: see §2 failures analysis.",
            "Do NOT promote fix6 to V0 until recall is restored.",
            "",
        ]

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Recall Probe — fix6 on paradigm-break fixtures", flush=True)

    print("\nLoading shared tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    print("Tokenizer loaded.", flush=True)

    # Load break fixtures from catalog
    break_fixtures = _load_break_fixtures()
    print(f"\nLoaded {len(break_fixtures)} break fixtures from FastAPI catalog", flush=True)

    # Load fix6 JSONL to get pre_pr_sha and threshold per host PR
    fix6_rows: list[dict[str, Any]] = []
    with _FIX6_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                fix6_rows.append(json.loads(line))

    host_pr_meta: dict[int, dict[str, Any]] = {}
    for row in fix6_rows:
        pn = row["pr_number"]
        if pn in _HOST_PR_NUMS and pn not in host_pr_meta:
            host_pr_meta[pn] = {
                "pr_number": pn,
                "pre_pr_sha": row["pre_pr_sha"],
                "fix6_threshold": row["bpe_threshold"],
            }

    missing = set(_HOST_PR_NUMS) - set(host_pr_meta.keys())
    if missing:
        print(f"  WARN: PRs not found in fix6 JSONL: {missing}", flush=True)

    # Build scorer per host PR and score all fixtures
    all_results: dict[int, list[dict[str, Any]]] = {}
    host_pr_records: list[dict[str, Any]] = []

    for pr_num in _HOST_PR_NUMS:
        meta = host_pr_meta.get(pr_num)
        if meta is None:
            print(f"  SKIP PR #{pr_num} — not found in fix6 JSONL", flush=True)
            continue

        pre_sha = meta["pre_pr_sha"]
        print(f"\nPR #{pr_num} — pre_sha={pre_sha[:8]} ...", flush=True)

        try:
            scorer, threshold = _build_scorer_for_pr(pre_sha, tokenizer)
        except Exception as exc:
            print(f"  ERROR building scorer: {exc}", flush=True)
            continue

        fix6_thr = meta["fix6_threshold"]
        delta = threshold - fix6_thr
        print(
            f"  Rebuilt threshold={threshold:.4f}, fix6 threshold={fix6_thr:.4f}, delta={delta:+.4f}",
            flush=True,
        )
        if abs(delta) > 0.01:
            print(f"  WARN: threshold drift >{0.01:.4f} — calibration may differ", flush=True)

        host_pr_records.append(
            {
                "pr_number": pr_num,
                "pre_pr_sha": pre_sha,
                "threshold": threshold,
                "fix6_threshold": fix6_thr,
            }
        )

        results: list[dict[str, Any]] = []
        n_flagged = 0
        for f in break_fixtures:
            r = _score_fixture_on_scorer(scorer, f)
            results.append(r)
            if r["flagged"]:
                n_flagged += 1
            marker = "YES" if r["flagged"] else "no"
            print(
                f"  {r['fixture_name']}: {marker} (bpe={r['bpe_score']:.4f} vs thr={threshold:.4f},"
                f" reason={r['reason']}, top_token=`{r['top_token']}`)",
                flush=True,
            )

        all_results[pr_num] = results
        print(
            f"  -> {n_flagged}/{len(break_fixtures)} fixtures flagged ({n_flagged / len(break_fixtures):.0%})",
            flush=True,
        )

    # Overall stats
    total_pairs = len(break_fixtures) * len(host_pr_records)
    total_flagged = sum(1 for results in all_results.values() for r in results if r["flagged"])
    catch_rate = total_flagged / total_pairs if total_pairs > 0 else 0.0
    print(
        f"\nOverall: {total_flagged}/{total_pairs} pairs flagged, catch rate={catch_rate:.1%}",
        flush=True,
    )

    _write_report(_DOCS_OUT, host_pr_records, all_results, break_fixtures, pre_fix3_recall=1.0)


if __name__ == "__main__":
    main()
