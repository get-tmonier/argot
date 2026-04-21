"""Phase 13 quick experiment: contrastive token TF-IDF on FastAPI.

Hypothesis: does the +0.28 AUC lift from ast_contrastive come from the contrastive
log-ratio alone, not AST treelets?  We apply the same formula to raw code tokens.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_tfidf.py \\
        --out docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.dataset import Language as Lang
from argot.research.signal.bootstrap import auc_from_scores
from argot.tokenize import tokenize_lines

_EPSILON = 1e-7
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
_REFERENCE_PATH = Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens.json"


def _load_model_b() -> tuple[dict[str, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[str, int] = raw["token_counts"]
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a(fastapi_dir: Path) -> tuple[dict[str, int], int]:
    counts: Counter[str] = Counter()
    lang: Lang = "python"
    for path in sorted((fastapi_dir / "fixtures" / "default").glob("control_*.py")):
        source = path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        tokens = tokenize_lines(lines, lang, 0, len(lines))
        counts.update(t.text for t in tokens)
    total = sum(counts.values())
    return dict(counts), total


def _hunk_texts(record: dict[str, Any]) -> list[str]:
    texts = [tok["text"] for tok in record["hunk_tokens"]]
    if len(texts) < 3:
        fixture_path = Path(record["_fixture_path"])
        lang: Lang = "python"
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        texts = [t.text for t in tokenize_lines(lines, lang, 0, len(lines))]
    return texts


def _score_one(
    record: dict[str, Any],
    model_a: dict[str, int],
    total_a: int,
    model_b: dict[str, int],
    total_b: int,
) -> float:
    hunk = _hunk_texts(record)
    scores = [
        math.log(model_b.get(t, 0) / total_b + _EPSILON)
        - math.log(model_a.get(t, 0) / total_a + _EPSILON)
        for t in hunk
    ]
    return max(scores) if scores else 0.0


def score_records(
    records: list[dict[str, Any]],
    model_a: dict[str, int],
    total_a: int,
    model_b: dict[str, int],
    total_b: int,
) -> list[float]:
    return [_score_one(r, model_a, total_a, model_b, total_b) for r in records]


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


def _interpretation(auc: float) -> str:
    if auc >= 0.85:
        return (
            f"AUC {auc:.4f} ≥ 0.85: the contrastive log-ratio formulation alone — applied "
            "to raw tokens rather than AST treelets — recovers most of the lift seen in "
            "ast_contrastive_max. The key innovation was the contrastive signal structure, "
            "not the AST treelet vocabulary. "
            "**Next step: pursue a contrastive MLM baseline** (e.g. CodeBERT "
            "log P_B(t) − log P_A(t)) to test whether a pre-trained token distribution "
            "outperforms the stdlib corpus."
        )
    if auc >= 0.76:
        return (
            f"AUC {auc:.4f} falls between 0.75 and 0.85. Both the contrastive formulation "
            "and the AST treelet vocabulary contribute to ast_contrastive's lift. The "
            "contrast is load-bearing but not sufficient alone; structural features also "
            "matter. Consider a contrastive MLM experiment alongside AST vocabulary "
            "refinement."
        )
    return (
        f"AUC {auc:.4f} ≤ 0.75: raw token contrast does not replicate ast_contrastive_max's "
        "lift. The AST treelet vocabulary was load-bearing — the contrast formula on surface "
        "tokens is insufficient. "
        "**Next step: structural features (AST treelets) must be preserved in any "
        "successor scorer; raw token MLM or TF-IDF contrastive approaches are unlikely "
        "to generalise.**"
    )


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | AUC |",
        "|---|---|",
        "| tfidf_anomaly (one-sided, existing) | 0.6968 |",
        "| ast_contrastive_max (AST + contrast) | 0.9742 |",
        f"| **contrastive_tfidf (tokens + contrast)** | **{overall_auc:.4f}** |",
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
        _interpretation(overall_auc),
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

    model_a, total_a = _build_model_a(fastapi_dir)
    model_b, total_b = _load_model_b()

    scores = score_records(records, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("tfidf_anomaly (one-sided, existing):     0.6968")
    print("ast_contrastive_max (AST + contrast):    0.9742")
    print(f"contrastive_tfidf (tokens + contrast):   {overall_auc:.4f}")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive TF-IDF experiment")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
