"""AST structural scorer audit — FastAPI fixtures.

Fits three AstStructuralScorer variants (loglik, zscore, oov) on the static
chunk corpus, scores FastAPI fixtures, blends with JEPA, reports results.

Run: uv run python -m argot.research.ast_structural_audit_test
"""

from __future__ import annotations

import statistics
import sys
import time
from pathlib import Path
from typing import Any

import numpy as np

from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    fixture_to_record,
    load_manifest,
)
from argot.research.signal.scorers.ast_structural import AstStructuralScorer
from argot.research.static_chunk_audit_test import (
    ENCODER_MODEL,
    ENSEMBLE_N,
    FASTAPI_CLONE_DIR,
    WINNER_BETA,
    WINNER_TAU,
    WINNER_WARMUP,
    _clone_or_reuse,
    _EnsembleForAudit,
    _extract_chunks,
    _per_category_delta,
    _top20_composition,
)
from argot.validate import compute_auc

ENTRY_DIR = CATALOG_DIR / "fastapi"
_REPO_ROOT = Path(__file__).parent.parent.parent.parent

AST_VARIANTS: list[tuple[str, AstStructuralScorer]] = [
    ("ast_ll", AstStructuralScorer(variant="loglik")),
    ("ast_zscore", AstStructuralScorer(variant="zscore")),
    ("ast_oov", AstStructuralScorer(variant="oov")),
]
BLEND_WEIGHTS: list[float] = [0.25, 0.50, 0.75]  # fraction of AST in blend


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _z_normalize(scores: list[float]) -> list[float]:
    arr = np.array(scores, dtype=float)
    mu = float(arr.mean())
    sigma = float(arr.std())
    if sigma == 0.0:
        return [0.0] * len(scores)
    return [(float(s) - mu) / sigma for s in scores]


def _delta(specs: list[FixtureSpec], scores: list[float]) -> float:
    brk = [s for spec, s in zip(specs, scores, strict=False) if spec.is_break]
    ctl = [s for spec, s in zip(specs, scores, strict=False) if not spec.is_break]
    if not brk or not ctl:
        return float("nan")
    return statistics.mean(brk) - statistics.mean(ctl)


def _auc(specs: list[FixtureSpec], scores: list[float]) -> float:
    brk = [s for spec, s in zip(specs, scores, strict=False) if spec.is_break]
    ctl = [s for spec, s in zip(specs, scores, strict=False) if not spec.is_break]
    if not brk or not ctl:
        return float("nan")
    return compute_auc(ctl, brk)


def _per_cat_auc(
    specs: list[FixtureSpec],
    scores: list[float],
) -> dict[str, float]:
    by_cat: dict[str, tuple[list[float], list[float]]] = {}
    for spec, score in zip(specs, scores, strict=False):
        cat = spec.category
        if cat not in by_cat:
            by_cat[cat] = ([], [])
        if spec.is_break:
            by_cat[cat][0].append(score)
        else:
            by_cat[cat][1].append(score)
    result: dict[str, float] = {}
    for cat, (brk, ctl) in by_cat.items():
        if brk and ctl:
            result[cat] = compute_auc(ctl, brk)
        else:
            result[cat] = float("nan")
    return result


# ---------------------------------------------------------------------------
# Report writing
# ---------------------------------------------------------------------------


def _write_report(
    head_sha: str,
    n_chunks: int,
    n_core_train: int,
    specs: list[FixtureSpec],
    chunks: list[dict[str, Any]],
    jepa_fixture_scores: list[float],
    ast_results: list[tuple[str, list[float]]],
    blend_results: list[tuple[str, list[float]]],
) -> None:
    n_break = sum(s.is_break for s in specs)
    n_ctrl = sum(not s.is_break for s in specs)

    all_scorers = [("jepa", jepa_fixture_scores)] + ast_results + blend_results
    categories = sorted({s.category for s in specs})

    md: list[str] = [
        "# Phase 9 — AST Structural Scorer — 2026-04-21",
        "",
        "## Setup",
        "",
        f"- Encoder: `{ENCODER_MODEL}`",
        f"- JEPA ensemble: `EnsembleInfoNCE(n={ENSEMBLE_N}, beta={WINNER_BETA}, tau={WINNER_TAU})`",
        f"- FastAPI HEAD: `{head_sha}`",
        f"- Static chunks: {n_chunks} total, {n_core_train} core used for training",
        f"- Fixtures: {len(specs)} total ({n_break} breaks, {n_ctrl} controls)",
        "- AST variants: loglik, zscore, oov (fully corpus-derived, no hand-crafted rules)",
        "- Blend weights tested: 0.25/0.50/0.75 fraction AST",
        "- Command: `uv run python -m argot.research.ast_structural_audit_test`",
        "",
        "## Headline AUC",
        "",
        "| Scorer | AUC | Delta (break−ctrl) |",
        "|--------|----:|-------------------:|",
    ]
    for name, scores in all_scorers:
        auc = _auc(specs, scores)
        d = _delta(specs, scores)
        auc_str = f"{auc:.4f}" if auc == auc else "n/a"
        d_str = f"{d:+.4f}" if d == d else "n/a"
        md.append(f"| {name} | {auc_str} | {d_str} |")

    md += [
        "",
        "## Per-category AUC",
        "",
    ]
    header = "| Category |" + "".join(f" {name} |" for name, _ in all_scorers)
    sep = "|----------|" + "".join("------:|" for _ in all_scorers)
    md += [header, sep]
    for cat in categories:
        row = f"| {cat} |"
        for _, scores in all_scorers:
            cat_auc = _per_cat_auc(specs, scores).get(cat, float("nan"))
            row += f" {cat_auc:.4f} |" if cat_auc == cat_auc else " n/a |"
        md.append(row)

    md += [
        "",
        "## Per-category Delta",
        "",
    ]
    header2 = "| Category |" + "".join(f" {name} |" for name, _ in all_scorers)
    sep2 = "|----------|" + "".join("-------:|" for _ in all_scorers)
    md += [header2, sep2]
    for cat in categories:
        row = f"| {cat} |"
        for _, scores in all_scorers:
            cat_rows = _per_category_delta(specs, scores)
            cat_delta = next((d for c, _, _, d, _, _ in cat_rows if c == cat), float("nan"))
            row += f" {cat_delta:+.4f} |" if cat_delta == cat_delta else " n/a |"
        md.append(row)

    # Top-20 composition under each AST variant
    md += ["", "## Top-20 Corpus Composition (per AST variant)", ""]
    md += ["| Scorer | core | test | docs_scripts |", "|--------|-----:|-----:|-------------:|"]
    # chunk-level composition written later in main() after scoring
    md += ["", "## Discussion", ""]

    def _safe_auc(t: tuple[str, list[float]]) -> float:
        v = _auc(specs, t[1])
        return v if v == v else 0.0

    best_ast_name, best_ast_scores = max(ast_results, key=_safe_auc)
    best_blend_name, best_blend_scores = max(blend_results, key=_safe_auc)
    jepa_auc = _auc(specs, jepa_fixture_scores)
    best_ast_auc = _auc(specs, best_ast_scores)
    best_blend_auc = _auc(specs, best_blend_scores)

    md.append(
        f"Best AST variant standalone: `{best_ast_name}` AUC={best_ast_auc:.4f} "
        f"(JEPA baseline: {jepa_auc:.4f}). "
        f"Best blend: `{best_blend_name}` AUC={best_blend_auc:.4f}."
    )
    hard_cats = ["exception_handling", "async_blocking", "dependency_injection"]
    for cat in hard_cats:
        jepa_cat = _per_cat_auc(specs, jepa_fixture_scores).get(cat, float("nan"))
        ast_cat = _per_cat_auc(specs, best_ast_scores).get(cat, float("nan"))
        blend_cat = _per_cat_auc(specs, best_blend_scores).get(cat, float("nan"))
        if jepa_cat == jepa_cat:
            md.append(
                f"`{cat}`: JEPA={jepa_cat:.4f}  {best_ast_name}={ast_cat:.4f}  "
                f"blend={blend_cat:.4f}."
            )

    report_dir = _REPO_ROOT / "docs" / "research" / "scoring" / "signal"
    report_dir.mkdir(parents=True, exist_ok=True)
    report_path = report_dir / "phase9_ast_structural_2026-04-21.md"
    report_path.write_text("\n".join(md) + "\n", encoding="utf-8")
    print(f"\nReport written to {report_path}", flush=True)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    import torch

    t_start = time.perf_counter()

    # 1. Clone/reuse FastAPI
    head_sha = _clone_or_reuse()

    # 2. Extract chunks
    print(f"\nExtracting AST chunks from {FASTAPI_CLONE_DIR} ...", flush=True)
    t0 = time.perf_counter()
    chunks = _extract_chunks(FASTAPI_CLONE_DIR, head_sha)
    print(f"Extracted {len(chunks)} chunks in {time.perf_counter() - t0:.1f}s", flush=True)

    if not chunks:
        print("ERROR: no chunks extracted.", file=sys.stderr)
        sys.exit(1)

    # 3. Core-only filter for training
    core_chunks = [c for c in chunks if c["_file_class"] != "test"]
    print(f"Core training corpus: {len(core_chunks)} chunks", flush=True)

    # 4. Fit AST scorers on core corpus (fast — no encoding needed)
    print("\nFitting AST structural scorers ...", flush=True)
    for name, scorer in AST_VARIANTS:
        t0 = time.perf_counter()
        scorer.fit(core_chunks)
        print(f"  {name}: fit in {time.perf_counter() - t0:.2f}s", flush=True)

    # 5. Load fixtures — attach _source_lines so AST scorer gets parseable Python
    specs: list[FixtureSpec] = load_manifest(ENTRY_DIR)
    fixture_records = []
    for s in specs:
        rec = fixture_to_record(ENTRY_DIR, s)
        source = (ENTRY_DIR / s.file).read_text(encoding="utf-8")
        lines = source.splitlines()
        rec["_source_lines"] = lines[s.hunk_start_line - 1 : s.hunk_end_line]
        fixture_records.append(rec)
    print(f"\nLoaded {len(specs)} fixtures", flush=True)

    # 6. Score fixtures with AST scorers
    ast_fixture_scores: list[tuple[str, list[float]]] = []
    for name, scorer in AST_VARIANTS:
        scores = scorer.score(fixture_records)
        ast_fixture_scores.append((name, scores))
        auc = _auc(specs, scores)
        d = _delta(specs, scores)
        print(f"  {name}: AUC={auc:.4f}  delta={d:+.4f}", flush=True)

    # 7. Pre-encode + train JEPA for apples-to-apples comparison
    from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
    from argot.research.static_chunk_audit_test import NORMALIZE_EMBEDDINGS
    from argot.train import _texts_for_records

    print(f"\nPre-encoding {len(core_chunks)} core chunks with {ENCODER_MODEL} ...", flush=True)
    device = select_device()
    pretrained = PretrainedEncoder(device=device, model_name=ENCODER_MODEL)
    ctx_texts, hunk_texts = _texts_for_records(core_chunks)
    t0 = time.perf_counter()
    norm = NORMALIZE_EMBEDDINGS
    with torch.no_grad():
        ctx_x = pretrained.encode_texts(ctx_texts, normalize_embeddings=norm).cpu()
        hunk_x = pretrained.encode_texts(hunk_texts, normalize_embeddings=norm).cpu()
    del pretrained
    print(f"  done in {time.perf_counter() - t0:.1f}s", flush=True)
    core_preencoded = (ctx_x, hunk_x)

    print(f"\nTraining {ENSEMBLE_N}-member JEPA ensemble ...", flush=True)
    ensemble = _EnsembleForAudit(
        n=ENSEMBLE_N,
        beta=WINNER_BETA,
        tau=WINNER_TAU,
        warmup_epochs=WINNER_WARMUP,
    )
    ensemble.fit(core_chunks, preencoded=core_preencoded)

    print(f"\nEncoding {len(fixture_records)} fixtures ...", flush=True)
    pretrained2 = PretrainedEncoder(device=device, model_name=ENCODER_MODEL)
    ctx_fix, hunk_fix = _texts_for_records(fixture_records)
    with torch.no_grad():
        ctx_fx = pretrained2.encode_texts(ctx_fix, normalize_embeddings=norm).cpu()
        hunk_fx = pretrained2.encode_texts(hunk_fix, normalize_embeddings=norm).cpu()
    del pretrained2
    jepa_fixture_scores = ensemble.score_from_preencoded(ctx_fx, hunk_fx)

    jepa_auc = _auc(specs, jepa_fixture_scores)
    jepa_d = _delta(specs, jepa_fixture_scores)
    print(f"  jepa: AUC={jepa_auc:.4f}  delta={jepa_d:+.4f}", flush=True)

    # 8. Blend sweep (z-normalized)
    jepa_z = _z_normalize(jepa_fixture_scores)
    blend_results: list[tuple[str, list[float]]] = []
    print("\nBlend sweep (z-normalized JEPA + AST):", flush=True)
    for ast_name, ast_scores in ast_fixture_scores:
        ast_z = _z_normalize(ast_scores)
        for w_ast in BLEND_WEIGHTS:
            blended = [
                (1 - w_ast) * j + w_ast * a
                for j, a in zip(jepa_z, ast_z, strict=False)
            ]
            label = f"jepa+{ast_name}@{w_ast:.2f}"
            blend_results.append((label, blended))
            auc = _auc(specs, blended)
            d = _delta(specs, blended)
            print(f"  {label}: AUC={auc:.4f}  delta={d:+.4f}", flush=True)

    # 9. Top-20 composition under each AST variant on corpus
    print("\nTop-20 corpus composition (AST scoring on all chunks):", flush=True)
    for name, scorer in AST_VARIANTS:
        chunk_scores = scorer.score(chunks)
        comp = _top20_composition(chunks, chunk_scores)
        core, test, docs = comp.get("core", 0), comp.get("test", 0), comp.get("docs_scripts", 0)
        print(f"  {name}: core={core}  test={test}  docs_scripts={docs}", flush=True)

    elapsed = time.perf_counter() - t_start
    print(f"\nTotal elapsed: {elapsed:.0f}s", flush=True)

    _write_report(
        head_sha=head_sha,
        n_chunks=len(chunks),
        n_core_train=len(core_chunks),
        specs=specs,
        chunks=chunks,
        jepa_fixture_scores=jepa_fixture_scores,
        ast_results=ast_fixture_scores,
        blend_results=blend_results,
    )


if __name__ == "__main__":
    main()
