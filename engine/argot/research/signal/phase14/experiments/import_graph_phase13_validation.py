# engine/argot/research/signal/phase14/experiments/import_graph_phase13_validation.py
"""Phase 14 Experiment 1 — Import-graph foreign-module scorer: Phase 13 domain validation.

Runs ImportGraphScorer on all three Phase 13 domains (FastAPI, rich, faker) and
emits a consolidated cross-domain report.

Per-domain:
  - FastAPI: model_A = control_*.py fixtures; scored against all manifest fixtures
  - Rich:    model_A = sources/model_a/*.py; scored against all manifest fixtures
  - Faker:   model_A = sources/model_a/*.py; scored against 5 break fixtures +
             159 calibration hunks (sampled_hunks.jsonl)

Key diagnostic: confirm that faker_hunk_0047 (BPE false-positive: error-handling code,
max BPE score 7.37) is NOT flagged by the import scorer — this validates the hypothesis
that the scorer filters BPE's false-positive pattern.

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/import_graph_phase13_validation.py
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from argot.research.signal.phase14.scorers.import_graph_scorer import ImportGraphScorer

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog"

_FASTAPI_DIR = _CATALOG_DIR / "fastapi"
_RICH_DIR = _CATALOG_DIR / "rich"
_FAKER_DIR = _CATALOG_DIR / "faker"

_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "import_graph_phase13_validation_scores.json"

_DOCS_OUT = (
    Path(__file__).parent.parent.parent.parent.parent.parent.parent
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "import_graph_phase13_validation_2026-04-22.md"
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _extract_file_to_hunk_end(path: Path, end_line: int) -> str:
    """Extract from file start through end_line (inclusive, 1-indexed).

    Used for FastAPI fixtures where imports are at the file top, outside the
    hunk boundary.  Scoring from file start ensures import lines are included.
    """
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    hi = min(len(lines), end_line)
    return "\n".join(lines[:hi])


def _load_manifest(path: Path) -> list[dict[str, Any]]:
    return json.loads(path.read_text(encoding="utf-8"))["fixtures"]  # type: ignore[no-any-return]


# ---------------------------------------------------------------------------
# FastAPI domain
# ---------------------------------------------------------------------------


def _run_fastapi(scorer: ImportGraphScorer) -> dict[str, Any]:
    model_a_files = sorted((_FASTAPI_DIR / "fixtures" / "default").glob("control_*.py"))
    scorer.fit(model_a_files)

    fixtures = _load_manifest(_FASTAPI_DIR / "manifest.json")
    results: list[dict[str, Any]] = []
    for f in fixtures:
        fixture_path = _FASTAPI_DIR / f["file"]
        # FastAPI fixture imports are at the file top (before hunk_start_line), so
        # score from file start through hunk_end_line to capture the import lines.
        hunk = _extract_file_to_hunk_end(fixture_path, f["hunk_end_line"])
        score = scorer.score_hunk(hunk)
        flagged = score >= 1.0
        results.append(
            {
                "name": f["name"],
                "category": f.get("category", ""),
                "is_break": f["is_break"],
                "score": score,
                "flagged": flagged,
            }
        )

    breaks = [r for r in results if r["is_break"]]
    controls = [r for r in results if not r["is_break"]]
    break_recall = sum(1 for r in breaks if r["flagged"]) / len(breaks) if breaks else 0.0
    control_precision = (
        sum(1 for r in controls if not r["flagged"]) / len(controls) if controls else 0.0
    )

    n_flagged = sum(1 for r in breaks if r["flagged"])
    n_clean = sum(1 for r in controls if not r["flagged"])
    print(f"\n=== FastAPI ({len(breaks)} breaks, {len(controls)} controls) ===")
    print(f"  Break recall: {break_recall:.0%}  ({n_flagged}/{len(breaks)})")
    print(f"  Control precision: {control_precision:.0%}  ({n_clean}/{len(controls)})")

    return {
        "domain": "fastapi",
        "n_breaks": len(breaks),
        "n_controls": len(controls),
        "n_breaks_flagged": sum(1 for r in breaks if r["flagged"]),
        "n_controls_flagged": sum(1 for r in controls if r["flagged"]),
        "break_recall": break_recall,
        "control_precision": control_precision,
        "fixtures": results,
    }


# ---------------------------------------------------------------------------
# Rich domain
# ---------------------------------------------------------------------------


def _run_rich(scorer: ImportGraphScorer) -> dict[str, Any]:
    model_a_files = sorted((_RICH_DIR / "sources" / "model_a").glob("*.py"))
    scorer.fit(model_a_files)

    fixtures = _load_manifest(_RICH_DIR / "manifest.json")
    results: list[dict[str, Any]] = []
    for f in fixtures:
        fixture_path = _RICH_DIR / f["file"]
        hunk = _extract_hunk(fixture_path, f["hunk_start_line"], f["hunk_end_line"])
        score = scorer.score_hunk(hunk)
        flagged = score >= 1.0
        results.append(
            {
                "name": f["name"],
                "category": f.get("category", ""),
                "is_break": f["is_break"],
                "score": score,
                "flagged": flagged,
            }
        )

    breaks = [r for r in results if r["is_break"]]
    controls = [r for r in results if not r["is_break"]]
    break_recall = sum(1 for r in breaks if r["flagged"]) / len(breaks) if breaks else 0.0
    control_precision = (
        sum(1 for r in controls if not r["flagged"]) / len(controls) if controls else 0.0
    )

    n_flagged = sum(1 for r in breaks if r["flagged"])
    n_clean = sum(1 for r in controls if not r["flagged"])
    print(f"\n=== Rich ({len(breaks)} breaks, {len(controls)} controls) ===")
    print(f"  Break recall: {break_recall:.0%}  ({n_flagged}/{len(breaks)})")
    print(f"  Control precision: {control_precision:.0%}  ({n_clean}/{len(controls)})")

    return {
        "domain": "rich",
        "n_breaks": len(breaks),
        "n_controls": len(controls),
        "n_breaks_flagged": sum(1 for r in breaks if r["flagged"]),
        "n_controls_flagged": sum(1 for r in controls if r["flagged"]),
        "break_recall": break_recall,
        "control_precision": control_precision,
        "fixtures": results,
    }


# ---------------------------------------------------------------------------
# Faker domain
# ---------------------------------------------------------------------------


def _run_faker(scorer: ImportGraphScorer) -> dict[str, Any]:
    model_a_files = sorted((_FAKER_DIR / "sources" / "model_a").glob("*.py"))
    scorer.fit(model_a_files)

    # Score 5 break fixtures
    break_manifest = _load_manifest(_FAKER_DIR / "breaks_manifest.json")
    break_results: list[dict[str, Any]] = []
    for f in break_manifest:
        fixture_path = _FAKER_DIR / f["file"]
        hunk = _extract_hunk(fixture_path, f["hunk_start_line"], f["hunk_end_line"])
        score = scorer.score_hunk(hunk)
        flagged = score >= 1.0
        break_results.append(
            {
                "name": f["name"],
                "category": f["category"],
                "is_break": True,
                "score": score,
                "flagged": flagged,
            }
        )

    # Score 159 calibration hunks (the FP diagnostic)
    cal_records: list[dict[str, Any]] = []
    with (_FAKER_DIR / "sampled_hunks.jsonl").open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                cal_records.append(json.loads(line))

    cal_results: list[dict[str, Any]] = []
    for rec in cal_records:
        score = scorer.score_hunk(rec["hunk_source"])
        flagged = score >= 1.0
        cal_results.append(
            {
                "name": rec["name"],
                "file_path": rec["file_path"],
                "score": score,
                "flagged": flagged,
            }
        )

    # Key diagnostic: is faker_hunk_0047 flagged?
    hunk_0047 = next((r for r in cal_results if r["name"] == "faker_hunk_0047"), None)

    break_recall = sum(1 for r in break_results if r["flagged"]) / len(break_results)
    cal_fp_rate = sum(1 for r in cal_results if r["flagged"]) / len(cal_results)

    n_break_flagged = sum(1 for r in break_results if r["flagged"])
    n_cal_flagged = sum(1 for r in cal_results if r["flagged"])
    print(f"\n=== Faker ({len(break_results)} breaks, {len(cal_results)} calibration hunks) ===")
    print(f"  Break recall: {break_recall:.0%}  ({n_break_flagged}/{len(break_results)})")
    print(f"  Calibration FP rate: {cal_fp_rate:.1%}  ({n_cal_flagged}/{len(cal_results)})")
    if hunk_0047:
        h47_score = hunk_0047["score"]
        h47_flagged = hunk_0047["flagged"]
        print(f"  faker_hunk_0047 (BPE outlier): score={h47_score:.0f}, flagged={h47_flagged}")

    return {
        "domain": "faker",
        "n_breaks": len(break_results),
        "n_calibration": len(cal_results),
        "n_breaks_flagged": sum(1 for r in break_results if r["flagged"]),
        "n_cal_flagged": sum(1 for r in cal_results if r["flagged"]),
        "break_recall": break_recall,
        "cal_fp_rate": cal_fp_rate,
        "hunk_0047_flagged": hunk_0047["flagged"] if hunk_0047 else None,
        "break_fixtures": break_results,
        "calibration": cal_results,
    }


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _verdict(recall: float, fp_rate: float, *, prefix: str = "") -> str:
    if recall >= 0.85 and fp_rate <= 0.02:
        return f"{prefix}STRONG"
    if recall >= 0.50 or fp_rate <= 0.10:
        return f"{prefix}PARTIAL"
    return f"{prefix}WEAK"


def _write_report(
    out: Path,
    fastapi: dict[str, Any],
    rich: dict[str, Any],
    faker: dict[str, Any],
) -> None:
    lines: list[str] = [
        "# Phase 14 Experiment 1 — Import-graph foreign-module scorer: "  # noqa: E501
        "Phase 13 domain validation (2026-04-22)",
        "",
        "**Scorer:** `ImportGraphScorer` — counts top-level modules in hunk"
        " that were never seen in model_A",
        "**Domains:** FastAPI (control fixtures as model_A), rich (72 source files),"
        " faker (722 source files)",
        "**Key diagnostic:** Does the scorer filter BPE's false-positive outlier"
        " `faker_hunk_0047` (error-handling code, BPE score 7.37)?",
        "",
        "---",
        "",
        "## 1. Cross-domain Summary",
        "",
    ]

    # Cross-domain table
    lines += [
        "| domain | model_A files | breaks | breaks flagged | recall"
        " | controls/cal | flagged | FP rate |",
        "|---|---|---|---|---|---|---|---|",
    ]

    fastapi_ma = len(sorted((_FASTAPI_DIR / "fixtures" / "default").glob("control_*.py")))
    rich_ma = len(sorted((_RICH_DIR / "sources" / "model_a").glob("*.py")))
    faker_ma = len(sorted((_FAKER_DIR / "sources" / "model_a").glob("*.py")))

    lines.append(
        f"| FastAPI | {fastapi_ma} | {fastapi['n_breaks']} | {fastapi['n_breaks_flagged']} "
        f"| {fastapi['break_recall']:.0%} | {fastapi['n_controls']} (controls) "
        f"| {fastapi['n_controls_flagged']} | {1 - fastapi['control_precision']:.0%} |"
    )
    lines.append(
        f"| rich | {rich_ma} | {rich['n_breaks']} | {rich['n_breaks_flagged']} "
        f"| {rich['break_recall']:.0%} | {rich['n_controls']} (controls) "
        f"| {rich['n_controls_flagged']} | {1 - rich['control_precision']:.0%} |"
    )
    lines.append(
        f"| faker | {faker_ma} | {faker['n_breaks']} | {faker['n_breaks_flagged']} "
        f"| {faker['break_recall']:.0%} | {faker['n_calibration']} (cal hunks) "
        f"| {faker['n_cal_flagged']} | {faker['cal_fp_rate']:.1%} |"
    )

    # Cross-domain averages
    all_breaks = fastapi["n_breaks"] + rich["n_breaks"] + faker["n_breaks"]
    all_flagged = fastapi["n_breaks_flagged"] + rich["n_breaks_flagged"] + faker["n_breaks_flagged"]
    overall_recall = all_flagged / all_breaks if all_breaks > 0 else 0.0
    lines += [
        "",
        f"Overall break recall across all domains: **{overall_recall:.0%}**"
        f" ({all_flagged}/{all_breaks})",
        "",
    ]

    # ---------------------------------------------------------------------------
    # 2. Per-domain fixture tables
    # ---------------------------------------------------------------------------
    lines += ["---", "", "## 2. Per-domain Fixture Details", ""]

    for domain_name, domain_data in [("FastAPI", fastapi), ("Rich", rich)]:
        lines += [
            f"### {domain_name}",
            "",
            "| name | category | is_break | score | flagged | foreign modules |",
            "|---|---|---|---|---|---|",
        ]
        for r in domain_data["fixtures"]:
            lines.append(
                f"| {r['name']} | {r['category']} | {'break' if r['is_break'] else 'control'} "
                f"| {r['score']:.0f} | {'YES' if r['flagged'] else 'no'} | — |"
            )
        lines.append("")

    lines += [
        "### Faker (break fixtures)",
        "",
        "| name | category | score | flagged |",
        "|---|---|---|---|",
    ]
    for r in faker["break_fixtures"]:
        flag_str = "YES" if r["flagged"] else "no"
        lines.append(f"| {r['name']} | {r['category']} | {r['score']:.0f} | {flag_str} |")
    lines.append("")

    # ---------------------------------------------------------------------------
    # 3. Key diagnostic: faker_hunk_0047
    # ---------------------------------------------------------------------------
    lines += ["---", "", "## 3. Key Diagnostic: faker_hunk_0047 (BPE False-positive)", ""]
    hunk_0047 = next((r for r in faker["calibration"] if r["name"] == "faker_hunk_0047"), None)
    if hunk_0047:
        verdict_0047 = "PASS" if not hunk_0047["flagged"] else "FAIL"
        outcome = (
            "scorer correctly ignores this hunk (error-handling code, no foreign imports)"
            if not hunk_0047["flagged"]
            else "UNEXPECTED: scorer flagged this hunk — hypothesis invalidated"
        )
        lines += [
            "- BPE score (from Phase 13): 7.3732 — the single hunk that caused"
            " BPE's `FULL OVERLAP` verdict on faker",
            f"- Import scorer score: {hunk_0047['score']:.0f}",
            f"- Flagged: {hunk_0047['flagged']}",
            f"- **{verdict_0047}** — {outcome}",
            "",
        ]
    else:
        lines += ["faker_hunk_0047 not found in calibration results.", ""]

    # All flagged calibration hunks
    cal_flagged = [r for r in faker["calibration"] if r["flagged"]]
    if cal_flagged:
        lines += [
            f"Other flagged calibration hunks ({len(cal_flagged)} total):",
            "",
            "| name | file_path | score |",
            "|---|---|---|",
        ]
        for r in cal_flagged:
            lines.append(f"| {r['name']} | {r['file_path']} | {r['score']:.0f} |")
        lines.append("")
    else:
        lines += ["No calibration hunks flagged (FP rate = 0%).", ""]

    # ---------------------------------------------------------------------------
    # 4. False-negatives: stdlib-only breaks
    # ---------------------------------------------------------------------------
    lines += ["---", "", "## 4. False-negatives (stdlib-only or no-import patterns)", ""]

    all_fn: dict[str, list[str]] = {}
    for domain_name, domain_data in [("FastAPI", fastapi), ("Rich", rich)]:
        fn = [r["name"] for r in domain_data["fixtures"] if r["is_break"] and not r["flagged"]]
        if fn:
            all_fn[domain_name] = fn

    faker_fn = [r["name"] for r in faker["break_fixtures"] if not r["flagged"]]
    if faker_fn:
        all_fn["Faker"] = faker_fn

    if all_fn:
        for d, names in all_fn.items():
            lines.append(f"**{d}:** {', '.join(names)}")
        lines.append("")
        lines += [
            "These breaks do not introduce foreign imports — they are either stdlib-only paradigm",
            "violations (e.g. raw ANSI escape codes, `assert` for validation, `raise ValueError`)",
            "or use libraries already present in model_A."
            " A second scorer layer is required to cover them.",
            "",
        ]
    else:
        lines += ["No false-negatives.", ""]

    # ---------------------------------------------------------------------------
    # 5. Verdict
    # ---------------------------------------------------------------------------
    lines += ["---", "", "## 5. Verdict", ""]

    # Compute per-domain verdicts
    fastapi_fp = 1 - fastapi["control_precision"]
    rich_fp = 1 - rich["control_precision"]
    faker_fp = faker["cal_fp_rate"]
    cross_domain_fp = (
        fastapi["n_controls_flagged"] + rich["n_controls_flagged"] + faker["n_cal_flagged"]
    ) / (fastapi["n_controls"] + rich["n_controls"] + faker["n_calibration"])

    fapi_v = _verdict(fastapi["break_recall"], fastapi_fp)
    rich_v = _verdict(rich["break_recall"], rich_fp)
    faker_v = _verdict(faker["break_recall"], faker_fp)
    lines += [
        "| domain | recall | FP rate | per-domain verdict |",
        "|---|---|---|---|",
        f"| FastAPI | {fastapi['break_recall']:.0%} | {fastapi_fp:.0%} | {fapi_v} |",
        f"| rich | {rich['break_recall']:.0%} | {rich_fp:.0%} | {rich_v} |",
        f"| faker | {faker['break_recall']:.0%} | {faker_fp:.1%} | {faker_v} |",
        f"| **combined** | **{overall_recall:.0%}** | **{cross_domain_fp:.1%}** | — |",
        "",
    ]

    combined_verdict = _verdict(overall_recall, cross_domain_fp)
    lines += [f"**Cross-domain verdict: {combined_verdict}**", ""]

    if combined_verdict == "STRONG":
        lines += [
            "The scorer achieves ≥85% recall with ≤2% FP across all domains."
            " Ship as Phase 14 primary scorer; narrow the residual false-negative"
            " gap (stdlib-only patterns) via a second layer.",
            "",
        ]
    elif combined_verdict == "PARTIAL":
        lines += [
            "The scorer has complementary signal — it catches foreign-library breaks"
            " that both BPE-tfidf and AST-contrastive can miss — but cannot be a"
            " standalone primary scorer. The false-negative set (stdlib-only paradigm"
            " breaks) requires a second axis.",
            "",
            "**Recommendation:** use as a fast pre-filter: any hunk with score ≥ 1"
            " is an instant flag (high precision); the remaining hunks (score = 0)"
            " are passed to BPE or AST for deeper scoring.",
        ]
    else:
        lines += [
            "The scorer has <50% recall or >10% FP cross-domain.",
            "Hypothesis rejected. Pivot to CodeBERT zero-shot or a different approach.",
        ]
    lines.append("")

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Experiment 1 — Import-graph scorer: Phase 13 validation", flush=True)

    scorer = ImportGraphScorer()

    print("\nRunning FastAPI domain...", flush=True)
    fastapi_result = _run_fastapi(scorer)

    print("\nRunning rich domain...", flush=True)
    rich_result = _run_rich(scorer)

    print("\nRunning faker domain...", flush=True)
    faker_result = _run_faker(scorer)

    # Save raw scores
    scores: dict[str, Any] = {
        "fastapi": fastapi_result,
        "rich": rich_result,
        "faker": faker_result,
    }
    _SCORES_OUT.write_text(json.dumps(scores, indent=2), encoding="utf-8")
    print(f"\nScores saved to {_SCORES_OUT}", flush=True)

    _write_report(_DOCS_OUT, fastapi_result, rich_result, faker_result)


if __name__ == "__main__":
    main()
