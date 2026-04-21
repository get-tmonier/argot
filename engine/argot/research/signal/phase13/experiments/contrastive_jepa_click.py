"""Phase 13 experiment: contrastive-JEPA on click (tier3 matched).

Mirrors contrastive_jepa.py but trains JEPA_A on click source (minus held-out files)
built as a sliding-window corpus, then scores click tier3 matched fixtures.

JEPA_B remains the identity predictor (pretrained encoder, no fine-tuning).

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_jepa_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/experiments/contrastive_jepa_click_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.dataset import Language as Lang
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.phase13.experiments.contrastive_jepa import (
    _write_report,
    score_contrastive,
    train_jepa_a,
)
from argot.tokenize import tokenize_lines
from argot.train import ModelBundle

_FIXTURE_DIR = Path(__file__).parent.parent / "tier3_fixtures" / "click"
_MANIFEST_PATH = _FIXTURE_DIR / "manifest_matched.json"
_HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})

_WINDOW = 64
_STRIDE = 32


def build_click_corpus(click_dir: Path) -> list[dict[str, Any]]:
    """Sliding-window corpus from click source (held-out files excluded)."""
    records: list[dict[str, Any]] = []
    src = click_dir / "src" / "click"
    lang: Lang = "python"
    for file_idx, path in enumerate(sorted(src.glob("*.py"))):
        if path.name in _HELD_OUT:
            continue
        source = path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        tokens = [t.text for t in tokenize_lines(lines, lang, 0, len(lines))]
        for j in range(0, len(tokens) - _WINDOW, _STRIDE):
            ctx_toks = [{"text": t} for t in tokens[j : j + _WINDOW // 2]]
            hunk_toks = [{"text": t} for t in tokens[j + _WINDOW // 2 : j + _WINDOW]]
            records.append(
                {
                    "context_before": ctx_toks,
                    "hunk_tokens": hunk_toks,
                    "author_date_iso": str(file_idx * 100_000 + j),
                }
            )
    return records


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


def run(*, click_dir: Path, out: Path | None = None) -> float:
    corpus = build_click_corpus(click_dir)
    print(f"Click corpus: {len(corpus)} sliding-window records", flush=True)
    bundle_a: ModelBundle = train_jepa_a(corpus)

    records, is_break, names, categories = _load_fixtures()

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 8, f"Expected 8 breaks, got {n_breaks}"
    assert n_ctrls == 10, f"Expected 10 controls, got {n_ctrls}"

    scores = score_contrastive(bundle_a, records)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("ast_contrastive_max on click:              0.2500")
    print("contrastive_tfidf on click:                0.7000")
    print(f"contrastive_jepa on click (this run):      {overall_auc:.4f}")

    if overall_auc >= 0.80:
        print("PASS: click gate ≥ 0.80")
    elif overall_auc >= 0.70:
        print("PARTIAL: click 0.70–0.80 — context-aware contrast adds signal but not enough")
    else:
        print("FAIL: click < 0.70 — contrastive-JEPA is not the answer")

    if out is not None:
        _write_report(
            out,
            overall_auc,
            per_cat,
            names,
            scores,
            is_break,
            categories,
            corpus_name="click (v2 matched)",
            prior_auc_label="contrastive_tfidf (click)",
            prior_auc_value="0.7000",
        )

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive-JEPA experiment on click")
    parser.add_argument("--click-dir", default="/tmp/click-clone", help="Path to click repo")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    run(click_dir=Path(args.click_dir), out=Path(args.out) if args.out else None)


if __name__ == "__main__":
    main()
