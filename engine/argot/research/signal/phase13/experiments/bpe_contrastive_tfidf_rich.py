# engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_rich.py
"""Phase 13: BPE-contrastive TF-IDF on rich (terminal rendering domain).

Mirrors bpe_contrastive_tfidf_click.py but uses the rich catalog with 10 breaks + 10 controls.
The rich catalog sources are already in the repo under acceptance/catalog/rich/sources/model_a/.

Key difference from click: applies len >= 3 + alphanumeric token filter to avoid saturation
on punctuation tokens like `_` and `]`.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/bpe_contrastive_tfidf_rich.py
"""

from __future__ import annotations

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

# Script is at engine/argot/research/signal/phase13/experiments/
# 5 parents up = engine/argot/
_ARGOT_DIR = Path(__file__).parent.parent.parent.parent.parent
_CATALOG_DIR = _ARGOT_DIR / "acceptance" / "catalog" / "rich"
_MANIFEST_PATH = _CATALOG_DIR / "manifest.json"
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


def _build_model_a_bpe_rich(tokenizer: Any) -> tuple[dict[int, int], int]:
    counts: Counter[int] = Counter()
    for path in sorted(_MODEL_A_DIR.glob("*.py")):
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
        # manifest file entries are relative to _CATALOG_DIR
        records.append(fixture_to_record(_CATALOG_DIR, spec, "file_only"))
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
    hunk_source = "".join(lines[hunk_start:hunk_end]) if hunk_end > hunk_start else source
    ids: list[int] = tokenizer.encode(hunk_source, add_special_tokens=False)
    if not ids:
        ids = tokenizer.encode(source, add_special_tokens=False)
    return ids


def _score_one_bpe(
    record: dict[str, Any],
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> float:
    ids = _hunk_bpe_ids(record, tokenizer)

    def _is_meaningful_token(token_str: str) -> bool:
        return len(token_str) >= 3 and any(c.isalnum() for c in token_str)

    filtered_ids = [i for i in ids if _is_meaningful_token(id_to_token.get(i, ""))]
    # Fall back to all ids if filter removes everything
    if not filtered_ids:
        filtered_ids = ids

    scores = [
        math.log(model_b.get(i, 0) / total_b + _EPSILON)
        - math.log(model_a.get(i, 0) / total_a + _EPSILON)
        for i in filtered_ids
    ]
    return max(scores) if scores else 0.0


def score_records_bpe(
    records: list[dict[str, Any]],
    tokenizer: Any,
    id_to_token: dict[int, str],
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> list[float]:
    return [
        _score_one_bpe(r, tokenizer, id_to_token, model_a, total_a, model_b, total_b)
        for r in records
    ]


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
            f"score {break_scores[0]:.4f}. Token filter did not resolve saturation."
        )
    return (
        f"Max-token saturation resolved: {unique}/{total} unique break scores "
        f"(token filter applied: len >= 3 + alphanumeric)."
    )


def _interpretation(auc: float, saturation_note: str) -> str:
    if auc >= 0.80:
        band = (
            f"AUC {auc:.4f} >= 0.80: **SUCCESS**. BPE subword tokenization with token filter "
            "successfully distinguishes rich-style breaks from controls. "
            "**Recommend promoting BPE-contrastive-tfidf as Phase 13 winner.**"
        )
    elif auc >= 0.70:
        band = (
            f"AUC {auc:.4f} in [0.70, 0.80): partial signal on rich domain. "
            "BPE tokens provide some discriminative power but not sufficient for gate pass."
        )
    else:
        band = (
            f"AUC {auc:.4f} < 0.70: weak signal on rich domain. "
            "BPE-contrastive-tfidf does not generalize well across framework domains. "
            "Consider a context-aware approach or domain-specific corpus weighting."
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
        "# Phase 13 — BPE Contrastive TF-IDF on Rich (Terminal Rendering, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | tokenizer | AUC |",
        "|---|---|---|---|",
        f"| **bpe_contrastive_tfidf** | **rich (v15.0.0, 72 files)**"
        f" | **UnixCoder BPE + token filter** | **{overall_auc:.4f}** |",
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


def run(out: Path | None = None) -> float:
    records, is_break, names, categories = _load_fixtures()

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 10, f"Expected 10 breaks, got {n_breaks}"
    assert n_ctrls == 10, f"Expected 10 controls, got {n_ctrls}"

    tokenizer = _get_tokenizer()
    vocab = tokenizer.get_vocab()
    id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

    model_a, total_a = _build_model_a_bpe_rich(tokenizer)
    model_b, total_b = _load_model_b_bpe()

    scores = score_records_bpe(records, tokenizer, id_to_token, model_a, total_a, model_b, total_b)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print(f"bpe_contrastive_tfidf on rich (this run):      {overall_auc:.4f}")
    saturation_unique = len(set(break_scores))
    print(
        f"Break score unique values: {saturation_unique}/10 "
        f"(token filter: len>=3 + alphanumeric)"
    )
    print("\nPer-category AUC:")
    for cat, (n, cat_auc) in per_cat.items():
        print(f"  {cat}: n={n}, AUC={cat_auc:.4f}")
    print("\nPer-fixture scores:")
    for name, cat, b, s in zip(names, categories, is_break, scores, strict=False):
        label = "BREAK" if b else "ctrl"
        print(f"  [{label}] {name} ({cat}): {s:.4f}")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main() -> None:
    run()


if __name__ == "__main__":
    main()
