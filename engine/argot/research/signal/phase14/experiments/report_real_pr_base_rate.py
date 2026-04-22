# engine/argot/research/signal/phase14/experiments/report_real_pr_base_rate.py
"""Phase 14 Exp #5 — Real-PR base-rate report.

Reads:  real_pr_base_rate_hunks_2026_04_22.jsonl
Writes: docs/research/scoring/signal/phase14/experiments/real_pr_base_rate_2026-04-22.md

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/report_real_pr_base_rate.py
"""

from __future__ import annotations

import json
from collections import Counter, defaultdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).parent
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_2026_04_22.jsonl"
_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "real_pr_base_rate_2026-04-22.md"
)

_TODAY = datetime(2026, 4, 22, tzinfo=UTC)


def _age_bucket(merged_at: str) -> str:
    dt = datetime.fromisoformat(merged_at.replace("Z", "+00:00"))
    days = (_TODAY - dt).days
    if days <= 90:
        return "≤90 days"
    elif days <= 180:
        return "91–180 days"
    else:
        return "181–365 days"


def _verdict_band(flag_rate: float) -> str:
    if flag_rate < 0.15:
        return "V1 USEFUL"
    elif flag_rate < 0.30:
        return "V1 PLAUSIBLE"
    elif flag_rate < 0.60:
        return "V1 INCONCLUSIVE"
    else:
        return "V1 USELESS"


def _auto_judgment(rec: dict[str, Any]) -> tuple[str, str]:
    """Best-effort automated judgment for Stage 1/2 flags."""
    if rec["reason"] == "import":
        mods = rec.get("foreign_modules") or []
        mod_str = ", ".join(mods) if mods else "unknown"
        return (
            "LIKELY_STYLE_DRIFT",
            f"Stage 1 flagged foreign module(s) never seen in fastapi/ source: {mod_str}.",
        )
    else:
        margin = rec["bpe_score"] - rec["bpe_threshold"]
        if margin > 2.0:
            return (
                "LIKELY_STYLE_DRIFT",
                f"BPE score {rec['bpe_score']:.3f} is {margin:.2f} nats above threshold — strong outlier.",  # noqa: E501
            )
        return (
            "AMBIGUOUS",
            f"BPE {rec['bpe_score']:.3f} marginally exceeds threshold {rec['bpe_threshold']:.3f} (margin {margin:+.3f}); no foreign imports.",  # noqa: E501
        )


def main() -> None:
    # Load records
    all_records: list[dict[str, Any]] = []
    with _HUNKS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                all_records.append(json.loads(line))

    source_records = [r for r in all_records if not r["is_test"]]
    test_records = [r for r in all_records if r["is_test"]]

    # Per-PR aggregation (source hunks only)
    pr_map: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for r in source_records:
        pr_map[r["pr_number"]].append(r)

    pr_stats: list[dict[str, Any]] = []
    for pr_num, hunks in pr_map.items():
        n_flagged = sum(1 for h in hunks if h["flagged"])
        stage_counts: Counter[str] = Counter(h["reason"] for h in hunks if h["flagged"])
        pr_stats.append(
            {
                "pr_number": pr_num,
                "pr_title": hunks[0]["pr_title"],
                "pr_mergedAt": hunks[0]["pr_mergedAt"],
                "pr_url": hunks[0]["pr_url"],
                "n_hunks": len(hunks),
                "n_flagged": n_flagged,
                "flag_pct": n_flagged / len(hunks) if hunks else 0.0,
                "stage_breakdown": dict(stage_counts),
                "flagged": n_flagged > 0,
            }
        )
    pr_stats.sort(key=lambda p: p["pr_mergedAt"], reverse=True)

    n_prs_total = len(pr_stats)
    n_prs_flagged = sum(1 for p in pr_stats if p["flagged"])
    pr_flag_rate = n_prs_flagged / n_prs_total if n_prs_total else 0.0

    bpe_threshold = source_records[0]["bpe_threshold"] if source_records else 0.0

    # -----------------------------------------------------------------------
    # Build report lines
    # -----------------------------------------------------------------------
    lines: list[str] = [
        "# Phase 14 Experiment 5 — Real-PR Base-Rate Validation on FastAPI (2026-04-22)",
        "",
        "**Scorer:** `SequentialImportBpeScorer` V1 (unmodified from exp #2c, seed 0 calibration)",
        "",
        "**Hypothesis (single, binary):** V1 flags fewer than 15% of merged FastAPI PRs from the last year.",  # noqa: E501
        "",
        "**Mining criteria:** merged 2025-04-22 to 2026-04-22, non-bot authors, touches fastapi/*.py.",  # noqa: E501
        "",
        "**Extraction:** file-start-to-hunk-end on current HEAD of cached repo (`.argot/research/repos/fastapi`).",  # noqa: E501
        "Note: shallow clone (depth=1) — line-number drift is possible for PRs merged >90 days ago.",  # noqa: E501
        "",
        f"**Calibration:** seed=0, n=100 hunks from fastapi source.  BPE threshold: `{bpe_threshold:.4f}`",  # noqa: E501
        "",
        "**Pre-registered verdict table:**",
        "",
        "| criterion | V1 USEFUL | V1 PLAUSIBLE | V1 INCONCLUSIVE | V1 USELESS |",
        "|---|---|---|---|---|",
        "| % PRs with ≥1 flagged hunk | <15% | 15–30% | 30–60% | >60% |",
        "",
        "V1 USEFUL additionally requires ≥50% of flagged PRs to have at least one hunk judged LIKELY_STYLE_DRIFT.",  # noqa: E501
        "",
        "---",
        "",
        "## §1. PR-Level Summary Table",
        "",
        f"Total PRs scored: **{n_prs_total}**  |  Total PRs flagged: **{n_prs_flagged}**  "
        f"|  PR flag rate: **{pr_flag_rate:.1%}**",
        "",
        "| PR# | Title (60 chars) | Merged | Hunks | Flagged | Flag% | By stage |",
        "|---|---|---|---|---|---|---|",
    ]

    for p in pr_stats:
        stage_str = "+".join(f"{k}:{v}" for k, v in sorted(p["stage_breakdown"].items())) or "—"
        lines.append(
            f"| [{p['pr_number']}]({p['pr_url']}) "
            f"| {p['pr_title'][:60]} "
            f"| {p['pr_mergedAt'][:10]} "
            f"| {p['n_hunks']} "
            f"| {p['n_flagged']} "
            f"| {p['flag_pct']:.0%} "
            f"| {stage_str} |"
        )

    lines += [
        "",
        "---",
        "",
        "## §2. Aggregate Stats",
        "",
        f"- **Total PRs scored:** {n_prs_total}",
        f"- **Total source hunks scored:** {len(source_records)}",
        f"- **PRs with ≥1 flagged hunk:** {n_prs_flagged} ({pr_flag_rate:.1%})",
        "",
        "### PR-level flag_pct distribution",
        "",
        "| bin | count | % of PRs |",
        "|---|---|---|",
    ]

    bins: list[tuple[str, float, float]] = [
        ("0%", -0.001, 0.0),
        ("1–10%", 0.0, 0.10),
        ("10–25%", 0.10, 0.25),
        ("25–50%", 0.25, 0.50),
        ("50–100%", 0.50, 1.01),
    ]
    for label, lo, hi in bins:
        n = sum(1 for p in pr_stats if lo < p["flag_pct"] <= hi)
        lines.append(f"| {label} | {n} | {n / n_prs_total:.0%} |")

    # Stage attribution
    flagged_source = [r for r in source_records if r["flagged"]]
    n_import = sum(1 for r in flagged_source if r["reason"] == "import")
    n_bpe = sum(1 for r in flagged_source if r["reason"] == "bpe")
    n_total_flagged_hunks = len(flagged_source)

    lines += [
        "",
        "### Stage attribution (flagged source hunks)",
        "",
        "| stage | count | % of flagged hunks |",
        "|---|---|---|",
        f"| Stage 1 (import) | {n_import} | {n_import / n_total_flagged_hunks:.0%} |"
        if n_total_flagged_hunks
        else "| — | 0 | — |",
        f"| Stage 2 (BPE only) | {n_bpe} | {n_bpe / n_total_flagged_hunks:.0%} |"
        if n_total_flagged_hunks
        else "",
    ]

    lines += [
        "",
        "---",
        "",
        "## §3. Drift Check (PR age vs flag rate)",
        "",
        "| age bucket | n_prs | n_flagged | flag_rate |",
        "|---|---|---|---|",
    ]

    bucket_order = ["≤90 days", "91–180 days", "181–365 days"]
    bucket_prs: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for p in pr_stats:
        bucket_prs[_age_bucket(p["pr_mergedAt"])].append(p)

    for bucket in bucket_order:
        prs_in_bucket = bucket_prs[bucket]
        if not prs_in_bucket:
            lines.append(f"| {bucket} | 0 | — | — |")
            continue
        n_b = len(prs_in_bucket)
        n_f = sum(1 for p in prs_in_bucket if p["flagged"])
        lines.append(f"| {bucket} | {n_b} | {n_f} | {n_f / n_b:.0%} |")

    lines += [
        "",
        "Drift interpretation: if recent PRs flag dramatically less than old PRs,",
        "line-number drift from the shallow clone is inflating old-PR flag rates.",
        "",
        "---",
        "",
        "## §4. Test-File Diagnostic",
        "",
        f"Test-file hunks scored: {len(test_records)}",
    ]

    if test_records:
        n_test_flagged = sum(1 for r in test_records if r["flagged"])
        lines += [
            f"Test-file hunks flagged: {n_test_flagged} ({n_test_flagged / len(test_records):.1%})",
            "",
            "High flag rate on test hunks would confirm that test files need their own calibration treatment.",  # noqa: E501
        ]
    else:
        lines.append("(No test-file hunks found in the scored PRs.)")

    lines += [
        "",
        "---",
        "",
        "## §5. Sample Inspection (up to 10 flagged source hunks)",
        "",
    ]

    # Sample: prefer high-margin, diverse across PRs
    flagged_src = [r for r in source_records if r["flagged"]]
    if flagged_src:

        def _margin(r: dict[str, Any]) -> float:
            if r["reason"] == "import":
                return float(r["import_score"])
            return float(r["bpe_score"]) - float(r["bpe_threshold"])

        # One per PR first, then fill up to 10 by margin
        seen_prs: set[int] = set()
        sample: list[dict[str, Any]] = []
        for r in sorted(flagged_src, key=_margin, reverse=True):
            if r["pr_number"] not in seen_prs:
                sample.append(r)
                seen_prs.add(r["pr_number"])
                if len(sample) >= 10:
                    break
        # Fill remaining slots with highest-margin hunks not yet sampled
        remaining = [r for r in sorted(flagged_src, key=_margin, reverse=True) if r not in sample]
        sample.extend(remaining[: max(0, 10 - len(sample))])

        for rec in sample[:10]:
            judgment, rationale = _auto_judgment(rec)
            stage_info = (
                f"Stage 1 (import_score={rec['import_score']:.1f})"
                if rec["reason"] == "import"
                else f"Stage 2 (bpe_score={rec['bpe_score']:.4f}, threshold={rec['bpe_threshold']:.4f})"  # noqa: E501
            )
            diff_snippet = rec.get("diff_content", "")[:600].strip()
            lines += [
                f"### PR #{rec['pr_number']} — {rec['pr_title'][:60]}",
                "",
                f"- **URL:** {rec['pr_url']}",
                f"- **File:** `{rec['file_path']}`  lines {rec['hunk_start_line']}–{rec['hunk_end_line']}",  # noqa: E501
                f"- **Stage:** {stage_info}",
                f"- **Judgment:** {judgment}",
                f"- **Rationale:** {rationale}",
                "",
                "```diff",
                diff_snippet if diff_snippet else "(diff content unavailable)",
                "```",
                "",
            ]
    else:
        lines.append("No flagged source hunks found.")

    lines += [
        "",
        "---",
        "",
        "## §6. High-Flag PRs (flag_pct > 50%)",
        "",
    ]

    high_flag = [p for p in pr_stats if p["flag_pct"] > 0.50]
    if high_flag:
        lines += [
            "| PR# | Title | flag_pct | stage breakdown |",
            "|---|---|---|---|",
        ]
        for p in sorted(high_flag, key=lambda x: x["flag_pct"], reverse=True):
            stage_str = "+".join(f"{k}:{v}" for k, v in sorted(p["stage_breakdown"].items())) or "—"
            lines.append(
                f"| [{p['pr_number']}]({p['pr_url']}) "
                f"| {p['pr_title'][:60]} "
                f"| {p['flag_pct']:.0%} "
                f"| {stage_str} |"
            )

        lines += [""]
        # Inspect one hunk from each high-flag PR
        lines.append("**Sample hunk from each high-flag PR:**")
        lines.append("")
        for p in high_flag:
            pr_hunks = [
                r for r in source_records if r["pr_number"] == p["pr_number"] and r["flagged"]
            ]  # noqa: E501
            if not pr_hunks:
                continue
            rec = pr_hunks[0]
            judgment, rationale = _auto_judgment(rec)
            diff_snippet = rec.get("diff_content", "")[:400].strip()
            lines += [
                f"**PR #{p['pr_number']}** `{rec['file_path']}` lines {rec['hunk_start_line']}–{rec['hunk_end_line']}",  # noqa: E501
                f"reason={rec['reason']} | judgment={judgment}",
                "",
                "```diff",
                diff_snippet if diff_snippet else "(unavailable)",
                "```",
                "",
            ]
    else:
        lines.append("No PRs with flag_pct > 50%.")

    lines += [
        "",
        "---",
        "",
        "## §7. Stage 1 (Import) Breakdown — Foreign Modules",
        "",
    ]

    import_records = [r for r in source_records if r["reason"] == "import"]
    if import_records:
        mod_counter: Counter[str] = Counter()
        for r in import_records:
            for mod in r.get("foreign_modules") or []:
                mod_counter[mod] += 1

        lines += [
            f"Total Stage 1 flags: {len(import_records)}",
            "",
            "| foreign module | hunk count |",
            "|---|---|",
        ]
        for mod, count in mod_counter.most_common():
            lines.append(f"| `{mod}` | {count} |")

        lines += [
            "",
            "Interpretation: modules that appear frequently here are legitimate new dependencies",
            "introduced by merged PRs — Stage 1 may be too coarse (critique #6: binary cutoff).",
        ]
    else:
        lines.append("No Stage 1 (import) flags found.")

    # -----------------------------------------------------------------------
    # §8. Verdict
    # -----------------------------------------------------------------------
    verdict = _verdict_band(pr_flag_rate)

    # Check §5 judgment quality: ≥50% LIKELY_STYLE_DRIFT among flagged PRs?
    flagged_pr_nums = {p["pr_number"] for p in pr_stats if p["flagged"]}
    sampled_flagged = [
        r for r in source_records if r["flagged"] and r["pr_number"] in flagged_pr_nums
    ]  # noqa: E501
    n_style_drift = sum(1 for r in sampled_flagged if _auto_judgment(r)[0] == "LIKELY_STYLE_DRIFT")
    pct_style_drift = n_style_drift / len(sampled_flagged) if sampled_flagged else 0.0

    v1_useful_quality_met = pct_style_drift >= 0.50

    lines += [
        "",
        "---",
        "",
        "## §8. Verdict",
        "",
        "| metric | value |",
        "|---|---|",
        f"| PRs scored | {n_prs_total} |",
        f"| PRs flagged | {n_prs_flagged} ({pr_flag_rate:.1%}) |",
        f"| Source hunks scored | {len(source_records)} |",
        f"| Source hunks flagged | {n_total_flagged_hunks} |",
        f"| BPE threshold (seed=0) | {bpe_threshold:.4f} |",
        f"| Stage 1 flags | {n_import} |",
        f"| Stage 2 flags | {n_bpe} |",
        f"| Flagged hunks → LIKELY_STYLE_DRIFT | {n_style_drift}/{len(sampled_flagged)} ({pct_style_drift:.0%}) |",  # noqa: E501
        "",
        f"**PR flag rate: {pr_flag_rate:.1%} → {verdict}**",
        "",
    ]

    if verdict == "V1 USEFUL":
        quality_str = "MET" if v1_useful_quality_met else "NOT MET"
        lines += [
            f"Quality gate (≥50% flagged hunks are LIKELY_STYLE_DRIFT): **{quality_str}**",
            "",
        ]
        if v1_useful_quality_met:
            lines += [
                "**Overall: V1 USEFUL — flag rate below 15% AND quality gate met.**",
                "",
                "Recommendation: proceed to real-PR validation on `rich` (separate experiment).",
            ]
        else:
            lines += [
                "**Overall: V1 USEFUL on flag rate, but quality gate NOT MET.**",
                "",
                "Recommendation: manually inspect flagged PRs to verify quality judgment before",
                "proceeding. The automated AMBIGUOUS/FP rate may be overestimated.",
            ]
    elif verdict == "V1 PLAUSIBLE":
        lines += [
            "Flag rate 15–30% — not operationally clean but not noise.",
            "",
            "**Dominant FP source (from §5/§7):** see import breakdown above.",
            "",
            "Recommendation: identify the dominant false-positive pattern (from §7 foreign-module",
            "histogram) and propose a targeted fix (e.g. allowlist stdlib/common modules in Stage 1).",  # noqa: E501
        ]
    elif verdict == "V1 INCONCLUSIVE":
        lines += [
            "Flag rate 30–60% — too noisy for production but not catastrophically broken.",
            "",
            "Recommendation: V1 is benchmark-only. Before re-testing real-world, must:",
            "1. Fix the dominant FP source identified in §7.",
            "2. Re-validate on synthetic fixtures to confirm fix doesn't lose recall.",
            "3. Re-run this experiment after the fix.",
        ]
    else:  # V1 USELESS
        lines += [
            "**HARD KILL: flag rate >60% — V1 flags the majority of normal merged PRs.**",
            "",
            "Recommendation: V1 is benchmark-only. Minimum changes required before re-testing:",
            "1. Stage 1: add module allowlist (stdlib, commonly imported third-party).",
            "2. Stage 2: raise threshold percentile or switch to per-file calibration.",
            "3. Full re-validation on synthetic fixtures required.",
        ]

    lines.append("")

    # Write
    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written → {_DOCS_OUT}", flush=True)
    print(
        f"\nHEADLINE: {n_prs_flagged}/{n_prs_total} PRs flagged ({pr_flag_rate:.1%}) → {verdict}",
        flush=True,
    )  # noqa: E501


if __name__ == "__main__":
    main()
