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


def render_report_md(reports: list[CorpusReport]) -> str:
    lines: list[str] = []
    ts = datetime.now(tz=UTC).isoformat()
    lines.append("# argot-bench report")
    lines.append("")
    lines.append(f"Generated: {ts}")
    lines.append("")

    # Headline table
    lines.append("## Headline")
    lines.append("")
    lines.append("| Corpus | Language | AUC | Recall (mean) | FP rate | Threshold CV |")
    lines.append("|:---|:---|---:|---:|---:|---:|")
    for r in reports:
        m = r.metrics
        recalls = list(m.get("recall_by_category", {}).values())
        mean_recall = sum(recalls) / len(recalls) if recalls else 0.0
        lines.append(
            f"| {r.corpus} | {r.language} | "
            f"{_fmt_f(m.get('auc_catalog', 0.0))} | "
            f"{_fmt_pct(mean_recall)} | "
            f"{_fmt_pct(m.get('fp_rate_real_pr', 0.0))} | "
            f"{_fmt_pct(m.get('threshold_cv', 0.0))} |"
        )
    lines.append("")

    # Per-corpus sections
    for r in reports:
        m = r.metrics
        lines.append(f"## {r.corpus} ({r.language})")
        lines.append("")
        lines.append(f"- **AUC (catalog vs real-PR controls):** {_fmt_f(m.get('auc_catalog', 0.0))}")
        lines.append(f"- **FP rate on real PR hunks:** {_fmt_pct(m.get('fp_rate_real_pr', 0.0))}")
        lines.append(f"- **Threshold CV (5 seeds):** {_fmt_pct(m.get('threshold_cv', 0.0))}")
        stab = m.get("calibration_stability", {})
        lines.append(
            f"- **Calibration stability:** rel_var={_fmt_f(stab.get('rel_var', 0.0), 6)}, "
            f"jaccard={_fmt_f(stab.get('jaccard', 0.0))}"
        )
        lines.append("")

        # Recall by category
        rbc = m.get("recall_by_category", {})
        if rbc:
            lines.append("### Recall by category")
            lines.append("")
            lines.append("```mermaid")
            lines.append("xychart-beta")
            lines.append(f'    title "{r.corpus} — recall by category"')
            cats = sorted(rbc.keys())
            lines.append(f"    x-axis [{', '.join(f'\"{c}\"' for c in cats)}]")
            lines.append('    y-axis "recall %" 0 --> 110')
            lines.append("    bar [" + ", ".join(_fmt_f(rbc[c] * 100, 1) for c in cats) + "]")
            lines.append("```")
            lines.append("")

        # Stage attribution
        sa = m.get("stage_attribution", {})
        if sa:
            lines.append("### Stage attribution")
            lines.append("")
            for k in ("import", "bpe", "none", "auto_generated"):
                if k in sa:
                    lines.append(f"- {k}: {sa[k]}")
            lines.append("")

    return "\n".join(lines)
