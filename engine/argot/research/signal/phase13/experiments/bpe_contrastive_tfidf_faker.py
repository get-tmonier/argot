# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_faker.py
"""Phase 13: BPE-contrastive TF-IDF calibration on faker (ordinary OSS code).

Scores 159 sampled hunks from the faker library to determine whether the scorer
produces false positives on ordinary, non-style-breaking code.

Key question: Does the faker distribution sit safely below the break range (5–11),
or does it overlap, indicating systemic hallucination?

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_faker.py
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


def _write_report(
    out: Path,
    records: list[dict[str, Any]],
    results: list[dict[str, Any]],
) -> None:
    max_scores = [r["max_score"] for r in results]

    # Summary stats
    mn = min(max_scores)
    mx = max(max_scores)
    mean_val = statistics.mean(max_scores)
    stdev_val = statistics.stdev(max_scores) if len(max_scores) > 1 else 0.0
    p10 = _percentile(max_scores, 10)
    p25 = _percentile(max_scores, 25)
    p50 = _percentile(max_scores, 50)
    p75 = _percentile(max_scores, 75)
    p90 = _percentile(max_scores, 90)
    p99 = _percentile(max_scores, 99)

    pct_above_5 = 100.0 * sum(1 for s in max_scores if s > 5) / len(max_scores)
    pct_above_6 = 100.0 * sum(1 for s in max_scores if s > 6) / len(max_scores)

    # Histogram
    import math as _math

    bucket_min = int(_math.floor(mn))
    bucket_max = int(_math.ceil(mx))
    histogram: dict[int, int] = {}
    for b in range(bucket_min, bucket_max):
        histogram[b] = 0
    for s in max_scores:
        b = int(_math.floor(s))
        if b >= bucket_max:
            b = bucket_max - 1
        histogram[b] = histogram.get(b, 0) + 1

    # Bimodality check: find gaps of >= 2 units with no hunks
    occupied_buckets = sorted(b for b, cnt in histogram.items() if cnt > 0)
    gap_found = False
    gap_description = ""
    gap_hi: int = 0
    if len(occupied_buckets) >= 2:
        for i in range(len(occupied_buckets) - 1):
            if occupied_buckets[i + 1] - occupied_buckets[i] >= 3:
                gap_found = True
                gap_lo = occupied_buckets[i]
                gap_hi = occupied_buckets[i + 1]
                gap_description = (
                    f"Gap detected: no hunks in [{gap_lo + 1}, {gap_hi}). "
                    f"Lower cluster: ≤ {gap_lo}. Upper cluster: ≥ {gap_hi}."
                )
                break

    # Top-10 and bottom-10
    sorted_by_max = sorted(results, key=lambda r: r["max_score"], reverse=True)
    top_10 = sorted_by_max[:10]
    bottom_10 = sorted_by_max[-10:][::-1]

    # Verdict
    if p99 < 5 and mx <= 6:
        verdict = "**Well-calibrated**: p99 < 5 AND no hunks above 6."
        prod_rec = "Scorer is safe to deploy on faker-like code. False-positive risk is negligible."
    elif pct_above_5 <= 15:
        verdict = f"**Partial false positives**: {pct_above_5:.1f}% of hunks above 5."
        prod_rec = (
            f"{pct_above_5:.1f}% false-positive rate on ordinary code. "
            "Acceptable if break recall is high, but monitor in production."
        )
    else:
        verdict = (
            f"**Systemic hallucination**: {pct_above_5:.1f}% of hunks above 5 "
            f"OR distribution overlaps break range."
        )
        prod_rec = (
            "Scorer cannot be deployed without a domain-specific threshold. "
            "False-positive rate on ordinary code is too high."
        )

    if gap_found:
        bimodal_note = f"**Bimodal distribution detected.** {gap_description}"
    else:
        bimodal_note = "Distribution appears unimodal — no gap of ≥ 2 units found."

    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — BPE Contrastive TF-IDF: Faker Calibration Run (2026-04-21)",
        "",
        "**Goal:** Determine whether the scorer produces false positives on ordinary faker code.",
        "",
        "## 1. Summary Stats (max_score, n=159)",
        "",
        "| stat | value |",
        "|---|---|",
        f"| min | {mn:.4f} |",
        f"| p10 | {p10:.4f} |",
        f"| p25 | {p25:.4f} |",
        f"| p50 | {p50:.4f} |",
        f"| p75 | {p75:.4f} |",
        f"| p90 | {p90:.4f} |",
        f"| p99 | {p99:.4f} |",
        f"| max | {mx:.4f} |",
        f"| mean | {mean_val:.4f} |",
        f"| stdev | {stdev_val:.4f} |",
        f"| % above 5 | {pct_above_5:.1f}% |",
        f"| % above 6 | {pct_above_6:.1f}% |",
        "",
        "## 2. Histogram (bucket width 1.0)",
        "",
        "| score_band | count | pct |",
        "|---|---|---|",
    ]
    for b in sorted(histogram.keys()):
        cnt = histogram[b]
        pct = 100.0 * cnt / len(max_scores)
        lines.append(f"| [{b}, {b + 1}) | {cnt} | {pct:.1f}% |")

    lines += [
        "",
        "## 3. Reference Comparison",
        "",
        "| corpus | min | p50 | max | notes |",
        "|---|---|---|---|---|",
        "| FastAPI breaks | 5.9 | 8.7 | 11.1 | style violations |",
        "| FastAPI controls | 0.8 | 1.9 | 4.5 | ordinary FastAPI code |",
        "| rich breaks | 4.98 | 6.7 | 7.79 | style violations |",
        "| rich controls | 2.93 | 4.16 | 5.60 | ordinary rich code |",
        f"| **faker (this run)** | **{mn:.2f}** | **{p50:.2f}** | **{mx:.2f}** |"
        f" **ordinary faker code (n=159)** |",
        "",
        "## 4. Top-10 Highest-Scoring Hunks",
        "",
        "| name | file_path | max_score | mean_score | top_3_tokens |",
        "|---|---|---|---|---|",
    ]
    for r in top_10:
        top3 = ", ".join(r["top_3_tokens"])
        lines.append(
            f"| {r['name']} | {r['file_path']} | {r['max_score']:.4f}"
            f" | {r['mean_score']:.4f} | {top3} |"
        )

    lines += [
        "",
        "## 5. Bottom-10 Lowest-Scoring Hunks",
        "",
        "| name | file_path | max_score | mean_score | top_3_tokens |",
        "|---|---|---|---|---|",
    ]
    for r in bottom_10:
        top3 = ", ".join(r["top_3_tokens"])
        lines.append(
            f"| {r['name']} | {r['file_path']} | {r['max_score']:.4f}"
            f" | {r['mean_score']:.4f} | {top3} |"
        )

    lines += [
        "",
        "## 6. Bimodality Check",
        "",
        bimodal_note,
        "",
    ]

    if gap_found:
        # Identify which files drive upper cluster
        upper_threshold = gap_hi
        upper_hunks = [r for r in results if r["max_score"] >= upper_threshold]
        lower_hunks = [r for r in results if r["max_score"] < upper_threshold]
        lines += [
            f"Upper cluster ({len(upper_hunks)} hunks, max_score ≥ {upper_threshold}):",
            "",
        ]
        for r in sorted(upper_hunks, key=lambda x: x["max_score"], reverse=True)[:5]:
            lines.append(f"- `{r['file_path']}` — max={r['max_score']:.4f}")
        lines += [
            "",
            f"Lower cluster: {len(lower_hunks)} hunks with max_score < {upper_threshold}.",
            "",
        ]

    lines += [
        "## 7. Verdict + Production-Readiness Recommendation",
        "",
        verdict,
        "",
        f"**Production recommendation:** {prod_rec}",
        "",
        "### Thresholds applied",
        "- Well-calibrated: p99 < 5 AND no hunks above 6",
        "- Partial false positives: 5–15% of hunks above 5",
        "- Systemic hallucination: > 15% of hunks above 5 OR distribution overlaps break range (5–10)",  # noqa: E501
        "- Bimodal diagnostic: two clear clusters — identify what drives each",
        "",
    ]

    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(out: Path | None = None) -> dict[str, float]:
    print("Loading tokenizer...", flush=True)
    tokenizer = _get_tokenizer()
    vocab = tokenizer.get_vocab()
    id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

    print("Building Model A (faker corpus)...", flush=True)
    model_a, total_a = _build_model_a_bpe_faker(tokenizer)
    print(f"  Model A: {len(model_a)} unique tokens, {total_a} total", flush=True)

    print("Loading P_B (generic reference)...", flush=True)
    model_b, total_b = _load_model_b_bpe()
    print(f"  Model B: {len(model_b)} unique tokens, {total_b} total", flush=True)

    print("Loading sampled hunks...", flush=True)
    records = _load_sampled_hunks()
    print(f"  Loaded {len(records)} hunks", flush=True)

    print("Scoring hunks...", flush=True)
    results: list[dict[str, Any]] = []
    for rec in records:
        hunk_source: str = rec["hunk_source"]
        max_score, mean_score, n_tokens_filtered, top_3_tokens = _score_hunk(
            hunk_source, tokenizer, id_to_token, model_a, total_a, model_b, total_b
        )
        results.append(
            {
                "name": rec["name"],
                "file_path": rec["file_path"],
                "max_score": max_score,
                "mean_score": mean_score,
                "n_tokens_filtered": n_tokens_filtered,
                "top_3_tokens": top_3_tokens,
            }
        )

    max_scores = [r["max_score"] for r in results]
    p50 = _percentile(max_scores, 50)
    p90 = _percentile(max_scores, 90)
    p99 = _percentile(max_scores, 99)
    mx = max(max_scores)
    pct_above_5 = 100.0 * sum(1 for s in max_scores if s > 5) / len(max_scores)

    print(f"\nFaker calibration results (n={len(records)}):")
    print(f"  p50={p50:.4f}, p90={p90:.4f}, p99={p99:.4f}, max={mx:.4f}")
    print(f"  % above 5: {pct_above_5:.1f}%")

    if out is not None:
        _write_report(out, records, results)

    return {
        "p50": p50,
        "p90": p90,
        "p99": p99,
        "max": mx,
        "pct_above_5": pct_above_5,
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
        / "bpe_contrastive_tfidf_faker_2026-04-21.md"
    )
    run(out=out)


if __name__ == "__main__":
    main()
