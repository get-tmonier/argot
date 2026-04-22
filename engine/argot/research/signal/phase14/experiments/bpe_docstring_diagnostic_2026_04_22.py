# engine/argot/research/signal/phase14/experiments/bpe_docstring_diagnostic_2026_04_22.py
"""Phase 14 Exp #6 Step 4 — BPE docstring/prose diagnostic.

Reads: real_pr_base_rate_hunks_fix1_2026_04_22.jsonl
For each source hunk, computes:
  - total_lines: number of lines in the file-prefix (file-start to hunk_end_line)
  - docstring_lines: lines inside triple-quoted string literals used as docstrings (ast-based)
  - comment_lines: lines starting with # (after stripping)
  - prose_ratio: (docstring_lines + comment_lines) / total_lines

Compares distributions between bpe_flagged=True and bpe_flagged=False hunks.

Writes: docs/research/scoring/signal/phase14/experiments/bpe_docstring_diagnostic_2026-04-22.md

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/bpe_docstring_diagnostic_2026_04_22.py
"""

from __future__ import annotations

import ast
import json
import re
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).parent
_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_FASTAPI_REPO = _REPOS_DIR / "fastapi"

_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix1_2026_04_22.jsonl"
_DOCS_OUT = (
    _PROJECT_ROOT
    / "docs"
    / "research"
    / "scoring"
    / "signal"
    / "phase14"
    / "experiments"
    / "bpe_docstring_diagnostic_2026-04-22.md"
)

_RE_TRIPLE_DOUBLE = re.compile(r'""".*?"""', re.DOTALL)
_RE_TRIPLE_SINGLE = re.compile(r"'''.*?'''", re.DOTALL)


def _docstring_line_ranges_ast(source: str) -> list[tuple[int, int]]:
    """Return (start_lineno, end_lineno) 1-indexed ranges for AST docstrings."""
    ranges: list[tuple[int, int]] = []
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return []

    for node in ast.walk(tree):
        if isinstance(
            node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Module)
        ):
            body = node.body
            if not body:
                continue
            first = body[0]
            if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant):
                if isinstance(first.value.value, str):
                    ranges.append((first.lineno, first.end_lineno or first.lineno))
    return ranges


def _docstring_line_ranges_regex(source: str) -> list[tuple[int, int]]:
    """Fallback: regex-detect triple-quoted blocks used as leading statements."""
    ranges: list[tuple[int, int]] = []
    for pat in (_RE_TRIPLE_DOUBLE, _RE_TRIPLE_SINGLE):
        for m in pat.finditer(source):
            start_line = source[: m.start()].count("\n") + 1
            end_line = source[: m.end()].count("\n") + 1
            # Only count if the block start looks like a module/class/def first statement
            # (no assignment before the triple-quote on its line)
            prefix_line = source[: m.start()].rsplit("\n", 1)[-1].strip()
            if prefix_line == "" or prefix_line.startswith("#"):
                ranges.append((start_line, end_line))
    return ranges


def _count_prose_lines(source: str) -> tuple[int, int, int]:
    """Return (total_lines, docstring_lines, comment_lines) for the given source."""
    lines = source.splitlines()
    total_lines = len(lines)
    if not total_lines:
        return 0, 0, 0

    # Docstring lines via ast, fallback to regex
    ds_ranges = _docstring_line_ranges_ast(source)
    if not ds_ranges:
        ds_ranges = _docstring_line_ranges_regex(source)

    docstring_line_set: set[int] = set()
    for lo, hi in ds_ranges:
        for ln in range(lo, hi + 1):
            docstring_line_set.add(ln)

    # Comment lines: lines whose non-whitespace content starts with #
    comment_line_set: set[int] = set()
    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("#"):
            comment_line_set.add(i)

    docstring_lines = len(docstring_line_set)
    comment_lines = len(comment_line_set - docstring_line_set)
    return total_lines, docstring_lines, comment_lines


def _extract_file_prefix(file_path: Path, end_line: int) -> str | None:
    if not file_path.exists():
        return None
    text = file_path.read_text(encoding="utf-8", errors="replace")
    all_lines = text.splitlines()
    hi = min(len(all_lines), end_line)
    return "\n".join(all_lines[:hi])


def _ascii_histogram(
    values: list[float],
    bins: list[tuple[str, float, float]],
    width: int = 30,
) -> list[str]:
    """Return ASCII histogram lines for the given values."""
    counts: dict[str, int] = {}
    for label, lo, hi in bins:
        counts[label] = sum(1 for v in values if lo <= v < hi)
    # last bin is inclusive
    last_label, last_lo, last_hi = bins[-1]
    counts[last_label] += sum(1 for v in values if v == last_hi)

    max_count = max(counts.values()) if counts else 1
    lines = []
    for label, lo, hi in bins:
        n = counts[label]
        bar_len = int(n / max_count * width) if max_count else 0
        bar = "#" * bar_len
        lines.append(f"  {label:>8s} | {bar:<{width}s} {n}")
    return lines


def _percentile(sorted_vals: list[float], p: float) -> float:
    if not sorted_vals:
        return float("nan")
    idx = (len(sorted_vals) - 1) * p
    lo = int(idx)
    hi = lo + 1
    if hi >= len(sorted_vals):
        return sorted_vals[-1]
    frac = idx - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def main() -> None:
    # Load records
    all_records: list[dict[str, Any]] = []
    with _HUNKS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line:
                all_records.append(json.loads(line))

    source_records = [r for r in all_records if not r["is_test"]]
    print(f"Source records: {len(source_records)}", flush=True)

    # Per-hunk prose analysis
    enriched: list[dict[str, Any]] = []
    n_missing = 0
    n_ast_ok = 0
    n_regex_fallback = 0

    for i, rec in enumerate(source_records):
        if i % 200 == 0:
            print(f"  [{i}/{len(source_records)}] ...", flush=True)

        fp = _FASTAPI_REPO / rec["file_path"]
        source = _extract_file_prefix(fp, rec["hunk_end_line"])
        if source is None:
            n_missing += 1
            continue

        # Check ast parse success
        try:
            ast.parse(source)
            n_ast_ok += 1
            ds_ranges = _docstring_line_ranges_ast(source)
        except SyntaxError:
            n_regex_fallback += 1
            ds_ranges = _docstring_line_ranges_regex(source)

        total, ds_lines, cmt_lines = _count_prose_lines(source)
        prose = (ds_lines + cmt_lines) / total if total else 0.0

        enriched.append(
            {
                **rec,
                "total_lines": total,
                "docstring_lines": ds_lines,
                "comment_lines": cmt_lines,
                "prose_ratio": prose,
                "bpe_flagged": rec["flagged"] and rec["reason"] == "bpe",
            }
        )

    flagged = [r for r in enriched if r["bpe_flagged"]]
    unflagged = [r for r in enriched if not r["bpe_flagged"]]

    print(f"Enriched: {len(enriched)}, missing: {n_missing}", flush=True)
    print(f"AST ok: {n_ast_ok}, regex fallback: {n_regex_fallback}", flush=True)
    print(f"BPE flagged: {len(flagged)}, unflagged: {len(unflagged)}", flush=True)

    # -------------------------------------------------------------------------
    # Statistics
    # -------------------------------------------------------------------------
    def _stats(group: list[dict[str, Any]], key: str) -> dict[str, float]:
        vals = sorted(r[key] for r in group)
        if not vals:
            return {"mean": float("nan"), "median": float("nan"), "p25": float("nan"), "p75": float("nan"), "p95": float("nan"), "min": float("nan"), "max": float("nan")}
        return {
            "mean": statistics.mean(vals),
            "median": statistics.median(vals),
            "p25": _percentile(vals, 0.25),
            "p75": _percentile(vals, 0.75),
            "p95": _percentile(vals, 0.95),
            "min": vals[0],
            "max": vals[-1],
        }

    prose_bins = [
        ("0-10%", 0.0, 0.10),
        ("10-25%", 0.10, 0.25),
        ("25-50%", 0.25, 0.50),
        ("50-75%", 0.50, 0.75),
        ("75-100%", 0.75, 1.01),
    ]

    # Flag rate per prose bucket
    bucket_stats: dict[str, dict[str, int]] = defaultdict(lambda: {"n": 0, "n_flagged": 0})
    for r in enriched:
        pr = r["prose_ratio"]
        for label, lo, hi in prose_bins:
            if lo <= pr < hi or (label == "75-100%" and pr >= 0.75):
                bucket_stats[label]["n"] += 1
                if r["bpe_flagged"]:
                    bucket_stats[label]["n_flagged"] += 1
                break

    # -------------------------------------------------------------------------
    # Verdict
    # -------------------------------------------------------------------------
    flagged_ratios = sorted(r["prose_ratio"] for r in flagged)
    unflagged_ratios = sorted(r["prose_ratio"] for r in unflagged)

    med_flagged = statistics.median(flagged_ratios) if flagged_ratios else 0.0
    med_unflagged = statistics.median(unflagged_ratios) if unflagged_ratios else 1.0

    # Ratio test: flagged median >= 1.5x unflagged median?
    median_ratio_test = med_flagged >= 1.5 * med_unflagged if med_unflagged > 0 else False

    # Top-bucket vs bottom-bucket flag rate
    top_bucket = bucket_stats["75-100%"]
    bottom_bucket = bucket_stats["0-10%"]
    top_rate = top_bucket["n_flagged"] / top_bucket["n"] if top_bucket["n"] else 0.0
    bottom_rate = bottom_bucket["n_flagged"] / bottom_bucket["n"] if bottom_bucket["n"] else 0.0
    bucket_ratio_test = top_rate >= 3 * bottom_rate if bottom_rate > 0 else top_rate > 0

    # Within 20%: medians within 20%?
    within_20pct = abs(med_flagged - med_unflagged) <= 0.20 * max(med_flagged, med_unflagged, 1e-9)
    flag_rate_flat = abs(top_rate - bottom_rate) <= 0.10

    if median_ratio_test and bucket_ratio_test:
        verdict = "CONFIRMED"
    elif within_20pct and flag_rate_flat:
        verdict = "REJECTED"
    else:
        verdict = "AMBIGUOUS"

    # -------------------------------------------------------------------------
    # Build report
    # -------------------------------------------------------------------------
    st_f = _stats(flagged, "prose_ratio")
    st_u = _stats(unflagged, "prose_ratio")

    lines: list[str] = [
        "# Phase 14 Exp #6 Step 4 — BPE Docstring/Prose Diagnostic (2026-04-22)",
        "",
        "**Input:** `real_pr_base_rate_hunks_fix1_2026_04_22.jsonl` (fix1 scored results)",
        "",
        "**Question:** Do BPE-flagged hunks have higher prose ratios than unflagged hunks?",
        "",
        "**Pre-registered verdict criteria:**",
        "- CONFIRMED: median(flagged) ≥ 1.5× median(unflagged) AND top-bucket flag rate ≥ 3× bottom-bucket rate",
        "- REJECTED: medians within 20% AND flag rate flat across buckets",
        "- AMBIGUOUS: in between",
        "",
        "---",
        "",
        "## §1. Per-hunk prose analysis method",
        "",
        "Each scored hunk is extracted as file-start to hunk-end-line (same as in scoring).",
        "For each prefix:",
        "- **docstring_lines**: line ranges of `Expr(Constant(str))` nodes that are the first",
        "  statement of a `Module`, `ClassDef`, `FunctionDef`, or `AsyncFunctionDef` (via `ast`).",
        "  Fallback to triple-quote regex when `ast.parse` raises `SyntaxError`.",
        "- **comment_lines**: lines whose stripped content starts with `#`, not already counted",
        "  as docstring lines.",
        "- **prose_ratio**: `(docstring_lines + comment_lines) / total_lines`",
        "",
        f"- Source records processed: {len(enriched)}",
        f"- Files missing from HEAD: {n_missing}",
        f"- AST parse succeeded: {n_ast_ok}",
        f"- Regex fallback: {n_regex_fallback}",
        f"- BPE-flagged hunks: {len(flagged)}",
        f"- BPE-unflagged hunks: {len(unflagged)}",
        "",
        "---",
        "",
        "## §2. Prose ratio distributions",
        "",
        "### Flagged (BPE-triggered) hunks",
        "",
        "| stat | prose_ratio |",
        "|---|---|",
        f"| mean | {st_f['mean']:.4f} |",
        f"| median | {st_f['median']:.4f} |",
        f"| p25 | {st_f['p25']:.4f} |",
        f"| p75 | {st_f['p75']:.4f} |",
        f"| p95 | {st_f['p95']:.4f} |",
        f"| min | {st_f['min']:.4f} |",
        f"| max | {st_f['max']:.4f} |",
        "",
        "### Unflagged hunks",
        "",
        "| stat | prose_ratio |",
        "|---|---|",
        f"| mean | {st_u['mean']:.4f} |",
        f"| median | {st_u['median']:.4f} |",
        f"| p25 | {st_u['p25']:.4f} |",
        f"| p75 | {st_u['p75']:.4f} |",
        f"| p95 | {st_u['p95']:.4f} |",
        f"| min | {st_u['min']:.4f} |",
        f"| max | {st_u['max']:.4f} |",
        "",
        f"**Median ratio (flagged / unflagged):** {med_flagged:.4f} / {med_unflagged:.4f} = {med_flagged / med_unflagged:.2f}x"
        if med_unflagged > 0
        else f"**Median flagged:** {med_flagged:.4f} | **Median unflagged:** {med_unflagged:.4f}",
        "",
        "---",
        "",
        "## §3. ASCII histogram — prose_ratio distribution",
        "",
        "### Flagged hunks",
        "```",
    ]
    lines.extend(_ascii_histogram(flagged_ratios, prose_bins))
    lines += [
        "```",
        "",
        "### Unflagged hunks",
        "```",
    ]
    lines.extend(_ascii_histogram(unflagged_ratios, prose_bins))
    lines += [
        "```",
        "",
        "---",
        "",
        "## §4. BPE flag rate per prose bucket",
        "",
        "| bucket | n_hunks | n_bpe_flagged | flag_rate |",
        "|---|---|---|---|",
    ]
    for label, lo, hi in prose_bins:
        bs = bucket_stats[label]
        n = bs["n"]
        nf = bs["n_flagged"]
        rate = nf / n if n else 0.0
        lines.append(f"| {label} | {n} | {nf} | {rate:.1%} |")

    lines += [
        "",
        f"Top-bucket (75–100%) flag rate: {top_rate:.1%}",
        f"Bottom-bucket (0–10%) flag rate: {bottom_rate:.1%}",
        f"Ratio: {top_rate / bottom_rate:.1f}x" if bottom_rate > 0 else "Ratio: ∞ (bottom bucket = 0)",
        "",
        "---",
        "",
        "## §5. Verdict",
        "",
        "| test | result |",
        "|---|---|",
        f"| median(flagged) ≥ 1.5× median(unflagged) | {'PASS' if median_ratio_test else 'FAIL'} ({med_flagged:.4f} vs {med_unflagged:.4f}) |",
        f"| top-bucket flag rate ≥ 3× bottom-bucket rate | {'PASS' if bucket_ratio_test else 'FAIL'} ({top_rate:.1%} vs {bottom_rate:.1%}) |",
        f"| medians within 20% | {'YES' if within_20pct else 'NO'} |",
        f"| flag rate flat across buckets | {'YES' if flag_rate_flat else 'NO'} |",
        "",
        f"## Verdict: {verdict}",
        "",
    ]

    if verdict == "CONFIRMED":
        lines += [
            "BPE-flagged hunks have substantially higher prose ratios than unflagged hunks.",
            "The prose-stripping fix (Step 5) is pre-authorized.",
            "",
            "**Interpretation:** The BPE over-trigger is driven by docstring and comment text in",
            "the file prefix. Tokens from long prose blocks (API documentation, warning strings,",
            "etc.) have high model_B / model_A ratios because generic corpora contain more prose",
            "than the fastapi library code. Stripping docstrings and comment lines before BPE",
            "scoring should substantially reduce false positives.",
        ]
    elif verdict == "REJECTED":
        lines += [
            "Prose ratio does NOT explain BPE over-trigger. Flagged and unflagged hunks have",
            "similar prose ratios. The BPE over-trigger has a different root cause.",
            "",
            "**HARD STOP: Step 5 not authorized.** Further investigation needed.",
        ]
    else:  # AMBIGUOUS
        lines += [
            "Results are ambiguous: prose ratio partially explains the BPE over-trigger",
            "but the signal is not strong enough to confirm. More diagnostics recommended",
            "before implementing the prose-stripping fix.",
            "",
            "**HARD STOP: Step 5 not authorized.** Recommend deeper token-level analysis.",
        ]

    lines.append("")

    # Write report
    _DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    _DOCS_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written → {_DOCS_OUT}", flush=True)
    print(f"\nVerdict: {verdict}", flush=True)
    print(
        f"Median flagged={med_flagged:.4f} unflagged={med_unflagged:.4f} ratio={med_flagged/med_unflagged:.2f}x"
        if med_unflagged > 0
        else f"Median flagged={med_flagged:.4f} unflagged={med_unflagged:.4f}",
        flush=True,
    )
    print(f"Top-bucket={top_rate:.1%} bottom-bucket={bottom_rate:.1%}", flush=True)


if __name__ == "__main__":
    main()
