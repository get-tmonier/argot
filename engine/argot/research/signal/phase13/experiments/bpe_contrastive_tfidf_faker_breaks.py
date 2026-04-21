# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_faker_breaks.py
"""Phase 13: BPE-contrastive TF-IDF break scoring on faker fixtures.

Scores the 5 paradigm-break fixtures against the faker corpus (model_A = faker sources)
and compares them to the 159 calibration scores from the ordinary faker hunk distribution.

Key question: Do the 5 breaks score clearly above the normal distribution (clean separation),
overlap partially, or invert (no separation)?

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_faker_breaks.py
"""

from __future__ import annotations

import json
import math
import statistics
from collections import Counter
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"

# Script is at engine/argot/research/signal/phase13/experiments/
# 5 parents up = engine/argot/
_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog" / "faker"
_SAMPLED_HUNKS_PATH = _CATALOG_DIR / "sampled_hunks.jsonl"
_MODEL_A_DIR = _CATALOG_DIR / "sources" / "model_a"
_FIXTURES_DIR = _CATALOG_DIR / "fixtures"
_BREAKS_MANIFEST_PATH = _CATALOG_DIR / "breaks_manifest.json"

# Reference is at engine/argot/research/reference/ (4 parents up = engine/argot/research/)
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)


def _get_tokenizer() -> Any:
    return AutoTokenizer.from_pretrained(_MODEL_NAME)  # type: ignore[no-untyped-call]


def _load_model_b_bpe() -> tuple[dict[int, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a_bpe_faker(tokenizer: Any) -> tuple[dict[int, int], int]:
    """Build P_A from faker sources/model_a/ — excludes fixtures."""
    counts: Counter[int] = Counter()
    for path in sorted(_MODEL_A_DIR.glob("*.py")):
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        counts.update(ids)
    total = sum(counts.values())
    return dict(counts), total


def _load_sampled_hunks() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with _SAMPLED_HUNKS_PATH.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def _load_breaks_manifest() -> list[dict[str, Any]]:
    data: dict[str, Any] = json.loads(_BREAKS_MANIFEST_PATH.read_text(encoding="utf-8"))
    return data["fixtures"]  # type: ignore[no-any-return]


def _is_meaningful_token(token_str: str) -> bool:
    return len(token_str) >= 3 and any(c.isalnum() for c in token_str)


def _score_hunk(
    hunk_source: str,
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> tuple[float, float, int, list[str]]:
    """Return (max_score, mean_score, n_tokens_filtered, top_3_tokens)."""
    ids: list[int] = tokenizer.encode(hunk_source, add_special_tokens=False)

    filtered_ids = [i for i in ids if _is_meaningful_token(id_to_token.get(i, ""))]
    if not filtered_ids:
        filtered_ids = ids

    token_scores = [
        (
            i,
            math.log(model_b.get(i, 0) / total_b + _EPSILON)
            - math.log(model_a.get(i, 0) / total_a + _EPSILON),
        )
        for i in filtered_ids
    ]

    if not token_scores:
        return 0.0, 0.0, 0, []

    scores_only = [s for _, s in token_scores]
    max_score = max(scores_only)
    mean_score = statistics.mean(scores_only)
    n_tokens_filtered = len(filtered_ids)

    # Top 3 tokens by individual score
    sorted_by_score = sorted(token_scores, key=lambda x: x[1], reverse=True)
    top_3_tokens = [id_to_token.get(tid, str(tid)) for tid, _ in sorted_by_score[:3]]

    return max_score, mean_score, n_tokens_filtered, top_3_tokens


def _percentile(data: list[float], p: float) -> float:
    """Compute percentile p (0-100) using linear interpolation."""
    if not data:
        return 0.0
    sorted_data = sorted(data)
    n = len(sorted_data)
    idx = (p / 100) * (n - 1)
    lo = int(idx)
    hi = lo + 1
    if hi >= n:
        return sorted_data[-1]
    frac = idx - lo
    return sorted_data[lo] + frac * (sorted_data[hi] - sorted_data[lo])


def _extract_hunk_lines(source: str, start_line: int, end_line: int) -> str:
    """Extract lines [start_line, end_line] (1-indexed, inclusive)."""
    lines = source.splitlines()
    # Convert to 0-indexed
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def _score_breaks(
    manifest: list[dict[str, Any]],
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for fixture in manifest:
        fixture_path = _CATALOG_DIR / fixture["file"]
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        hunk_source = _extract_hunk_lines(
            source, fixture["hunk_start_line"], fixture["hunk_end_line"]
        )
        max_score, mean_score, n_tokens, top_3 = _score_hunk(
            hunk_source, tokenizer, id_to_token, model_a, total_a, model_b, total_b
        )
        results.append(
            {
                "name": fixture["name"],
                "category": fixture["category"],
                "max_score": max_score,
                "mean_score": mean_score,
                "n_tokens_filtered": n_tokens,
                "top_3_tokens": top_3,
            }
        )
    return results


def _percentile_rank(value: float, normal_scores: list[float]) -> float:
    """What % of normal_scores is strictly below value."""
    below = sum(1 for s in normal_scores if s < value)
    return 100.0 * below / len(normal_scores)


def _write_report(
    out: Path,
    break_results: list[dict[str, Any]],
    normal_scores: list[float],
) -> None:
    import math as _math

    break_max_scores = [r["max_score"] for r in break_results]

    # Calibration stats
    max_normal = max(normal_scores)
    p99_normal = _percentile(normal_scores, 99)
    p95_normal = _percentile(normal_scores, 95)
    p90_normal = _percentile(normal_scores, 90)
    min_break = min(break_max_scores)

    margin_vs_max = min_break - max_normal
    margin_vs_p99 = min_break - p99_normal

    # Verdict
    if min_break > max_normal:
        verdict = "CLEAN SEPARATION"
        verdict_detail = (
            f"All 5 break scores ({min_break:.4f}–{max(break_max_scores):.4f}) "
            f"sit strictly above every normal hunk (max_normal={max_normal:.4f}). "
            f"margin_vs_max={margin_vs_max:.4f} > 0."
        )
        phase14_rec = (
            "Ship BPE-tfidf as Phase 13 winner with per-repo calibration requirement; "
            "V1 scope = Python repos >= 50 files; "
            "Phase 14 focus = cross-language (TypeScript) validation."
        )
    elif min_break >= p95_normal:
        verdict = "PARTIAL OVERLAP"
        verdict_detail = (
            f"min(break_scores)={min_break:.4f} is in [{p95_normal:.4f}, {max_normal:.4f}] "
            f"(p95_normal..max_normal). Not full inversion, but some breaks fall in the "
            f"normal distribution's upper tail."
        )
        phase14_rec = (
            "BPE-tfidf is a partial solution; V1 must either restrict scope to strong-argot "
            "repos (FastAPI/rich) or add a second axis (AST structural contrast). "
            "Discuss trade-offs."
        )
    else:
        verdict = "FULL OVERLAP / INVERSION"
        verdict_detail = (
            f"min(break_scores)={min_break:.4f} < p95_normal={p95_normal:.4f}. "
            "Some breaks score no better than the bottom 5% of normal hunks."
        )
        phase14_rec = (
            "BPE-tfidf not production-ready for a general style linter. "
            "Investigate semantic approaches (tree-sitter AST structural, or "
            "CodeBERT zero-shot context-conditional perplexity)."
        )

    # ASCII histogram of normal scores (bucket width 0.5)
    normal_min = min(normal_scores)
    normal_max_val = max(normal_scores)
    bucket_lo = _math.floor(normal_min * 2) / 2
    bucket_hi = _math.ceil(normal_max_val * 2) / 2
    buckets: dict[float, int] = {}
    b = bucket_lo
    while b < bucket_hi:
        buckets[round(b, 1)] = 0
        b = round(b + 0.5, 1)

    for s in normal_scores:
        b_key = round(_math.floor(s * 2) / 2, 1)
        if b_key > bucket_hi - 0.5:
            b_key = round(bucket_hi - 0.5, 1)
        buckets[b_key] = buckets.get(b_key, 0) + 1

    max_bucket_count = max(buckets.values()) if buckets else 1
    bar_width = 40

    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — BPE Contrastive TF-IDF: Faker Break Scoring (2026-04-21)",
        "",
        "**Goal:** Score the 5 paradigm-break fixtures against the faker corpus "
        "and determine separation from the 159-hunk normal distribution.",
        "",
        "## 1. Break Scores",
        "",
        "| fixture | category | max_score | mean_score | top-3 tokens |",
        "|---|---|---|---|---|",
    ]

    for r in break_results:
        top3 = ", ".join(r["top_3_tokens"])
        lines.append(
            f"| {r['name']} | {r['category']} | {r['max_score']:.4f}"
            f" | {r['mean_score']:.4f} | {top3} |"
        )

    lines += [
        "",
        "## 2. Overlap with Normal Distribution",
        "",
        "Percentile rank = % of 159 normal scores strictly below this break's max_score.",
        "",
        "| break | max_score | higher than X% of normal | percentile rank |",
        "|---|---|---|---|",
    ]

    for r in break_results:
        prank = _percentile_rank(r["max_score"], normal_scores)
        lines.append(f"| {r['name']} | {r['max_score']:.4f} | {prank:.1f}% | p{prank:.1f} |")

    lines += [
        "",
        "## 3. Separation Metrics",
        "",
        "| metric | value |",
        "|---|---|",
        f"| max_normal (max of 159 normal scores) | {max_normal:.4f} |",
        f"| p99_normal | {p99_normal:.4f} |",
        f"| p95_normal | {p95_normal:.4f} |",
        f"| p90_normal | {p90_normal:.4f} |",
        f"| min(break_scores) | {min_break:.4f} |",
        f"| max(break_scores) | {max(break_max_scores):.4f} |",
        f"| **margin_vs_max** (min_break − max_normal) | **{margin_vs_max:.4f}** |",
        f"| **margin_vs_p99** (min_break − p99_normal) | **{margin_vs_p99:.4f}** |",
        "",
        "A positive `margin_vs_max` means clean separation: every break scores above "
        "every normal hunk.",
        "",
        "## 4. ASCII Histogram of Normal Scores + Break Annotations",
        "",
        "Normal distribution (159 hunks, bucket width 0.5):",
        "",
        "```",
    ]

    for b_key in sorted(buckets.keys()):
        cnt = buckets[b_key]
        bar_len = round(cnt * bar_width / max_bucket_count) if max_bucket_count > 0 else 0
        bar = "#" * bar_len
        hi_key = round(b_key + 0.5, 1)
        lines.append(f"[{b_key:.1f}-{hi_key:.1f}]: {bar:<{bar_width}} ({cnt})")

    lines.append("```")
    lines.append("")
    lines.append("Break positions:")
    lines.append("")

    for r in break_results:
        score = r["max_score"]
        bucket_key = round(_math.floor(score * 2) / 2, 1)
        hi_key = round(bucket_key + 0.5, 1)
        annotation = (
            f"- `{r['name']}`: {score:.4f}" f"  <- falls in bucket {bucket_key:.1f}-{hi_key:.1f}"
        )
        lines.append(annotation)

    lines += [
        "",
        "## 5. Verdict",
        "",
        f"**{verdict}**",
        "",
        verdict_detail,
        "",
        "### Thresholds applied",
        "",
        "| criterion | condition | value | result |",
        "|---|---|---|---|",
        f"| Clean separation | min(break_scores) > max_normal | "
        f"{min_break:.4f} > {max_normal:.4f} | "
        f"{'YES' if min_break > max_normal else 'NO'} |",
        f"| Partial overlap | min(break_scores) in [p95_normal, max_normal] | "
        f"{min_break:.4f} in [{p95_normal:.4f}, {max_normal:.4f}] | "
        f"{'YES' if p95_normal <= min_break <= max_normal else 'NO'} |",
        f"| Full overlap | min(break_scores) < p95_normal | "
        f"{min_break:.4f} < {p95_normal:.4f} | "
        f"{'YES' if min_break < p95_normal else 'NO'} |",
        "",
        "## 6. Phase 13 Final Recommendation",
        "",
        f"**Verdict: {verdict}**",
        "",
        phase14_rec,
        "",
        "### Supporting numbers",
        "",
        f"- Normal distribution: n=159, max={max_normal:.4f}, p99={p99_normal:.4f}, "
        f"p95={p95_normal:.4f}",
        f"- Break score range: [{min_break:.4f}, {max(break_max_scores):.4f}]",
        f"- margin_vs_max = {margin_vs_max:.4f} "
        f"({'positive — clean gap' if margin_vs_max > 0 else 'negative — overlap'})",
        f"- margin_vs_p99 = {margin_vs_p99:.4f}",
        "",
    ]

    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(out: Path | None = None) -> dict[str, Any]:
    print("Loading tokenizer...", flush=True)
    tokenizer = _get_tokenizer()
    vocab = tokenizer.get_vocab()
    id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

    print("Building Model A (faker sources/model_a corpus)...", flush=True)
    model_a, total_a = _build_model_a_bpe_faker(tokenizer)
    print(f"  Model A: {len(model_a)} unique tokens, {total_a} total", flush=True)

    print("Loading P_B (generic reference)...", flush=True)
    model_b, total_b = _load_model_b_bpe()
    print(f"  Model B: {len(model_b)} unique tokens, {total_b} total", flush=True)

    print("Loading calibration (normal) scores from sampled_hunks.jsonl...", flush=True)
    normal_records = _load_sampled_hunks()
    print(f"  Loaded {len(normal_records)} normal hunks", flush=True)

    print("Re-scoring normal hunks to get calibration distribution...", flush=True)
    normal_scores: list[float] = []
    for rec in normal_records:
        max_score, _, _, _ = _score_hunk(
            rec["hunk_source"], tokenizer, id_to_token, model_a, total_a, model_b, total_b
        )
        normal_scores.append(max_score)

    max_normal = max(normal_scores)
    p99_n = _percentile(normal_scores, 99)
    p95_n = _percentile(normal_scores, 95)
    p90_n = _percentile(normal_scores, 90)
    print(
        f"  Normal distribution: max={max_normal:.4f}, p99={p99_n:.4f}, "
        f"p95={p95_n:.4f}, p90={p90_n:.4f}",
        flush=True,
    )

    print("Loading break fixtures manifest...", flush=True)
    manifest = _load_breaks_manifest()

    print("Scoring break fixtures...", flush=True)
    break_results = _score_breaks(
        manifest, tokenizer, id_to_token, model_a, total_a, model_b, total_b
    )

    print("\nBreak scores:", flush=True)
    print(f"{'fixture':<35} {'category':<22} {'max_score':>10} {'mean_score':>10}", flush=True)
    print("-" * 82, flush=True)
    for r in break_results:
        print(
            f"{r['name']:<35} {r['category']:<22} {r['max_score']:>10.4f} {r['mean_score']:>10.4f}",
            flush=True,
        )

    print(f"\nCalibration stats (n={len(normal_scores)}):", flush=True)
    print(
        f"  max_normal={max_normal:.4f}, p99={p99_n:.4f}, p95={p95_n:.4f}, p90={p90_n:.4f}",
        flush=True,
    )

    break_max_scores = [r["max_score"] for r in break_results]
    min_break = min(break_max_scores)
    margin_vs_max = min_break - max_normal
    margin_vs_p99 = min_break - p99_n

    if min_break > max_normal:
        verdict = "CLEAN SEPARATION"
    elif min_break >= p95_n:
        verdict = "PARTIAL OVERLAP"
    else:
        verdict = "FULL OVERLAP / INVERSION"

    print(f"\nVerdict: {verdict}", flush=True)
    print(f"  margin_vs_max={margin_vs_max:.4f}", flush=True)
    print(f"  margin_vs_p99={margin_vs_p99:.4f}", flush=True)

    if out is not None:
        _write_report(out, break_results, normal_scores)

    return {
        "break_results": break_results,
        "normal_max": max_normal,
        "p99_normal": p99_n,
        "p95_normal": p95_n,
        "margin_vs_max": margin_vs_max,
        "margin_vs_p99": margin_vs_p99,
        "verdict": verdict,
    }


def main() -> None:
    out = (
        Path(__file__).parent.parent.parent.parent.parent.parent.parent
        / "docs"
        / "research"
        / "scoring"
        / "signal"
        / "phase13"
        / "experiments"
        / "bpe_contrastive_tfidf_faker_breaks_2026-04-21.md"
    )
    run(out=out)


if __name__ == "__main__":
    main()
