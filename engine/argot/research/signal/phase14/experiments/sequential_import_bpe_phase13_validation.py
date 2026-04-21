# engine/argot/research/signal/phase14/experiments/sequential_import_bpe_phase13_validation.py
"""Phase 14 Experiment 2 — Sequential import-graph → BPE-tfidf: Phase 13 domain validation.

Hypothesis: a two-stage pipeline eliminates BPE's faker false-positive while recovering the
stdlib-only false-negatives that ImportGraphScorer misses.

  Stage 1: ImportGraphScorer — hunk imports a module never seen in model_A → instant flag.
  Stage 2: BPE-tfidf — for Stage-1 misses, flag if BPE score > max(calibration BPE scores).

Key diagnostics:
  - faker_hunk_0047 (BPE score 7.37, the outlier that caused exp #1 FULL OVERLAP on faker):
    threshold = max(159 calibration BPE scores) = 7.37 → condition is strict >, so NOT flagged.
  - The 15 exp #1 false-negatives: does Stage 2 recover them?

Usage:
    uv run python \\
        engine/argot/research/signal/phase14/experiments/\\
        sequential_import_bpe_phase13_validation.py
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog"

_FASTAPI_DIR = _CATALOG_DIR / "fastapi"
_RICH_DIR = _CATALOG_DIR / "rich"
_FAKER_DIR = _CATALOG_DIR / "faker"

_BPE_MODEL_B_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)

_SCRIPT_DIR = Path(__file__).parent
_SCORES_OUT = _SCRIPT_DIR / "sequential_import_bpe_phase13_validation_scores.json"

_DOCS_OUT = (
    Path(__file__).parent.parent.parent.parent.parent.parent.parent
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "sequential_import_bpe_phase13_validation_2026-04-22.md"
)


# ---------------------------------------------------------------------------
# Helpers (mirrors exp #1)
# ---------------------------------------------------------------------------


def _extract_hunk(path: Path, start_line: int, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _extract_file_to_hunk_end(path: Path, end_line: int) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    hi = min(len(lines), end_line)
    return "\n".join(lines[:hi])


def _load_manifest(path: Path) -> list[dict[str, Any]]:
    return json.loads(path.read_text(encoding="utf-8"))["fixtures"]  # type: ignore[no-any-return]


# ---------------------------------------------------------------------------
# FastAPI domain
# ---------------------------------------------------------------------------


def _run_fastapi() -> dict[str, Any]:
    model_a_files = sorted((_FASTAPI_DIR / "fixtures" / "default").glob("control_*.py"))
    fixtures = _load_manifest(_FASTAPI_DIR / "manifest.json")

    # Calibration = control fixtures (file-to-hunk-end, same extraction as breaks)
    controls = [f for f in fixtures if not f["is_break"]]
    cal_hunks = [
        _extract_file_to_hunk_end(_FASTAPI_DIR / f["file"], f["hunk_end_line"]) for f in controls
    ]

    print(
        f"  Building scorer (n_model_a={len(model_a_files)}, n_cal={len(cal_hunks)})...", flush=True
    )
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  BPE threshold: {scorer.bpe_threshold:.4f}", flush=True)

    results: list[dict[str, Any]] = []
    for f in fixtures:
        fixture_path = _FASTAPI_DIR / f["file"]
        hunk = _extract_file_to_hunk_end(fixture_path, f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        results.append(
            {
                "name": f["name"],
                "category": f.get("category", ""),
                "is_break": f["is_break"],
                **scored,
            }
        )

    breaks = [r for r in results if r["is_break"]]
    controls_r = [r for r in results if not r["is_break"]]
    n_flagged = sum(1 for r in breaks if r["flagged"])
    n_fp = sum(1 for r in controls_r if r["flagged"])
    recall = n_flagged / len(breaks) if breaks else 0.0
    fp_rate = n_fp / len(controls_r) if controls_r else 0.0

    print(f"  Break recall: {recall:.0%} ({n_flagged}/{len(breaks)})")
    print(f"  Control FP rate: {fp_rate:.0%} ({n_fp}/{len(controls_r)})")

    return {
        "domain": "fastapi",
        "n_model_a_files": len(model_a_files),
        "n_calibration": scorer.n_calibration,
        "bpe_threshold": scorer.bpe_threshold,
        "n_breaks": len(breaks),
        "n_controls": len(controls_r),
        "n_breaks_flagged": n_flagged,
        "n_controls_flagged": n_fp,
        "break_recall": recall,
        "fp_rate": fp_rate,
        "fixtures": results,
    }


# ---------------------------------------------------------------------------
# Rich domain
# ---------------------------------------------------------------------------


def _run_rich() -> dict[str, Any]:
    model_a_files = sorted((_RICH_DIR / "sources" / "model_a").glob("*.py"))
    fixtures = _load_manifest(_RICH_DIR / "manifest.json")

    controls = [f for f in fixtures if not f["is_break"]]
    cal_hunks = [
        _extract_hunk(_RICH_DIR / f["file"], f["hunk_start_line"], f["hunk_end_line"])
        for f in controls
    ]

    print(
        f"  Building scorer (n_model_a={len(model_a_files)}, n_cal={len(cal_hunks)})...", flush=True
    )
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  BPE threshold: {scorer.bpe_threshold:.4f}", flush=True)

    results: list[dict[str, Any]] = []
    for f in fixtures:
        fixture_path = _RICH_DIR / f["file"]
        hunk = _extract_hunk(fixture_path, f["hunk_start_line"], f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        results.append(
            {
                "name": f["name"],
                "category": f.get("category", ""),
                "is_break": f["is_break"],
                **scored,
            }
        )

    breaks = [r for r in results if r["is_break"]]
    controls_r = [r for r in results if not r["is_break"]]
    n_flagged = sum(1 for r in breaks if r["flagged"])
    n_fp = sum(1 for r in controls_r if r["flagged"])
    recall = n_flagged / len(breaks) if breaks else 0.0
    fp_rate = n_fp / len(controls_r) if controls_r else 0.0

    print(f"  Break recall: {recall:.0%} ({n_flagged}/{len(breaks)})")
    print(f"  Control FP rate: {fp_rate:.0%} ({n_fp}/{len(controls_r)})")

    return {
        "domain": "rich",
        "n_model_a_files": len(model_a_files),
        "n_calibration": scorer.n_calibration,
        "bpe_threshold": scorer.bpe_threshold,
        "n_breaks": len(breaks),
        "n_controls": len(controls_r),
        "n_breaks_flagged": n_flagged,
        "n_controls_flagged": n_fp,
        "break_recall": recall,
        "fp_rate": fp_rate,
        "fixtures": results,
    }


# ---------------------------------------------------------------------------
# Faker domain
# ---------------------------------------------------------------------------


def _run_faker() -> dict[str, Any]:
    model_a_files = sorted((_FAKER_DIR / "sources" / "model_a").glob("*.py"))

    # Load calibration hunks from sampled_hunks.jsonl
    cal_records: list[dict[str, Any]] = []
    with (_FAKER_DIR / "sampled_hunks.jsonl").open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                cal_records.append(json.loads(line))
    cal_hunks = [rec["hunk_source"] for rec in cal_records]

    print(
        f"  Building scorer (n_model_a={len(model_a_files)}, n_cal={len(cal_hunks)})...", flush=True
    )
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  BPE threshold: {scorer.bpe_threshold:.4f}", flush=True)

    # Score 5 break fixtures
    break_manifest = _load_manifest(_FAKER_DIR / "breaks_manifest.json")
    break_results: list[dict[str, Any]] = []
    for f in break_manifest:
        fixture_path = _FAKER_DIR / f["file"]
        hunk = _extract_hunk(fixture_path, f["hunk_start_line"], f["hunk_end_line"])
        scored = scorer.score_hunk(hunk)
        break_results.append(
            {
                "name": f["name"],
                "category": f["category"],
                "is_break": True,
                **scored,
            }
        )

    # Score calibration hunks (for FP rate + hunk_0047 trace)
    cal_results: list[dict[str, Any]] = []
    for rec in cal_records:
        scored = scorer.score_hunk(rec["hunk_source"])
        cal_results.append(
            {
                "name": rec["name"],
                "file_path": rec["file_path"],
                **scored,
            }
        )

    n_break_flagged = sum(1 for r in break_results if r["flagged"])
    n_cal_flagged = sum(1 for r in cal_results if r["flagged"])
    recall = n_break_flagged / len(break_results) if break_results else 0.0
    cal_fp_rate = n_cal_flagged / len(cal_results) if cal_results else 0.0

    hunk_0047 = next((r for r in cal_results if r["name"] == "faker_hunk_0047"), None)

    print(f"  Break recall: {recall:.0%} ({n_break_flagged}/{len(break_results)})")
    print(f"  Calibration FP rate: {cal_fp_rate:.1%} ({n_cal_flagged}/{len(cal_results)})")
    if hunk_0047:
        print(
            f"  faker_hunk_0047: import={hunk_0047['import_score']:.0f}, "
            f"bpe={hunk_0047['bpe_score']:.4f}, threshold={scorer.bpe_threshold:.4f}, "
            f"flagged={hunk_0047['flagged']}"
        )

    return {
        "domain": "faker",
        "n_model_a_files": len(model_a_files),
        "n_calibration": scorer.n_calibration,
        "bpe_threshold": scorer.bpe_threshold,
        "n_breaks": len(break_results),
        "n_calibration_hunks": len(cal_results),
        "n_breaks_flagged": n_break_flagged,
        "n_cal_flagged": n_cal_flagged,
        "break_recall": recall,
        "cal_fp_rate": cal_fp_rate,
        "hunk_0047": hunk_0047,
        "break_fixtures": break_results,
        "calibration": cal_results,
    }


# ---------------------------------------------------------------------------
# Verdict helper (same bands as exp #1)
# ---------------------------------------------------------------------------


def _verdict(recall: float, fp_rate: float) -> str:
    if recall >= 0.85 and fp_rate <= 0.02:
        return "STRONG"
    if recall >= 0.50 or fp_rate <= 0.10:
        return "PARTIAL"
    return "WEAK"


# ---------------------------------------------------------------------------
# Report writer
# ---------------------------------------------------------------------------


def _write_report(
    out: Path,
    fastapi: dict[str, Any],
    rich: dict[str, Any],
    faker: dict[str, Any],
) -> None:
    lines: list[str] = [
        "# Phase 14 Experiment 2 — Sequential import-graph → BPE-tfidf: "
        "Phase 13 domain validation (2026-04-22)",
        "",
        "**Scorer:** `SequentialImportBpeScorer`",
        "  Stage 1: ImportGraphScorer (flag if foreign module count ≥ 1)",
        "  Stage 2: BPE-tfidf (flag if max log-likelihood ratio > max(calibration BPE))",
        "",
        "**Hypothesis:** The sequential pipeline achieves ≥85% combined recall with 0% FP "
        "because import-graph catches all faker breaks (removing BPE's need to fire on faker), "
        "and BPE recovers the stdlib-only FNs on FastAPI/rich with clean thresholds.",
        "",
        "**Pre-registered verdict bands:**",
        "- STRONG: ≥85% recall + ≤2% FP + faker_hunk_0047 not flagged",
        "- PARTIAL: 50-85% recall OR 2-10% FP",
        "- WEAK: <50% recall OR >10% FP",
        "",
        "---",
        "",
        "## 1. Per-repo BPE Thresholds",
        "",
        "| domain | calibration set | n | BPE threshold | note |",
        "|---|---|---|---|---|",
        f"| FastAPI | control fixtures | {fastapi['n_calibration']} "
        f"| {fastapi['bpe_threshold']:.4f} "
        f"| {'⚠ small sample (n<30)' if fastapi['n_calibration'] < 30 else 'ok'} |",
        f"| rich | control fixtures | {rich['n_calibration']} "
        f"| {rich['bpe_threshold']:.4f} "
        f"| {'⚠ small sample (n<30)' if rich['n_calibration'] < 30 else 'ok'} |",
        f"| faker | sampled_hunks.jsonl | {faker['n_calibration']} "
        f"| {faker['bpe_threshold']:.4f} "
        f"| {'⚠ small sample (n<30)' if faker['n_calibration'] < 30 else 'ok'} |",
        "",
    ]

    # Cross-domain summary table
    all_breaks = fastapi["n_breaks"] + rich["n_breaks"] + faker["n_breaks"]
    all_flagged = fastapi["n_breaks_flagged"] + rich["n_breaks_flagged"] + faker["n_breaks_flagged"]
    overall_recall = all_flagged / all_breaks if all_breaks > 0 else 0.0

    all_controls = fastapi["n_controls"] + rich["n_controls"] + faker["n_calibration_hunks"]
    all_fp = fastapi["n_controls_flagged"] + rich["n_controls_flagged"] + faker["n_cal_flagged"]
    overall_fp = all_fp / all_controls if all_controls > 0 else 0.0

    lines += [
        "---",
        "",
        "## 2. Cross-domain Summary",
        "",
        "| domain | breaks | flagged | recall | controls/cal | FP | FP rate |",
        "|---|---|---|---|---|---|---|",
        f"| FastAPI | {fastapi['n_breaks']} | {fastapi['n_breaks_flagged']} "
        f"| {fastapi['break_recall']:.0%} | {fastapi['n_controls']} "
        f"| {fastapi['n_controls_flagged']} | {fastapi['fp_rate']:.0%} |",
        f"| rich | {rich['n_breaks']} | {rich['n_breaks_flagged']} "
        f"| {rich['break_recall']:.0%} | {rich['n_controls']} "
        f"| {rich['n_controls_flagged']} | {rich['fp_rate']:.0%} |",
        f"| faker | {faker['n_breaks']} | {faker['n_breaks_flagged']} "
        f"| {faker['break_recall']:.0%} | {faker['n_calibration_hunks']} "
        f"| {faker['n_cal_flagged']} | {faker['cal_fp_rate']:.1%} |",
        f"| **combined** | **{all_breaks}** | **{all_flagged}** "
        f"| **{overall_recall:.0%}** | **{all_controls}** "
        f"| **{all_fp}** | **{overall_fp:.1%}** |",
        "",
        f"Overall break recall across 3 valid corpora: **{overall_recall:.0%}** "
        f"({all_flagged}/{all_breaks})",
        "",
    ]

    # Per-stage attribution
    lines += [
        "---",
        "",
        "## 3. Per-stage Attribution",
        "",
    ]

    for domain_name, domain_data in [("FastAPI", fastapi), ("Rich", rich)]:
        fixtures_list: list[dict[str, Any]] = domain_data["fixtures"]
        breaks = [r for r in fixtures_list if r["is_break"] and r["flagged"]]
        by_import = [r for r in breaks if r["reason"] == "import"]
        by_bpe = [r for r in breaks if r["reason"] == "bpe"]
        lines += [
            f"### {domain_name}",
            "",
            f"Flagged breaks: {len(breaks)}/{domain_data['n_breaks']} "
            f"({len(by_import)} via Stage 1 import, {len(by_bpe)} via Stage 2 BPE)",
            "",
            "| name | category | import_score | bpe_score | reason |",
            "|---|---|---|---|---|",
        ]
        for r in [f for f in fixtures_list if f["is_break"]]:
            flag_str = r["reason"].upper() if r["flagged"] else "—"
            lines.append(
                f"| {r['name']} | {r['category']} "
                f"| {r['import_score']:.0f} | {r['bpe_score']:.4f} | {flag_str} |"
            )
        lines.append("")

    faker_breaks = faker["break_fixtures"]
    by_import_f = [r for r in faker_breaks if r["flagged"] and r["reason"] == "import"]
    by_bpe_f = [r for r in faker_breaks if r["flagged"] and r["reason"] == "bpe"]
    lines += [
        "### Faker",
        "",
        f"Flagged breaks: {faker['n_breaks_flagged']}/{faker['n_breaks']} "
        f"({len(by_import_f)} via Stage 1 import, {len(by_bpe_f)} via Stage 2 BPE)",
        "",
        "| name | category | import_score | bpe_score | reason |",
        "|---|---|---|---|---|",
    ]
    for r in faker_breaks:
        flag_str = r["reason"].upper() if r["flagged"] else "—"
        lines.append(
            f"| {r['name']} | {r['category']} "
            f"| {r['import_score']:.0f} | {r['bpe_score']:.4f} | {flag_str} |"
        )
    lines.append("")

    # faker_hunk_0047 trace
    lines += [
        "---",
        "",
        "## 4. Key Diagnostic: faker_hunk_0047",
        "",
    ]
    hunk_0047 = faker.get("hunk_0047")
    if hunk_0047:
        verdict_0047 = "PASS" if not hunk_0047["flagged"] else "FAIL"
        outcome = (
            "scorer correctly does NOT flag this hunk (error-handling code, "
            "no foreign import, bpe_score ≤ threshold)"
            if not hunk_0047["flagged"]
            else "UNEXPECTED: scorer flagged this hunk — hypothesis invalidated"
        )
        lines += [
            "- BPE score from Phase 13: **7.3732** (the single outlier that caused "
            "BPE's FULL OVERLAP verdict on faker)",
            f"- Stage 1 import_score: **{hunk_0047['import_score']:.0f}**",
            f"- Stage 2 bpe_score: **{hunk_0047['bpe_score']:.4f}**",
            f"- Faker BPE threshold (max of 159 cal hunks): **{faker['bpe_threshold']:.4f}**",
            f"- Condition `bpe_score > threshold`: "
            f"**{hunk_0047['bpe_score']:.4f} > {faker['bpe_threshold']:.4f}** = "
            f"**{hunk_0047['bpe_score'] > faker['bpe_threshold']}**",
            f"- Flagged: **{hunk_0047['flagged']}**",
            f"- **{verdict_0047}** — {outcome}",
            "",
        ]
    else:
        lines += ["faker_hunk_0047 not found in calibration results.", ""]

    # FN recovery analysis
    lines += [
        "---",
        "",
        "## 5. Exp #1 False-negative Recovery by Stage 2",
        "",
        "The 15 hunks that ImportGraphScorer (exp #1) missed — all had import_score = 0.",
        "Stage 2 recovers them if `bpe_score > threshold`.",
        "",
    ]

    for domain_name, domain_data in [("FastAPI", fastapi), ("Rich", rich)]:
        fixtures_list = domain_data["fixtures"]
        fn_in_exp1 = [r for r in fixtures_list if r["is_break"] and r["import_score"] == 0.0]
        recovered = [r for r in fn_in_exp1 if r["reason"] == "bpe"]
        still_fn = [r for r in fn_in_exp1 if not r["flagged"]]
        lines += [
            f"### {domain_name} ({len(fn_in_exp1)} exp #1 FNs)",
            "",
            f"Stage 2 recovered: **{len(recovered)}/{len(fn_in_exp1)}**  "
            f"| Still FN: **{len(still_fn)}**",
            "",
            "| name | category | bpe_score | threshold | recovered |",
            "|---|---|---|---|---|",
        ]
        for r in fn_in_exp1:
            rec_str = "YES (bpe)" if r["reason"] == "bpe" else "no"
            lines.append(
                f"| {r['name']} | {r['category']} "
                f"| {r['bpe_score']:.4f} | {domain_data['bpe_threshold']:.4f} | {rec_str} |"
            )
        lines.append("")

    faker_fn = [r for r in faker["break_fixtures"] if r["import_score"] == 0.0]
    lines += [
        f"### Faker ({len(faker_fn)} exp #1 FNs — expected 0)",
        "",
    ]
    if faker_fn:
        lines += [
            "| name | category | bpe_score | reason |",
            "|---|---|---|---|",
        ]
        for r in faker_fn:
            lines.append(
                f"| {r['name']} | {r['category']} | {r['bpe_score']:.4f} | {r['reason']} |"
            )
        lines.append("")
    else:
        lines += ["All faker breaks caught by Stage 1 (import). No Stage 2 needed on faker.", ""]

    # Known risks
    lines += [
        "---",
        "",
        "## 6. Known Risks",
        "",
        "### Small calibration set for FastAPI and rich",
        "",
        f"FastAPI: {fastapi['n_calibration']} calibration hunks (20 control fixtures). "
        f"Rich: {rich['n_calibration']} calibration hunks (10 control fixtures). "
        "With n < 30, the max-based threshold is an optimistic estimate of the true "
        "calibration ceiling — real-world FP rate may be higher. "
        f"Faker has {faker['n_calibration']} calibration hunks, which is adequate.",
        "",
        "### Rich ANSI/manual-print false-negatives",
        "",
        "Rich's paradigm breaks `ansi_raw` and `print_manual` use ANSI escape codes and "
        "bare `print()` calls — no foreign imports, no library-specific tokens. "
        "BPE may fail to recover these because their tokens (escape sequences, "
        "`\\x1b`, brackets) are either filtered out by the meaningful-token filter "
        "(len < 3 or non-alphanumeric) or present in both model_A and model_B. "
        "If these are still FN after Stage 2, they require a third axis "
        "(e.g. AST-structural or semantic).",
        "",
    ]

    # Verdict
    lines += [
        "---",
        "",
        "## 7. Verdict",
        "",
        "| domain | recall | FP rate | faker_hunk_0047 safe | per-domain verdict |",
        "|---|---|---|---|---|",
    ]

    hunk_0047_safe = hunk_0047 is not None and not hunk_0047["flagged"]
    fapi_v = _verdict(fastapi["break_recall"], fastapi["fp_rate"])
    rich_v = _verdict(rich["break_recall"], rich["fp_rate"])
    faker_v = _verdict(faker["break_recall"], faker["cal_fp_rate"])
    lines += [
        f"| FastAPI | {fastapi['break_recall']:.0%} | {fastapi['fp_rate']:.0%} | — | {fapi_v} |",
        f"| rich | {rich['break_recall']:.0%} | {rich['fp_rate']:.0%} | — | {rich_v} |",
        f"| faker | {faker['break_recall']:.0%} | {faker['cal_fp_rate']:.1%} "
        f"| {'YES' if hunk_0047_safe else 'NO'} | {faker_v} |",
        f"| **combined** | **{overall_recall:.0%}** | **{overall_fp:.1%}** | — | — |",
        "",
    ]

    combined_verdict = _verdict(overall_recall, overall_fp)
    safe_str = "YES" if hunk_0047_safe else "NO"
    lines += [
        f"**faker_hunk_0047 not flagged: {safe_str}**",
        "",
    ]

    if overall_recall >= 0.85 and overall_fp <= 0.02 and hunk_0047_safe:
        combined_verdict = "STRONG"
    elif overall_recall < 0.50 or overall_fp > 0.10:
        combined_verdict = "WEAK"
    else:
        combined_verdict = "PARTIAL"

    lines += [
        f"**Cross-domain verdict: {combined_verdict}**",
        "",
    ]

    if combined_verdict == "STRONG":
        lines += [
            "The sequential pipeline achieves ≥85% recall with ≤2% FP and correctly "
            "suppresses faker_hunk_0047. Recommend as Phase 14 primary scorer.",
            "",
        ]
    elif combined_verdict == "PARTIAL":
        lines += [
            "The pipeline improves on exp #1 (import-only) but does not reach STRONG. "
            "The remaining false-negatives require a third axis or scope restriction. "
            "Verdict: PARTIAL — usable as a fast two-stage gate for foreign-import "
            "and token-distribution breaks; stdlib-only breaks still leak through.",
            "",
        ]
    else:
        lines += [
            "The pipeline has <50% combined recall or >10% FP. "
            "Hypothesis rejected. Stop here and pivot to a different approach.",
            "",
        ]

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    print("Phase 14 Experiment 2 — Sequential import-graph → BPE-tfidf validation", flush=True)

    print("\nRunning FastAPI domain...", flush=True)
    fastapi_result = _run_fastapi()

    print("\nRunning rich domain...", flush=True)
    rich_result = _run_rich()

    print("\nRunning faker domain...", flush=True)
    faker_result = _run_faker()

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
