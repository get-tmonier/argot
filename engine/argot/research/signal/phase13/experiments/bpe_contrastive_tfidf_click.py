# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py
"""Phase 13: BPE-contrastive TF-IDF on click (tier3 matched).

Mirrors bpe_contrastive_tfidf.py but uses click's 18-fixture matched manifest.
Hypothesis: BPE subword tokenization closes vocabulary holes in cross-repo evaluation.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/experiments/\\
            bpe_contrastive_tfidf_click_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from transformers import AutoTokenizer

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.research.signal.bootstrap import auc_from_scores

_EPSILON = 1e-7
_MODEL_NAME = "microsoft/unixcoder-base"
_FIXTURE_DIR = Path(__file__).parent.parent / "tier3_fixtures" / "click"
_MANIFEST_PATH = _FIXTURE_DIR / "manifest_matched.json"
_REFERENCE_PATH = (
    Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens_bpe.json"
)
_HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})


def _get_tokenizer() -> Any:
    return AutoTokenizer.from_pretrained(_MODEL_NAME)  # type: ignore[no-untyped-call]


def _load_model_b_bpe() -> tuple[dict[int, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a_bpe_click(click_dir: Path, tokenizer: Any) -> tuple[dict[int, int], int]:
    counts: Counter[int] = Counter()
    src = click_dir / "src" / "click"
    for path in sorted(src.glob("*.py")):
        if path.name in _HELD_OUT:
            continue
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=False)
        counts.update(ids)
    total = sum(counts.values())
    return dict(counts), total


def _load_fixtures() -> tuple[list[dict[str, Any]], list[bool], list[str], list[str]]:
    manifest = json.loads(_MANIFEST_PATH.read_text())
    records: list[dict[str, Any]] = []
    is_break: list[bool] = []
    names: list[str] = []
    categories: list[str] = []
    for f in manifest["fixtures"]:
        spec = FixtureSpec(
            name=f["name"],
            scope="default",
            file=f["file"],
            hunk_start_line=f["hunk_start_line"],
            hunk_end_line=f["hunk_end_line"],
            is_break=f["is_break"],
            rationale=f.get("rationale", ""),
            category=f.get("category", "control" if not f["is_break"] else "break"),
        )
        records.append(fixture_to_record(_FIXTURE_DIR, spec, "file_only"))
        is_break.append(spec.is_break)
        names.append(spec.name)
        categories.append(spec.category)
    return records, is_break, names, categories


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
        return (
            f"**Max-token saturation still present**: all {total} breaks share identical "
            f"score {break_scores[0]:.4f}. BPE did not resolve saturation."
        )
    return (
        f"Max-token saturation resolved: {unique}/{total} unique break scores "
        f"(word baseline had 1/8 unique scores — all 8 identical at 8.4418)."
    )


def _interpretation(auc: float, saturation_note: str) -> str:
    if auc >= 0.80:
        band = (
            f"AUC {auc:.4f} ≥ 0.80: **SUCCESS**. BPE subword tokenization fixed the vocabulary "
            "holes and saturation issues from the word-token baseline (0.7000). "
            "**Recommend promoting BPE-contrastive-tfidf as Phase 13 winner.**"
        )
    elif auc >= 0.70:
        band = (
            f"AUC {auc:.4f} in [0.70, 0.80): partial lift over word baseline (0.7000) or at parity."
            " BPE tokens are not the primary bottleneck; contrastive-MLM is justified as next step."
        )
    else:
        band = (
            f"AUC {auc:.4f} < 0.70: regression from word baseline (0.7000). "
            "BPE tokens made things worse — vocabulary holes are not the bottleneck. "
            "Recommend a context-aware approach (conditional distributions over context windows)."
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
        "# Phase 13 — BPE Contrastive TF-IDF on Click (Tier 3 Matched, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | tokenizer | AUC |",
        "|---|---|---|---|",
        "| contrastive_tfidf (word baseline) | click (v2 matched)"
        " | argot tokenize_lines | 0.7000 |",
        f"| **bpe_contrastive_tfidf** | **click (v2 matched)**"
        f" | **UnixCoder BPE** | **{overall_auc:.4f}** |",
        "",
        "## Per-Category AUC\n",
        "",
        "*(break category vs all 10 controls)*\n",
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


def run(*, click_dir: Path, out: Path | None = None) -> float:
    records, is_break, names, categories = _load_fixtures()

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 8, f"Expected 8 breaks, got {n_breaks}"
    assert n_ctrls == 10, f"Expected 10 controls, got {n_ctrls}"

    tokenizer = _get_tokenizer()
    model_a, total_a = _build_model_a_bpe_click(click_dir, tokenizer)
    model_b, total_b = _load_model_b_bpe()

    scores = score_records_bpe(records, tokenizer, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("contrastive_tfidf on click (word baseline):    0.7000")
    print(f"bpe_contrastive_tfidf on click (this run):     {overall_auc:.4f}")

    saturation_unique = len(set(break_scores))
    print(f"Break score unique values: {saturation_unique}/8 (word baseline: 1/8, all identical)")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="BPE Contrastive TF-IDF experiment — click")
    parser.add_argument("--click-dir", default="/tmp/click-clone", help="Path to click repo")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    run(click_dir=Path(args.click_dir), out=Path(args.out) if args.out else None)


if __name__ == "__main__":
    main()
