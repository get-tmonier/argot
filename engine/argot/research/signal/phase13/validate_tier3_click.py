"""Phase 13 Stage 3 Tier 3 — cross-domain validation on click.

Builds model_A from click source files (excluding the 10 fixture controls),
scores 18 fixtures (10 controls + 8 synthetic breaks), computes AUC, writes
a markdown report.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/validate_tier3_click.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from argot.acceptance.runner import FixtureSpec, fixture_to_record
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer

FIXTURE_DIR = Path(__file__).parent / "tier3_fixtures" / "click"
MANIFEST_PATH = FIXTURE_DIR / "manifest.json"


def _load_fixtures() -> tuple[list[dict[str, Any]], list[bool], list[str], list[str]]:
    manifest = json.loads(MANIFEST_PATH.read_text())
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
        records.append(fixture_to_record(FIXTURE_DIR, spec, "file_only"))
        is_break.append(spec.is_break)
        names.append(spec.name)
        categories.append(spec.category)
    return records, is_break, names, categories


def _model_a_files(click_dir: Path) -> list[Path]:
    """All click/*.py files except those mirrored as fixture controls."""
    manifest = json.loads(MANIFEST_PATH.read_text())
    control_basenames = {
        Path(f["file"]).name
        for f in manifest["fixtures"]
        if not f["is_break"]
    }
    src = click_dir / "src" / "click"
    return [p for p in sorted(src.glob("*.py")) if p.name not in control_basenames]


def run(*, click_dir: Path, out: Path) -> dict[str, Any]:
    records, is_break, names, categories = _load_fixtures()
    model_a = _model_a_files(click_dir)
    if len(model_a) < 5:
        raise RuntimeError(
            f"Too few model_A files at {click_dir}/src/click ({len(model_a)}); "
            "expected click repo cloned at tag 8.1.7"
        )

    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    scorer.fit([], model_a_files=model_a)
    scores = scorer.score(records)

    break_scores = [s for s, b in zip(scores, is_break) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break) if not b]
    auc = auc_from_scores(break_scores, ctrl_scores)

    _write_report(out, auc, scores, is_break, names, categories, model_a)
    return {
        "auc": auc,
        "scores": scores,
        "names": names,
        "is_break": is_break,
        "model_a_count": len(model_a),
    }


def _verdict(auc: float) -> str:
    if auc >= 0.80:
        return f"**PASS** (AUC {auc:.4f} ≥ 0.80). Method generalises; proceed to Stage 4."
    if auc >= 0.65:
        return (
            f"**MIXED** (AUC {auc:.4f} in 0.65–0.80). Investigate per-category "
            "before any promotion decision."
        )
    return (
        f"**FAIL** (AUC {auc:.4f} < 0.65). Method is FastAPI-tuned; do not promote."
    )


def _write_report(
    out: Path,
    auc: float,
    scores: list[float],
    is_break: list[bool],
    names: list[str],
    categories: list[str],
    model_a: list[Path],
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 Stage 3 — Tier 3 Cross-Domain Validation (click)\n",
        "Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`",
        "",
        f"model_A: {len(model_a)} click source files "
        "(click/*.py minus 10 held-out controls)",
        "",
        f"**Overall AUC: {auc:.4f}**",
        "",
        "## Per-category scores",
        "",
        "| fixture | category | is_break | score |",
        "|---|---|---|---|",
    ]
    for n, c, b, s in zip(names, categories, is_break, scores):
        lines.append(f"| {n} | {c} | {b} | {s:.4f} |")
    lines += [
        "",
        "## Verdict",
        "",
        _verdict(auc),
        "",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--click-dir", required=True, type=Path)
    parser.add_argument(
        "--out", type=Path,
        default=Path("docs/research/scoring/signal/phase13/stage3_tier3_click_2026-04-21.md"),
    )
    args = parser.parse_args()
    result = run(click_dir=args.click_dir, out=args.out)
    print(f"Overall AUC: {result['auc']:.4f}", flush=True)


if __name__ == "__main__":
    main()
