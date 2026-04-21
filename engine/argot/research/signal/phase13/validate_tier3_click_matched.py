"""Phase 13 Stage 3 — Tier 3 methodology-controlled rerun on click.

Rerun with two structural fixes vs. validate_tier3_click.py:
  1. held-out set shrunk 10 → 3 (model_A grows 6 → 13 files)
  2. controls are ~20-line hunks, matched to break hunk sizes

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/validate_tier3_click_matched.py \\
        --click-dir /tmp/click-clone \\
        --out docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md
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
MANIFEST_PATH = FIXTURE_DIR / "manifest_matched.json"

# The 3 held-out click source filenames. Controls are hunks drawn from these.
HELD_OUT: frozenset[str] = frozenset({"decorators.py", "types.py", "core.py"})

# Prior-run AUC (see stage3_tier3_click_2026-04-21.md) — referenced in report comparison.
V1_AUC = 0.1187


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
    """All click/*.py files except the 3 held-out sources."""
    src = click_dir / "src" / "click"
    return [p for p in sorted(src.glob("*.py")) if p.name not in HELD_OUT]


def run(*, click_dir: Path, out: Path) -> dict[str, Any]:
    records, is_break, names, categories = _load_fixtures()
    model_a = _model_a_files(click_dir)
    if len(model_a) < 10:
        raise RuntimeError(
            f"Too few model_A files at {click_dir}/src/click ({len(model_a)}); "
            "expected click repo cloned at tag 8.1.7 (16 files, 13 after hold-out)"
        )

    scorer = ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")
    scorer.fit([], model_a_files=model_a)
    scores = scorer.score(records)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
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
        return (
            f"**PASS** (AUC {auc:.4f} ≥ 0.80). Method generalises once corpus and hunk "
            "sizes are matched; proceed to Stage 4."
        )
    if auc >= 0.65:
        return (
            f"**MIXED** (AUC {auc:.4f} in 0.65–0.80). Method shows signal after "
            "methodology fixes but not enough to promote — investigate per-category."
        )
    return (
        f"**FAIL** (AUC {auc:.4f} < 0.65). Method does not generalise to click even "
        "with corpus and hunk sizes matched; prior-run abandonment recommendation stands."
    )


def _comparison(auc: float) -> str:
    delta = auc - V1_AUC
    if auc >= 0.65:
        interp = (
            "Recovery above the FAIL threshold. The v1 result was an artifact of the "
            "6-file model_A and whole-file controls, not evidence that the scorer is "
            "FastAPI-tuned. Remediation (larger corpus / matched hunk sizes), not "
            "abandonment, was the correct response."
        )
    elif delta >= 0.30:
        interp = (
            "Partial recovery. Methodology fixes account for a large share of v1's "
            "failure, but the scorer still underperforms the gate — genuine "
            "cross-domain weakness, not purely an artifact."
        )
    else:
        interp = (
            "No meaningful recovery. Methodology fixes do not rescue the scorer — "
            "v1's 'FastAPI-tuned' verdict is supported by this rerun."
        )
    return (
        f"Prior run (v1, 6-file model_A, whole-file controls): **AUC {V1_AUC:.4f}**\n\n"
        f"This run (v2, 13-file model_A, size-matched controls): **AUC {auc:.4f}** "
        f"(Δ = {delta:+.4f})\n\n"
        f"{interp}"
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
        "# Phase 13 Stage 3 — Tier 3 Methodology-Controlled Rerun (click)\n",
        "Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`",
        "",
        f"model_A: {len(model_a)} click source files "
        "(click/*.py minus 3 held-out files: decorators.py, types.py, core.py)",
        "",
        "Controls: 10 × ~20-line hunks drawn from the 3 held-out files "
        "(size-matched to break hunks).",
        "",
        f"**Overall AUC: {auc:.4f}**",
        "",
        "## Per-fixture scores",
        "",
        "| fixture | category | is_break | score |",
        "|---|---|---|---|",
    ]
    for n, c, b, s in zip(names, categories, is_break, scores, strict=False):
        lines.append(f"| {n} | {c} | {b} | {s:.4f} |")
    lines += [
        "",
        "## Verdict",
        "",
        _verdict(auc),
        "",
        "## Comparison to v1",
        "",
        _comparison(auc),
        "",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--click-dir", required=True, type=Path)
    parser.add_argument(
        "--out", type=Path,
        default=Path(
            "docs/research/scoring/signal/phase13/"
            "stage3_tier3_click_matched_2026-04-21.md"
        ),
    )
    args = parser.parse_args()
    result = run(click_dir=args.click_dir, out=args.out)
    print(f"Overall AUC: {result['auc']:.4f}", flush=True)


if __name__ == "__main__":
    main()
