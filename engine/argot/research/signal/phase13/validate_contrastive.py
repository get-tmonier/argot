"""Phase 13 Stage 2 — generalisability validation for ContrastiveAstTreeletScorer.

Tiers:
  1.1  Smoke (model_A = stdlib = model_B) → expect scores ≈ 0
  1.2  LOO over 20 control files → AUC min/mean/max
  2    Wrong-contrast: Django view files as model_A

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/validate_contrastive.py \\
        --django-dir /tmp/django-clone \\
        --out docs/research/scoring/signal/phase13/stage2_validation_2026-04-21.md
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.cli.build_reference import _stdlib_root, _walk_py_files
from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer

CATALOG = Path(__file__).parent.parent.parent.parent / "acceptance" / "catalog"
FASTAPI_DIR = CATALOG / "fastapi"
FIXTURE_DEFAULT = FASTAPI_DIR / "fixtures" / "default"


def _build_scorer() -> ContrastiveAstTreeletScorer:
    return ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation="max")


def _load_fixture_records(
    context_mode: str = "file_only",
) -> tuple[list[dict], list[bool], list[str]]:
    specs = load_manifest(FASTAPI_DIR)
    records = [fixture_to_record(FASTAPI_DIR, spec, context_mode) for spec in specs]
    is_break = [spec.is_break for spec in specs]
    names = [spec.name for spec in specs]
    return records, is_break, names


def _compute_auc(scores: list[float], is_break: list[bool]) -> float:
    break_scores = [s for s, b in zip(scores, is_break) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break) if not b]
    return auc_from_scores(break_scores, ctrl_scores)


def _stdlib_py_files() -> list[Path]:
    return _walk_py_files(_stdlib_root())


def run_tier1_smoke(
    records: list[dict],
    is_break: list[bool],
    names: list[str],
) -> dict:
    """Build model_A from same stdlib files as model_B. Expect all scores ≈ 0."""
    print("  Tier 1.1: building model_A from stdlib files ...", flush=True)
    stdlib_files = _stdlib_py_files()
    print(f"    Found {len(stdlib_files)} stdlib .py files", flush=True)
    scorer = _build_scorer()
    scorer.fit([], model_a_files=stdlib_files)
    scores = scorer.score(records)
    max_abs = max(abs(s) for s in scores)
    rows = [{"name": n, "score": s, "is_break": b}
            for n, s, b in zip(names, scores, is_break)]
    passed = max_abs < 0.5
    return {"scores": rows, "max_abs_score": max_abs, "passed": passed}


def run_tier1_loo(
    records: list[dict],
    is_break: list[bool],
    names: list[str],
) -> dict:
    """For each of the 20 control files, exclude it, rebuild model_A, record AUC."""
    control_files = sorted(FIXTURE_DEFAULT.glob("control*.py"))
    print(f"  Tier 1.2: LOO over {len(control_files)} control files ...", flush=True)
    auc_rows: list[dict] = []
    for exclude in control_files:
        remaining = [f for f in control_files if f != exclude]
        scorer = _build_scorer()
        scorer.fit([], model_a_files=remaining)
        scores = scorer.score(records)
        auc = _compute_auc(scores, is_break)
        auc_rows.append({"excluded": exclude.name, "auc": auc})
        print(f"    excluded={exclude.name}  AUC={auc:.4f}", flush=True)
    aucs = [r["auc"] for r in auc_rows]
    return {
        "loo_rows": auc_rows,
        "min_auc": min(aucs),
        "mean_auc": sum(aucs) / len(aucs),
        "max_auc": max(aucs),
    }


def _django_view_files(django_dir: Path) -> list[Path]:
    """Return up to 20 Django view .py files from a cloned Django repo."""
    candidates: list[Path] = []
    candidates += sorted((django_dir / "django" / "views" / "generic").glob("*.py"))
    auth_views = django_dir / "django" / "contrib" / "auth" / "views.py"
    if auth_views.exists():
        candidates.append(auth_views)
    candidates += sorted((django_dir / "django" / "contrib" / "admin" / "views").glob("*.py"))
    return candidates[:20]


def run_tier2_wrong_contrast(
    records: list[dict],
    is_break: list[bool],
    names: list[str],
    django_dir: Path,
) -> dict:
    """Use Django view files as model_A; keep model_B = stdlib. Record AUC."""
    django_files = _django_view_files(django_dir)
    print(f"  Tier 2: using {len(django_files)} Django view files as model_A ...", flush=True)
    for f in django_files:
        print(f"    {f.relative_to(django_dir)}", flush=True)
    scorer = _build_scorer()
    scorer.fit([], model_a_files=django_files)
    scores = scorer.score(records)
    auc = _compute_auc(scores, is_break)
    rows = [{"name": n, "score": s, "is_break": b}
            for n, s, b in zip(names, scores, is_break)]
    return {
        "auc": auc,
        "delta_vs_baseline": auc - 0.9742,
        "django_files": [str(f.relative_to(django_dir)) for f in django_files],
        "scores": rows,
    }


def _write_report(
    out: Path,
    t11: dict,
    t12: dict,
    t2: dict,
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 Stage 2 — Contrastive AST Validation\n",
        "Baseline AUC (FastAPI, model_A=control files): **0.9742**\n",
        "",
        "## Tier 1.1 — Smoke Test (model_A = model_B = stdlib)\n",
        "",
        f"Expected: all scores ≈ 0.  **Result: {'PASS' if t11['passed'] else 'FAIL'}** "
        f"(max |score| = {t11['max_abs_score']:.4f})\n",
        "",
        "| fixture | is_break | score |",
        "|---|---|---|",
    ]
    for row in t11["scores"]:
        lines.append(f"| {row['name']} | {row['is_break']} | {row['score']:.4f} |")
    lines += [
        "",
        "## Tier 1.2 — Leave-One-Out over Control Files\n",
        "",
        "| excluded file | AUC |",
        "|---|---|",
    ]
    for row in t12["loo_rows"]:
        lines.append(f"| {row['excluded']} | {row['auc']:.4f} |")
    lines += [
        "",
        f"**min AUC:** {t12['min_auc']:.4f}  "
        f"**mean AUC:** {t12['mean_auc']:.4f}  "
        f"**max AUC:** {t12['max_auc']:.4f}\n",
        "",
        "## Tier 2 — Wrong-Contrast (Django view files as model_A)\n",
        "",
        "Django repo: `https://github.com/django/django` tag `4.2.16`\n",
        "",
        f"model_A files ({len(t2['django_files'])}):",
    ]
    for f in t2["django_files"]:
        lines.append(f"- `{f}`")
    delta_str = f"{t2['delta_vs_baseline']:+.4f}"
    lines += [
        "",
        f"**AUC with Django model_A:** {t2['auc']:.4f}  "
        f"(Δ vs baseline: {delta_str})\n",
        "",
        "## Verdict\n",
        "",
        "_PLACEHOLDER_VERDICT_\n",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--django-dir", required=True, type=Path,
        help="Path to a local Django git clone (tag 4.2.16 recommended)",
    )
    parser.add_argument(
        "--out", type=Path,
        default=Path("docs/research/scoring/signal/phase13/stage2_validation_2026-04-21.md"),
    )
    args = parser.parse_args()

    print("Loading FastAPI fixtures ...", flush=True)
    records, is_break, names = _load_fixture_records()
    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    print(f"  {len(records)} fixtures ({n_breaks} breaks, {n_ctrls} controls)\n")

    t11 = run_tier1_smoke(records, is_break, names)
    print(f"  Smoke result: max|score|={t11['max_abs_score']:.4f}  {'PASS' if t11['passed'] else 'FAIL'}\n")

    t12 = run_tier1_loo(records, is_break, names)
    print(f"  LOO AUC: min={t12['min_auc']:.4f} mean={t12['mean_auc']:.4f} max={t12['max_auc']:.4f}\n")

    t2 = run_tier2_wrong_contrast(records, is_break, names, args.django_dir)
    print(f"  Wrong-contrast AUC: {t2['auc']:.4f}  Δ={t2['delta_vs_baseline']:+.4f}\n")

    _write_report(args.out, t11, t12, t2)


if __name__ == "__main__":
    main()
