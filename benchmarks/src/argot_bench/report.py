from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class CorpusReport:
    corpus: str
    language: str
    metrics: dict[str, Any]
    raw_scores: list[dict[str, Any]] = field(default_factory=list)


def write_corpus_json(report: CorpusReport, out_path: Path) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(asdict(report), indent=2, sort_keys=True))


def _fmt_pct(v: float) -> str:
    return f"{v * 100:.1f}%"


def _fmt_f(v: float, places: int = 4) -> str:
    return f"{v:.{places}f}"


def _quantile(sorted_xs: list[float], q: float) -> float:
    """Linear-interpolation quantile. `sorted_xs` must be sorted ascending."""
    if not sorted_xs:
        return 0.0
    if len(sorted_xs) == 1:
        return sorted_xs[0]
    pos = q * (len(sorted_xs) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(sorted_xs) - 1)
    frac = pos - lo
    return sorted_xs[lo] * (1 - frac) + sorted_xs[hi] * frac


def _split_raw(raw: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    breaks = [r for r in raw if r.get("source") != "real_pr"]
    ctrls = [r for r in raw if r.get("source") == "real_pr"]
    return breaks, ctrls


def _render_headline(reports: list[CorpusReport]) -> list[str]:
    lines = [
        "## Headline",
        "",
        "| Corpus | Lang | AUC | Recall | FP | Gap | N_fix | N_ctrl | Thr |",
        "|:---|:---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for r in reports:
        m = r.metrics
        breaks, ctrls = _split_raw(r.raw_scores)
        break_scores = sorted(float(b["bpe_score"]) for b in breaks)
        ctrl_scores = sorted(float(c["bpe_score"]) for c in ctrls)
        gap = (break_scores[0] - ctrl_scores[-1]) if (break_scores and ctrl_scores) else 0.0
        recalls = list(m.get("recall_by_category", {}).values())
        mean_recall = sum(recalls) / len(recalls) if recalls else 0.0
        lines.append(
            f"| {r.corpus} | {r.language} | "
            f"{_fmt_f(m.get('auc_catalog', 0.0))} | "
            f"{_fmt_pct(mean_recall)} | "
            f"{_fmt_pct(m.get('fp_rate_real_pr', 0.0))} | "
            f"{gap:+.3f} | "
            f"{m.get('n_fixtures', len(breaks))} | "
            f"{m.get('n_real_pr_hunks', len(ctrls))} | "
            f"{_fmt_f(m.get('threshold_mean', 0.0), 3)} |"
        )
    lines.append("")
    lines.append(
        "_Gap = min(break) − max(control). Positive = clean separation; negative = overlap._"
    )
    lines.append("")
    return lines


def _render_summary(r: CorpusReport) -> list[str]:
    m = r.metrics
    breaks, ctrls = _split_raw(r.raw_scores)
    break_scores = sorted(float(b["bpe_score"]) for b in breaks)
    ctrl_scores = sorted(float(c["bpe_score"]) for c in ctrls)
    gap = (break_scores[0] - ctrl_scores[-1]) if (break_scores and ctrl_scores) else 0.0
    fp_count = sum(1 for c in ctrls if c.get("flagged"))

    lines: list[str] = ["### Summary", ""]
    lines.append(f"- **AUC (catalog vs real-PR controls):** {_fmt_f(m.get('auc_catalog', 0.0))}")
    recalls = list(m.get("recall_by_category", {}).values())
    mean_recall = sum(recalls) / len(recalls) if recalls else 0.0
    lines.append(f"- **Recall (mean across categories):** {_fmt_pct(mean_recall)}")
    lines.append(
        f"- **FP rate on real PR hunks:** {_fmt_pct(m.get('fp_rate_real_pr', 0.0))} "
        f"({fp_count}/{len(ctrls)})"
    )
    thr_mean = m.get("threshold_mean", 0.0)
    lines.append(
        f"- **Threshold (mean across seeds):** {_fmt_f(thr_mean, 4)} "
        f"(CV: {_fmt_pct(m.get('threshold_cv', 0.0))})"
    )
    stab = m.get("calibration_stability", {})
    lines.append(
        f"- **Calibration stability:** rel_var={_fmt_f(stab.get('rel_var', 0.0), 6)}, "
        f"jaccard={_fmt_f(stab.get('jaccard', 0.0))}"
    )
    marker = "clean" if gap > 0 else "overlap"
    lines.append(f"- **Separation gap (min break − max control):** {gap:+.4f} ({marker})")
    lines.append(f"- **Sample sizes:** {len(breaks)} fixtures · {len(ctrls)} real-PR controls")
    sc = m.get("sample_controls")
    if sc:
        lines.append(f"- ⚠️  **Control subsample:** {sc} hunks per PR — not a baseline run")
    lines.append("")
    return lines


def _render_score_distribution(r: CorpusReport) -> list[str]:
    breaks, ctrls = _split_raw(r.raw_scores)
    if not breaks and not ctrls:
        return []
    break_scores = sorted(float(b["bpe_score"]) for b in breaks)
    ctrl_scores = sorted(float(c["bpe_score"]) for c in ctrls)
    thr = float(r.metrics.get("threshold_mean", 0.0))

    lines = ["### Score distribution", ""]
    lines.append("| | n | min | p25 | median | p75 | p90 | max |")
    lines.append("|:---|---:|---:|---:|---:|---:|---:|---:|")

    def _row(label: str, xs: list[float]) -> str:
        if not xs:
            return f"| {label} | 0 | — | — | — | — | — | — |"
        return (
            f"| {label} | {len(xs)} | "
            f"{_fmt_f(xs[0], 3)} | "
            f"{_fmt_f(_quantile(xs, 0.25), 3)} | "
            f"{_fmt_f(_quantile(xs, 0.50), 3)} | "
            f"{_fmt_f(_quantile(xs, 0.75), 3)} | "
            f"{_fmt_f(_quantile(xs, 0.90), 3)} | "
            f"{_fmt_f(xs[-1], 3)} |"
        )

    lines.append(_row("Break (catalog)", break_scores))
    lines.append(_row("Control (real PR)", ctrl_scores))
    lines.append("")
    if thr and break_scores and ctrl_scores:
        below_break = sum(1 for s in break_scores if s < thr)
        above_ctrl = sum(1 for s in ctrl_scores if s >= thr)
        lines.append(
            f"Threshold **{_fmt_f(thr, 4)}** — "
            f"{below_break}/{len(break_scores)} breaks fall below it (misses), "
            f"{above_ctrl}/{len(ctrl_scores)} controls fall at/above (false positives)."
        )
        lines.append("")
    return lines


def _render_recall_chart(r: CorpusReport) -> list[str]:
    rbc = r.metrics.get("recall_by_category", {})
    if not rbc:
        return []
    cats = sorted(rbc.keys())
    return [
        "### Recall by category",
        "",
        "```mermaid",
        "xychart-beta",
        f'    title "{r.corpus} — recall by category"',
        f"    x-axis [{', '.join(f'\"{c}\"' for c in cats)}]",
        '    y-axis "recall %" 0 --> 110',
        "    bar [" + ", ".join(_fmt_f(rbc[c] * 100, 1) for c in cats) + "]",
        "```",
        "",
    ]


def _render_category_table(r: CorpusReport) -> list[str]:
    breaks, _ = _split_raw(r.raw_scores)
    if not breaks:
        return []
    by_cat: dict[str, list[dict[str, Any]]] = {}
    for b in breaks:
        by_cat.setdefault(str(b.get("category", "?")), []).append(b)

    lines = ["### Per-category detail", ""]
    lines.append("| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |")
    lines.append("|:---|---:|---:|---:|---:|---:|:---|")
    for cat in sorted(by_cat.keys()):
        items = by_cat[cat]
        scores = [float(i["bpe_score"]) for i in items]
        hits = sum(1 for i in items if i.get("flagged"))
        total = len(items)
        recall = hits / total if total else 0.0
        mean_s = sum(scores) / len(scores) if scores else 0.0
        ids = ", ".join(str(i.get("id", "?")) for i in items)
        lines.append(
            f"| {cat} | {_fmt_pct(recall)} | {hits}/{total} | "
            f"{_fmt_f(mean_s, 3)} | {_fmt_f(min(scores), 3)} | {_fmt_f(max(scores), 3)} | "
            f"{ids} |"
        )
    lines.append("")
    return lines


def _render_fixture_table(r: CorpusReport) -> list[str]:
    breaks, _ = _split_raw(r.raw_scores)
    if not breaks:
        return []
    lines = [
        "### Per-fixture results",
        "",
        "<details>",
        f"<summary>{len(breaks)} fixtures (click to expand)</summary>",
        "",
        "| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |",
        "|:---|:---|---:|:---:|:---|:---|:---|:---|",
    ]
    sorted_breaks = sorted(
        breaks, key=lambda x: (str(x.get("category", "")), str(x.get("id", "")))
    )
    for b in sorted_breaks:
        flag = "✓" if b.get("flagged") else "✗"
        rationale = str(b.get("rationale", "") or "").replace("|", "\\|").replace("\n", " ")
        if len(rationale) > 140:
            rationale = rationale[:137] + "…"
        file_ = str(b.get("file", "") or "—")
        hs = b.get("hunk_start_line")
        he = b.get("hunk_end_line")
        line_range = f"{hs}–{he}" if hs is not None and he is not None else "—"
        lines.append(
            f"| {b.get('id', '?')} | {b.get('category', '?')} | "
            f"{_fmt_f(float(b.get('bpe_score', 0.0)), 3)} | {flag} | "
            f"{b.get('reason', '?')} | {file_} | {line_range} | {rationale} |"
        )
    lines.append("")
    lines.append("</details>")
    lines.append("")
    return lines


def _render_missed_fixtures(r: CorpusReport) -> list[str]:
    breaks, _ = _split_raw(r.raw_scores)
    missed = [b for b in breaks if not b.get("flagged")]
    if not missed:
        return []
    thr = float(r.metrics.get("threshold_mean", 0.0))
    lines = [
        f"### Missed fixtures ({len(missed)})",
        "",
        f"Breaks that didn't trip the scorer (threshold {_fmt_f(thr, 4)}):",
        "",
    ]
    for b in sorted(missed, key=lambda x: float(x.get("bpe_score", 0.0)), reverse=True):
        score = float(b.get("bpe_score", 0.0))
        gap = thr - score if thr else 0.0
        lines.append(
            f"- **{b.get('id', '?')}** "
            f"(`{b.get('category', '?')}`) — score {_fmt_f(score, 4)}, "
            f"{_fmt_f(gap, 4)} below threshold, reason: `{b.get('reason', '?')}`"
        )
        rationale = str(b.get("rationale", "") or "").strip()
        if rationale:
            lines.append(f"  - _Rationale:_ {rationale}")
    lines.append("")
    return lines


def _render_top_controls(r: CorpusReport, k: int = 5) -> list[str]:
    _, ctrls = _split_raw(r.raw_scores)
    if not ctrls:
        return []
    top = sorted(ctrls, key=lambda x: float(x.get("bpe_score", 0.0)), reverse=True)[:k]
    thr = float(r.metrics.get("threshold_mean", 0.0))
    lines = [
        f"### Top {len(top)} real-PR controls (closest to false positives)",
        "",
        "| Rank | BPE | Flagged | Reason | File | Lines |",
        "|---:|---:|:---:|:---|:---|:---|",
    ]
    for i, c in enumerate(top, start=1):
        flag = "✓" if c.get("flagged") else "✗"
        fp = str(c.get("file_path", "") or "—")
        hs = c.get("hunk_start_line")
        he = c.get("hunk_end_line")
        line_range = f"{hs}–{he}" if hs is not None and he is not None else "—"
        lines.append(
            f"| {i} | {_fmt_f(float(c.get('bpe_score', 0.0)), 3)} | {flag} | "
            f"{c.get('reason', '?')} | {fp} | {line_range} |"
        )
    lines.append("")
    if thr:
        lines.append(
            f"_Threshold is {_fmt_f(thr, 4)}; top control scores "
            f"{_fmt_f(float(top[0].get('bpe_score', 0.0)), 4)}._"
        )
        lines.append("")
    return lines


def _render_stage_attribution(r: CorpusReport) -> list[str]:
    sa = r.metrics.get("stage_attribution", {})
    if not sa:
        return []
    total = sum(int(v) for v in sa.values())
    lines = ["### Stage attribution", ""]
    for k in ("import", "call_receiver", "bpe", "none", "auto_generated"):
        if k in sa:
            n = int(sa[k])
            pct = (n / total * 100) if total else 0.0
            lines.append(f"- `{k}`: {n} ({pct:.1f}%)")
    lines.append("")
    return lines


def _render_difficulty_breakdown(r: CorpusReport) -> list[str]:
    rbd = r.metrics.get("recall_by_difficulty", {})
    breaks, _ = _split_raw(r.raw_scores)
    if not rbd and not breaks:
        return []
    totals: dict[str, int] = {}
    hits: dict[str, int] = {}
    for b in breaks:
        d = str(b.get("difficulty") or "")
        if not d:
            continue
        totals[d] = totals.get(d, 0) + 1
        if b.get("flagged"):
            hits[d] = hits.get(d, 0) + 1
    if not totals:
        return []
    lines = ["### Recall by difficulty", ""]
    lines.append("| Difficulty | Recall | Hits | Definition |")
    lines.append("|:---|---:|---:|:---|")
    defs = {
        "easy": "Stage 1 import catch — foreign module in hunk",
        "medium": "Stage 2 BPE catch — token-level novelty, no foreign import",
        "hard": "Stage 1.5 call-receiver catch — receiver novelty",
        "uncaught": "Scorer currently misses — known gap",
    }
    for band in ("easy", "medium", "hard", "uncaught"):
        if band not in totals:
            continue
        recall_val = rbd.get(band, 0.0)
        h = hits.get(band, 0)
        t = totals.get(band, 0)
        lines.append(
            f"| {band} | {_fmt_pct(recall_val)} | {h}/{t} | {defs[band]} |"
        )
    lines.append("")
    return lines


def render_report_md(reports: list[CorpusReport]) -> str:
    lines: list[str] = [
        "# argot-bench report",
        "",
        f"Generated: {datetime.now(tz=UTC).isoformat()}",
        "",
    ]
    lines.extend(_render_headline(reports))

    for r in reports:
        lines.append(f"## {r.corpus} ({r.language})")
        lines.append("")
        lines.extend(_render_summary(r))
        lines.extend(_render_score_distribution(r))
        lines.extend(_render_recall_chart(r))
        lines.extend(_render_category_table(r))
        lines.extend(_render_fixture_table(r))
        lines.extend(_render_missed_fixtures(r))
        lines.extend(_render_top_controls(r))
        lines.extend(_render_stage_attribution(r))
        lines.extend(_render_difficulty_breakdown(r))

    return "\n".join(lines)
