# engine/argot/research/signal/phase14/experiments/stage2_recall_probe_2026_04_22.py
"""Phase 14 Experiment O — Stage-2 Recall Isolation.

Motivation:
    Step L recall probe passed 124/124 but ALL fixtures fired via Stage 1
    (foreign imports).  Stage 2 (BPE) recall is unmeasured after four rounds
    of precision tuning.  Before V0 ship, confirm BPE-scoring still catches
    paradigm shifts when the Stage 1 fast path doesn't rescue them.

Design:
    Stage2OnlyScorer: subclass of SequentialImportBpeScorer that overrides
    score_hunk to always skip Stage 1 (import_score forced to 0) and force
    Stage 2 execution.  Production scorer is unchanged.

    Phase 1 — Existing catalog through Stage 2 only:
        31 break fixtures × 4 host PRs (#14862, #14944, #14856, #14806).
        Same per-PR recalibration as fix6.  Verdict gate: ≥50%.

    Phase 2 — Stage-2-only fixture pack (8 fixtures):
        Fixtures in fixtures/stage2_only/ use only stdlib / in-corpus imports
        so Stage 1 cannot fire.  Same 4 host PRs.  Verdict gate: ≥70%.

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/stage2_recall_probe_2026_04_22.py
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
    Reason,
    ScoredHunk,
    SequentialImportBpeScorer,
    _blank_prose_lines,
    _is_meaningful_token,
)

# ---------------------------------------------------------------------------
# Stage2OnlyScorer — experiment-only subclass, do not use in production
# ---------------------------------------------------------------------------


class Stage2OnlyScorer(SequentialImportBpeScorer):
    """SequentialImportBpeScorer with Stage 1 (import detection) permanently disabled.

    Overrides score_hunk to set import_score=0 unconditionally, forcing every
    hunk through Stage 2 (BPE-tfidf) regardless of whether it imports foreign
    modules.  All other behaviour — threshold computation, prose masking,
    BPE scoring — is identical to the parent.

    Purpose: isolate Stage 2 recall independently of Stage 1.
    """

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
    ) -> ScoredHunk:
        # Stage 1 permanently disabled — import_score always 0
        import_score: float = 0.0

        # Stage 2: prose masking + BPE (identical to parent)
        bpe_input = hunk_content
        if file_source is not None and hunk_start_line is not None and hunk_end_line is not None:
            file_prose = self._parser.prose_line_ranges(file_source)
            hunk_prose_local: frozenset[int] = frozenset(
                ln - hunk_start_line + 1
                for ln in file_prose
                if hunk_start_line <= ln <= hunk_end_line
            )
            bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)

        bpe_score: float = self._bpe_score(bpe_input)
        reason: Reason = "bpe" if bpe_score > self.bpe_threshold else "none"

        return {
            "import_score": import_score,
            "bpe_score": bpe_score,
            "flagged": reason != "none",
            "reason": reason,
        }


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

_STAGE2_FIXTURES_DIR = Path(__file__).parent.parent / "fixtures" / "stage2_only"

_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "stage2_recall_probe_2026-04-22.md"
)

_N_CAL = 100
_CAL_SEED = 0
_HOST_PR_NUMS = [14862, 14944, 14856, 14806]

# Phase 2 fixture definitions: name, file, hunk_start_line, hunk_end_line
_STAGE2_FIXTURE_META: list[dict[str, Any]] = [
    {
        "name": "walrus_operator",
        "file": _STAGE2_FIXTURES_DIR / "walrus_operator.py",
        "hunk_start_line": 12,
        "hunk_end_line": 36,
        "description": "walrus := in while/if conditions — Python 3.8+",
    },
    {
        "name": "match_case",
        "file": _STAGE2_FIXTURES_DIR / "match_case.py",
        "hunk_start_line": 12,
        "hunk_end_line": 35,
        "description": "match/case structural pattern matching — Python 3.10+",
    },
    {
        "name": "dataclass_migration",
        "file": _STAGE2_FIXTURES_DIR / "dataclass_migration.py",
        "hunk_start_line": 13,
        "hunk_end_line": 37,
        "description": "@dataclass(frozen=True, slots=True) — host uses plain __init__ classes",
    },
    {
        "name": "fstring_adoption",
        "file": _STAGE2_FIXTURES_DIR / "fstring_adoption.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "f-strings with nested {val!r:>10} format specs throughout",
    },
    {
        "name": "async_adoption",
        "file": _STAGE2_FIXTURES_DIR / "async_adoption.py",
        "hunk_start_line": 13,
        "hunk_end_line": 36,
        "description": "asyncio.gather / to_thread / Semaphore concurrency primitives",
    },
    {
        "name": "genexpr_shift",
        "file": _STAGE2_FIXTURES_DIR / "genexpr_shift.py",
        "hunk_start_line": 13,
        "hunk_end_line": 33,
        "description": "sum/any/all genexpr chains where host uses list comprehensions",
    },
    {
        "name": "type_annotations",
        "file": _STAGE2_FIXTURES_DIR / "type_annotations.py",
        "hunk_start_line": 12,
        "hunk_end_line": 38,
        "description": "PEP 695 type parameters def f[T] / Protocol with covariant TypeVars",
    },
    {
        "name": "union_syntax",
        "file": _STAGE2_FIXTURES_DIR / "union_syntax.py",
        "hunk_start_line": 14,
        "hunk_end_line": 36,
        "description": "X | None / int | str union syntax throughout — Python 3.10+",
    },
]


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
    return scorer._id_to_token.get(best_id, f"<id:{best_id}>"), llr


def _load_break_fixtures() -> list[dict[str, Any]]:
    manifest_path = _FASTAPI_CATALOG / "manifest.json"
    data: dict[str, Any] = json.loads(manifest_path.read_text(encoding="utf-8"))
    return [f for f in data["fixtures"] if f.get("is_break", False)]


def _build_scorer_for_pr(
    pre_sha: str,
    tokenizer: Any,
) -> tuple[Stage2OnlyScorer, float]:
    """Build a Stage2OnlyScorer for the given pre-merge SHA."""
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
        scorer = Stage2OnlyScorer(
            model_a_files=py_files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=cal_hunks,
            _tokenizer=tokenizer,
        )
        return scorer, scorer.bpe_threshold


def _score_catalog_fixture(
    scorer: Stage2OnlyScorer,
    fixture: dict[str, Any],
) -> dict[str, Any]:
    """Score one catalog fixture (from manifest) against a calibrated scorer."""
    fixture_path = _FASTAPI_CATALOG / fixture["file"]
    hunk_content = _extract_hunk(fixture_path, fixture["hunk_start_line"], fixture["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")

    result = scorer.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=fixture["hunk_start_line"],
        hunk_end_line=fixture["hunk_end_line"],
    )

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


def _score_stage2_fixture(
    scorer: Stage2OnlyScorer,
    meta: dict[str, Any],
) -> dict[str, Any]:
    """Score one Phase 2 stage2_only fixture against a calibrated scorer."""
    fixture_path: Path = meta["file"]
    hunk_content = _extract_hunk(fixture_path, meta["hunk_start_line"], meta["hunk_end_line"])
    file_source = fixture_path.read_text(encoding="utf-8", errors="replace")

    result = scorer.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=meta["hunk_start_line"],
        hunk_end_line=meta["hunk_end_line"],
    )

    from argot.research.signal.phase14.parsers import PythonTreeSitterParser

    parser = PythonTreeSitterParser()
    file_prose = parser.prose_line_ranges(file_source)
    hunk_prose_local: frozenset[int] = frozenset(
        ln - meta["hunk_start_line"] + 1
        for ln in file_prose
        if meta["hunk_start_line"] <= ln <= meta["hunk_end_line"]
    )
    bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)
    top_token, top_llr = _top_llr_token(scorer, bpe_input)

    return {
        "fixture_name": meta["name"],
        "description": meta.get("description", ""),
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
    phase1_results: dict[int, list[dict[str, Any]]],
    phase2_results: dict[int, list[dict[str, Any]]],
    break_fixtures: list[dict[str, Any]],
    stage2_fixtures: list[dict[str, Any]],
) -> None:
    pr_nums = [p["pr_number"] for p in host_prs]

    def _catch_stats(
        results: dict[int, list[dict[str, Any]]],
        fixture_names: list[str],
    ) -> tuple[int, int, dict[str, int], dict[int, int]]:
        total_pairs = len(fixture_names) * len(pr_nums)
        total_flagged = sum(1 for rs in results.values() for r in rs if r["flagged"])
        fixture_catch: dict[str, int] = {n: 0 for n in fixture_names}
        for rs in results.values():
            for r in rs:
                if r["flagged"]:
                    fixture_catch[r["fixture_name"]] = fixture_catch.get(r["fixture_name"], 0) + 1
        host_catch: dict[int, int] = {
            pn: sum(1 for r in results.get(pn, []) if r["flagged"]) for pn in pr_nums
        }
        return total_pairs, total_flagged, fixture_catch, host_catch

    p1_names = [f["name"] for f in break_fixtures]
    p2_names = [m["name"] for m in stage2_fixtures]

    p1_total, p1_flagged, p1_fx_catch, p1_host_catch = _catch_stats(phase1_results, p1_names)
    p2_total, p2_flagged, p2_fx_catch, p2_host_catch = _catch_stats(phase2_results, p2_names)

    p1_rate = p1_flagged / p1_total if p1_total else 0.0
    p2_rate = p2_flagged / p2_total if p2_total else 0.0

    if p1_rate < 0.50:
        verdict = "STAGE-2 OVERTUNED — Phase 1 <50%. Recommend rollback investigation."
    elif p2_rate >= 0.70:
        verdict = "STAGE-2 FUNCTIONAL — V0 unblocked on this dimension."
    else:
        verdict = "STAGE-2 LIMITED — catches foreign-vocabulary only, not in-repo paradigm shifts."

    lines: list[str] = [
        "# Phase 14 Stage-2 Recall Isolation Probe (2026-04-22)",
        "",
        "**Purpose:** Measure Stage 2 (BPE-tfidf) recall independently of Stage 1",
        "(import detection).  Step L proved 124/124 recall but all fixtures fired via",
        "Stage 1.  This probe forces Stage 2 by using `Stage2OnlyScorer` (import_score",
        "permanently 0) across two phases.",
        "",
        "**Scorer:** `Stage2OnlyScorer` (experiment subclass, Stage 1 disabled)",
        "**Per-PR recalibration:** identical to fix6 (git archive → sample_hunks seed=0 N=100)",
        "",
        "---",
        "",
        "## §0 Summary",
        "",
        "| phase | fixtures | host PRs | total pairs | flagged | catch rate | gate | result |",
        "|---|---|---|---|---|---|---|---|",
        f"| Phase 1 — catalog breaks | {len(p1_names)} | {len(pr_nums)} | {p1_total} |"
        f" {p1_flagged} | {p1_rate:.1%} | ≥50% | {'PASS' if p1_rate >= 0.50 else 'FAIL'} |",
        f"| Phase 2 — stage2-only fixtures | {len(p2_names)} | {len(pr_nums)} | {p2_total} |"
        f" {p2_flagged} | {p2_rate:.1%} | ≥70% | {'PASS' if p2_rate >= 0.70 else 'FAIL'} |",
        "",
        f"**Verdict: {verdict}**",
        "",
        "---",
        "",
        "## §1 Phase 1 — Catalog Break Fixtures Through Stage 2 Only",
        "",
        "All 31 break fixtures from the acceptance catalog, scored via Stage2OnlyScorer.",
        "Import-flagged fixtures from Step L are now tested on BPE merit alone.",
        "",
        "### Per-fixture catch rate",
        "",
        "| fixture | category | flagged (of 4 hosts) | catch rate |",
        "|---|---|---|---|",
    ]

    for f in break_fixtures:
        n = p1_fx_catch.get(f["name"], 0)
        rate = n / len(pr_nums)
        marker = " ⚠" if rate < 1.0 else ""
        lines.append(f"| {f['name']} | {f.get('category', '')} | {n}/{len(pr_nums)} | {rate:.0%}{marker} |")

    lines += [
        "",
        "### Per-host catch rate (Phase 1)",
        "",
        "| host PR | threshold | flagged | catch rate |",
        "|---|---|---|---|",
    ]
    for pr in host_prs:
        pn = pr["pr_number"]
        n = p1_host_catch.get(pn, 0)
        lines.append(
            f"| #{pn} | {pr['threshold']:.4f} | {n}/{len(p1_names)} | {n / len(p1_names):.1%} |"
        )

    lines += [
        "",
        "### Phase 1 — per-fixture × per-host score table",
        "",
        "Cell format: `YES bpe>thr` / `no bpe<thr`.",
        "",
    ]

    # Build lookup: fixture_name → {pr_num → result}
    p1_lookup: dict[str, dict[int, dict[str, Any]]] = {n: {} for n in p1_names}
    for pn, rs in phase1_results.items():
        for r in rs:
            p1_lookup[r["fixture_name"]][pn] = r

    header = "| fixture | category |" + "".join(f" #{pn} |" for pn in pr_nums)
    sep = "|---|---|" + "".join("---|" for _ in pr_nums)
    lines += [header, sep]

    for f in break_fixtures:
        name = f["name"]
        row = f"| {name} | {f.get('category', '')} |"
        for pn in pr_nums:
            r = p1_lookup[name].get(pn)
            if r is None:
                row += " N/A |"
            elif r["flagged"]:
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" **YES** {r['bpe_score']:.3f}>{pr_thr:.3f} |"
            else:
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" no {r['bpe_score']:.3f}<{pr_thr:.3f} |"
        lines.append(row)

    # Phase 1 failures analysis
    lines += [
        "",
        "### Phase 1 — failures (fixtures that never fire via Stage 2)",
        "",
    ]
    p1_failures = [f for f in break_fixtures if p1_fx_catch.get(f["name"], 0) == 0]
    if p1_failures:
        lines += [
            "| fixture | category | max bpe_score (across 4 hosts) | top_token | top_llr |",
            "|---|---|---|---|---|",
        ]
        for f in p1_failures:
            name = f["name"]
            scores = [r["bpe_score"] for r in p1_lookup[name].values()]
            max_bpe = max(scores) if scores else 0.0
            # Use the result from the host with the highest score for token attribution
            best_r = max(p1_lookup[name].values(), key=lambda r: r["bpe_score"], default={})
            tok = best_r.get("top_token", "?")
            llr = best_r.get("top_llr", 0.0)
            lines.append(
                f"| {name} | {f.get('category', '')} | {max_bpe:.4f} | `{tok}` | {llr:.4f} |"
            )
    else:
        lines.append("None — all fixtures fired via Stage 2 on at least one host PR.")

    p1_partial = [f for f in break_fixtures if 0 < p1_fx_catch.get(f["name"], 0) < len(pr_nums)]
    if p1_partial:
        lines += [
            "",
            "### Phase 1 — partial misses (fire in some hosts only)",
            "",
            "| fixture | category | hosts hit/total | scores vs threshold |",
            "|---|---|---|---|",
        ]
        for f in p1_partial:
            name = f["name"]
            n = p1_fx_catch[name]
            score_cells = " / ".join(
                f"{r['bpe_score']:.3f}{'>' if r['flagged'] else '<'}{next(p['threshold'] for p in host_prs if p['pr_number'] == pn):.3f}"
                for pn, r in sorted(p1_lookup[name].items())
            )
            lines.append(f"| {name} | {f.get('category', '')} | {n}/{len(pr_nums)} | {score_cells} |")

    lines += [
        "",
        "---",
        "",
        "## §2 Phase 2 — Stage-2-Only Fixture Pack",
        "",
        "8 fixtures using only stdlib / in-corpus imports.  Stage 1 cannot fire.",
        "Tests whether BPE catches in-repo paradigm shifts (syntax/style, not framework swaps).",
        "",
        "### Fixture catalogue",
        "",
        "| fixture | pattern | hunk lines |",
        "|---|---|---|",
    ]
    for m in stage2_fixtures:
        n_lines = m["hunk_end_line"] - m["hunk_start_line"] + 1
        lines.append(f"| {m['name']} | {m['description']} | {n_lines} |")

    lines += [
        "",
        "### Per-fixture catch rate (Phase 2)",
        "",
        "| fixture | flagged (of 4 hosts) | catch rate |",
        "|---|---|---|",
    ]
    for m in stage2_fixtures:
        n = p2_fx_catch.get(m["name"], 0)
        rate = n / len(pr_nums)
        marker = " ⚠" if rate == 0.0 else ""
        lines.append(f"| {m['name']} | {n}/{len(pr_nums)} | {rate:.0%}{marker} |")

    lines += [
        "",
        "### Per-host catch rate (Phase 2)",
        "",
        "| host PR | threshold | flagged | catch rate |",
        "|---|---|---|---|",
    ]
    for pr in host_prs:
        pn = pr["pr_number"]
        n = p2_host_catch.get(pn, 0)
        lines.append(
            f"| #{pn} | {pr['threshold']:.4f} | {n}/{len(p2_names)} | {n / len(p2_names):.1%} |"
        )

    lines += [
        "",
        "### Phase 2 — per-fixture × per-host score table",
        "",
    ]

    p2_lookup: dict[str, dict[int, dict[str, Any]]] = {m["name"]: {} for m in stage2_fixtures}
    for pn, rs in phase2_results.items():
        for r in rs:
            p2_lookup[r["fixture_name"]][pn] = r

    header2 = "| fixture |" + "".join(f" #{pn} |" for pn in pr_nums)
    sep2 = "|---|" + "".join("---|" for _ in pr_nums)
    lines += [header2, sep2]

    for m in stage2_fixtures:
        name = m["name"]
        row = f"| {name} |"
        for pn in pr_nums:
            r = p2_lookup[name].get(pn)
            if r is None:
                row += " N/A |"
            elif r["flagged"]:
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" **YES** {r['bpe_score']:.3f}>{pr_thr:.3f} |"
            else:
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                row += f" no {r['bpe_score']:.3f}<{pr_thr:.3f} |"
        lines.append(row)

    # Phase 2 fixture-level analysis
    lines += [
        "",
        "### Phase 2 — fixture-level analysis",
        "",
    ]
    for m in stage2_fixtures:
        name = m["name"]
        n = p2_fx_catch.get(name, 0)
        results_by_pr = p2_lookup[name]
        best_r = max(results_by_pr.values(), key=lambda r: r["bpe_score"], default={})
        max_bpe = best_r.get("bpe_score", 0.0)
        tok = best_r.get("top_token", "?")
        llr = best_r.get("top_llr", 0.0)
        catch_rate = n / len(pr_nums)

        lines += [
            f"#### {name} ({n}/{len(pr_nums)} hosts)",
            "",
            f"Pattern: {m['description']}",
            f"Max bpe_score: {max_bpe:.4f} | top_token: `{tok}` | top_llr: {llr:.4f}",
            "",
        ]

        if catch_rate == 0.0:
            lines += [
                "**0/4 — NEVER fires via Stage 2.**",
                f"Max bpe_score {max_bpe:.4f} did not clear any host threshold.",
                f"Top LLR token `{tok}` (llr={llr:.4f}): this token is apparently well-represented",
                "in model_A, so the LLR is insufficient for flagging.",
                "Diagnosis: BPE cannot distinguish this in-repo syntax pattern from baseline code.",
                "",
            ]
        elif catch_rate < 1.0:
            lines += [
                f"**Inconsistent: {n}/{len(pr_nums)} hosts.**",
                "Score vs threshold per host:",
                "",
                "| host PR | bpe_score | threshold | margin |",
                "|---|---|---|---|",
            ]
            for pn in pr_nums:
                r = results_by_pr.get(pn, {})
                bpe = r.get("bpe_score", 0.0)
                pr_thr = next(p["threshold"] for p in host_prs if p["pr_number"] == pn)
                margin = bpe - pr_thr
                lines.append(f"| #{pn} | {bpe:.4f} | {pr_thr:.4f} | {margin:+.4f} |")
            lines.append("")
        else:
            lines += [
                "**4/4 — fires consistently across all host PRs.**",
                "",
            ]

    lines += [
        "---",
        "",
        "## §3 Verdict",
        "",
        "| metric | value |",
        "|---|---|",
        f"| Phase 1 catch rate (catalog breaks, Stage 2 only) | {p1_rate:.1%} |",
        f"| Phase 1 gate (≥50%) | {'PASS' if p1_rate >= 0.50 else 'FAIL'} |",
        f"| Phase 2 catch rate (stage2-only fixtures) | {p2_rate:.1%} |",
        f"| Phase 2 gate (≥70%) | {'PASS' if p2_rate >= 0.70 else 'FAIL'} |",
        "",
        f"**{verdict}**",
        "",
    ]

    if p1_rate < 0.50:
        lines += [
            "Phase 1 <50%: Stage 2 is overtuned. The BPE threshold has been raised so high",
            "that even fixtures with genuine foreign-vocabulary tokens no longer clear it.",
            "Recommend: identify which fix (fix3/4/5/6) introduced the recall collapse by",
            "re-running Phase 1 against those scorers.",
            "",
        ]
    elif p2_rate >= 0.70:
        lines += [
            "Both phases pass. Stage 2 is functional:",
            "- It retains signal on catalog breaks even when Stage 1 is bypassed.",
            "- It catches in-repo paradigm shifts (syntax/style patterns) above the 70% gate.",
            "V0 is unblocked on the Stage-2 recall dimension.",
            "",
        ]
    else:
        lines += [
            "Phase 1 ≥50% but Phase 2 <70%: Stage 2 catches foreign-vocabulary breaks",
            "(where tokens are OOV in both model_A and the generic corpus) but struggles",
            "with pure in-repo syntax shifts that use common tokens (walrus :=, match/case,",
            "union | syntax).  This is a known architectural limit, not a regression.",
            "Document as: Stage 2 is a foreign-vocabulary detector, not a syntax-shift detector.",
            "",
        ]

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written → {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Stage-2 Recall Isolation Probe", flush=True)

    print("\nLoading shared tokenizer...", flush=True)
    from transformers import AutoTokenizer  # type: ignore[import-untyped]

    tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
    print("Tokenizer loaded.", flush=True)

    break_fixtures = _load_break_fixtures()
    print(f"\nLoaded {len(break_fixtures)} break fixtures from FastAPI catalog", flush=True)

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
            }

    missing = set(_HOST_PR_NUMS) - set(host_pr_meta.keys())
    if missing:
        print(f"  WARN: PRs not found in fix6 JSONL: {missing}", flush=True)

    phase1_results: dict[int, list[dict[str, Any]]] = {}
    phase2_results: dict[int, list[dict[str, Any]]] = {}
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

        host_pr_records.append({"pr_number": pr_num, "pre_pr_sha": pre_sha, "threshold": threshold})
        print(f"  Stage2OnlyScorer threshold={threshold:.4f}", flush=True)

        # Phase 1: catalog break fixtures
        p1_results: list[dict[str, Any]] = []
        p1_flagged = 0
        print(f"  Phase 1: scoring {len(break_fixtures)} catalog fixtures ...", flush=True)
        for f in break_fixtures:
            r = _score_catalog_fixture(scorer, f)
            p1_results.append(r)
            if r["flagged"]:
                p1_flagged += 1
        phase1_results[pr_num] = p1_results
        print(
            f"  -> Phase 1: {p1_flagged}/{len(break_fixtures)} flagged"
            f" ({p1_flagged / len(break_fixtures):.0%})",
            flush=True,
        )

        # Phase 2: stage2-only fixtures
        p2_results: list[dict[str, Any]] = []
        p2_flagged = 0
        print(f"  Phase 2: scoring {len(_STAGE2_FIXTURE_META)} stage2-only fixtures ...", flush=True)
        for fm in _STAGE2_FIXTURE_META:
            r = _score_stage2_fixture(scorer, fm)
            p2_results.append(r)
            if r["flagged"]:
                p2_flagged += 1
            marker = "YES" if r["flagged"] else "no"
            print(
                f"    {r['fixture_name']}: {marker}"
                f" (bpe={r['bpe_score']:.4f} vs thr={threshold:.4f},"
                f" top_token=`{r['top_token']}`)",
                flush=True,
            )
        phase2_results[pr_num] = p2_results
        print(
            f"  -> Phase 2: {p2_flagged}/{len(_STAGE2_FIXTURE_META)} flagged"
            f" ({p2_flagged / len(_STAGE2_FIXTURE_META):.0%})",
            flush=True,
        )

    p1_total = len(break_fixtures) * len(host_pr_records)
    p1_total_flagged = sum(1 for rs in phase1_results.values() for r in rs if r["flagged"])
    p2_total = len(_STAGE2_FIXTURE_META) * len(host_pr_records)
    p2_total_flagged = sum(1 for rs in phase2_results.values() for r in rs if r["flagged"])

    print(
        f"\nPhase 1: {p1_total_flagged}/{p1_total} pairs flagged"
        f" ({p1_total_flagged / p1_total:.1%})" if p1_total else "\nPhase 1: no results",
        flush=True,
    )
    print(
        f"Phase 2: {p2_total_flagged}/{p2_total} pairs flagged"
        f" ({p2_total_flagged / p2_total:.1%})" if p2_total else "Phase 2: no results",
        flush=True,
    )

    _write_report(
        _DOCS_OUT,
        host_pr_records,
        phase1_results,
        phase2_results,
        break_fixtures,
        _STAGE2_FIXTURE_META,
    )


if __name__ == "__main__":
    main()
