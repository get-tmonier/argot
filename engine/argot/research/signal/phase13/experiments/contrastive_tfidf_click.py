"""Phase 13 quick experiment: contrastive token TF-IDF on click (tier3 matched).

Does the contrastive log-ratio on raw tokens generalise cross-repo?
Mirrors contrastive_tfidf.py exactly but uses click's 18-fixture matched manifest.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_tfidf_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/experiments/contrastive_tfidf_click_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.dataset import Language as Lang
from argot.research.signal.bootstrap import auc_from_scores
from argot.tokenize import tokenize_lines

_EPSILON = 1e-7
_FIXTURE_DIR = Path(__file__).parent.parent / "tier3_fixtures" / "click"
_MANIFEST_PATH = _FIXTURE_DIR / "manifest_matched.json"
_REFERENCE_PATH = Path(__file__).parent.parent.parent.parent / "reference" / "generic_tokens.json"
_HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})


def _load_model_b() -> tuple[dict[str, int], int]:
    raw: dict[str, Any] = json.loads(_REFERENCE_PATH.read_text(encoding="utf-8"))
    token_counts: dict[str, int] = raw["token_counts"]
    total_tokens: int = raw["total_tokens"]
    return token_counts, total_tokens


def _build_model_a(click_dir: Path) -> tuple[dict[str, int], int]:
    counts: Counter[str] = Counter()
    lang: Lang = "python"
    src = click_dir / "src" / "click"
    for path in sorted(src.glob("*.py")):
        if path.name in _HELD_OUT:
            continue
        source = path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        counts.update(t.text for t in tokenize_lines(lines, lang, 0, len(lines)))
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
    if auc >= 0.80:
        return (
            f"AUC {auc:.4f} ≥ 0.80: contrastive-tokens generalises cross-repo. "
            "The raw token log-ratio — without any AST structure — is sufficient to detect "
            "foreign-framework imports (argparse, optparse, docopt, sys.argv) in click code. "
            "The identifier vocabulary alone carries the cross-repo signal. "
            "**Promote as Stage 4 candidate.** Skip the more expensive contrastive-MLM "
            "experiment — the simpler scorer already does the job."
        )
    if auc >= 0.55:
        return (
            f"AUC {auc:.4f} falls between 0.55 and 0.80: partial generalisation. "
            "The contrast formulation carries real cross-repo signal but is not sufficient alone. "
            "The contrastive-MLM experiment is now justified as the next step — a pre-trained "
            "token distribution may close the gap."
        )
    return (
        f"AUC {auc:.4f} < 0.55: contrastive-tokens is also framework-tuned. "
        "Neither the log-ratio formulation nor the raw identifier vocabulary generalises "
        "cross-repo from a single small corpus. Something deeper is wrong — fixture design, "
        "corpus structure, or the formulation itself. **Stop and rethink before more experiments.**"
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
        "# Phase 13 — Contrastive TF-IDF on Click (Tier 3 Matched, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | AUC |",
        "|---|---|---|",
        "| ast_contrastive_max | click (v2 matched) | 0.2500 |",
        "| contrastive_tfidf | FastAPI | 0.9847 |",
        f"| **contrastive_tfidf** | **click (v2 matched)** | **{overall_auc:.4f}** |",
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
        _interpretation(overall_auc),
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

    model_a, total_a = _build_model_a(click_dir)
    model_b, total_b = _load_model_b()

    scores = [_score_one(r, model_a, total_a, model_b, total_b) for r in records]

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("ast_contrastive_max on click (v2 matched fixtures):  0.2500")
    print("contrastive_tfidf on FastAPI:                        0.9847")
    print(f"contrastive_tfidf on click (this run):               {overall_auc:.4f}")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive TF-IDF experiment on click")
    parser.add_argument("--click-dir", default="/tmp/click-clone", help="Path to click repo")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    run(click_dir=Path(args.click_dir), out=Path(args.out) if args.out else None)


if __name__ == "__main__":
    main()
