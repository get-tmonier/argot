# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py
"""Phase 13: BPE-contrastive TF-IDF on FastAPI.

Hypothesis: swapping word-level tokenization for UnixCoder BPE subword tokenization
fixes (1) single-token max saturation and (2) held-out vocabulary holes seen in
the word-token baseline (AUC 0.9847).

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf.py \\
        --out docs/research/scoring/signal/phase13/experiments/\\
            bpe_contrastive_tfidf_fastapi_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.bootstrap import auc_from_scores

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
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


def _build_model_a_bpe(fastapi_dir: Path, tokenizer: Any) -> tuple[dict[int, int], int]:
    counts: Counter[int] = Counter()
    for path in sorted((fastapi_dir / "fixtures" / "default").glob("control_*.py")):
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        counts.update(ids)
    total = sum(counts.values())
    return dict(counts), total


def _hunk_bpe_ids(record: dict[str, Any], tokenizer: Any) -> list[int]:
    fixture_path = Path(record["_fixture_path"])
    hunk_start = record.get("hunk_start_line", 0)
    hunk_end = record.get("hunk_end_line", 0)
    source = fixture_path.read_text(encoding="utf-8", errors="replace")
    lines = source.splitlines(keepends=True)
    # Use hunk slice when line range is non-empty; fall back to full file if BPE yields nothing
    hunk_source = "".join(lines[hunk_start:hunk_end]) if hunk_end > hunk_start else source
    ids: list[int] = tokenizer.encode(hunk_source, add_special_tokens=False)
    if not ids:
        ids = tokenizer.encode(source, add_special_tokens=False)
    return ids


def _score_one_bpe(
    record: dict[str, Any],
    tokenizer: Any,
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> float:
    ids = _hunk_bpe_ids(record, tokenizer)
    scores = [
        math.log(model_b.get(i, 0) / total_b + _EPSILON)
        - math.log(model_a.get(i, 0) / total_a + _EPSILON)
        for i in ids
    ]
    return max(scores) if scores else 0.0


def score_records_bpe(
    records: list[dict[str, Any]],
    tokenizer: Any,
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[float]:
    return [_score_one_bpe(r, tokenizer, model_a, total_a, model_b, total_b) for r in records]


def _per_category_auc(
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
    ctrl_scores: list[float],
) -> dict[str, tuple[int, float]]:
    cat_breaks: dict[str, list[float]] = defaultdict(list)
    for s, b, cat in zip(scores, is_break, categories, strict=False):
        if b:
            cat_breaks[cat].append(s)
    return {
        cat: (len(cat_s), auc_from_scores(cat_s, ctrl_scores))
        for cat, cat_s in sorted(cat_breaks.items())
    }


def _saturation_check(break_scores: list[float]) -> str:
    unique = len(set(break_scores))
    total = len(break_scores)
    if unique == 1:
        score = break_scores[0]
        return (
            f"**Max-token saturation present**: all {total} breaks share identical"
            f" score {score:.4f}."
        )
    return (
        f"Max-token saturation resolved: {unique}/{total} unique break scores"
        " (word baseline had 1/8)."
    )


def _interpretation(auc: float, saturation_note: str) -> str:
    if auc >= 0.90:
        band = (
            f"AUC {auc:.4f} ≥ 0.90: FastAPI gate passed. "
            "BPE tokenization preserves the word-token baseline signal. Proceed to click."
        )
    elif auc >= 0.80:
        band = (
            f"AUC {auc:.4f} falls between 0.80 and 0.90: minor regression from word baseline"
            " (0.9847). BPE did not improve FastAPI; may still be worth running click to check"
            " cross-repo lift."
        )
    else:
        band = (
            f"AUC {auc:.4f} < 0.80: FastAPI gate FAILED — BPE tokenizer broke something. "
            "Stop and diagnose before proceeding to click."
        )
    return f"{band}\n\n{saturation_note}"


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    saturation_note = _saturation_check(break_scores)
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — BPE Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | tokenizer | AUC |",
        "|---|---|---|",
        "| contrastive_tfidf (word baseline) | argot tokenize_lines | 0.9847 |",
        f"| **bpe_contrastive_tfidf** | **UnixCoder BPE** | **{overall_auc:.4f}** |",
        "",
        "## Per-Category AUC\n",
        "",
        "*(break category vs all controls)*\n",
        "",
        "| category | n_breaks | AUC |",
        "|---|---|---|",
    ]
    for cat, (n, cat_auc) in per_cat.items():
        lines.append(f"| {cat} | {n} | {cat_auc:.4f} |")
    lines += [
        "",
        "## Fixture Scores\n",
        "",
        "| fixture | category | is_break | score |",
        "|---|---|---|---|",
    ]
    for name, cat, b, s in zip(names, categories, is_break, scores, strict=False):
        lines.append(f"| {name} | {cat} | {b} | {s:.4f} |")
    lines += [
        "",
        "## Interpretation\n",
        "",
        _interpretation(overall_auc, saturation_note),
        "",
    ]
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(fastapi_dir: Path = _FASTAPI_DIR, out: Path | None = None) -> float:
    specs = load_manifest(fastapi_dir)
    records = [fixture_to_record(fastapi_dir, spec) for spec in specs]

    is_break = [spec.is_break for spec in specs]
    categories = [spec.category for spec in specs]
    names = [spec.name for spec in specs]

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 31, f"Expected 31 breaks, got {n_breaks}"
    assert n_ctrls == 20, f"Expected 20 controls, got {n_ctrls}"

    tokenizer = _get_tokenizer()
    model_a, total_a = _build_model_a_bpe(fastapi_dir, tokenizer)
    model_b, total_b = _load_model_b_bpe()

    scores = score_records_bpe(records, tokenizer, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("contrastive_tfidf (word baseline):  0.9847")
    print(f"bpe_contrastive_tfidf (this run):   {overall_auc:.4f}")

    if overall_auc < 0.90:
        print("WARNING: FastAPI gate FAILED (< 0.90). Stop before running click.")
    else:
        print("FastAPI gate passed (≥ 0.90). Proceed to click.")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="BPE Contrastive TF-IDF experiment — FastAPI")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
