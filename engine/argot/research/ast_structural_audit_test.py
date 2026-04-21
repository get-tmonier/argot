"""AST structural scorer audit — Phase 10 config matrix + result caching.

Fits AST scorer configs (baseline_9, ctx, cooc, full, jepa_alone, all) on the
static chunk corpus, scores 51 FastAPI fixtures, blends with JEPA, caches results.

Run: uv run python -m argot.research.ast_structural_audit_test [--config all] [--fixtures-only]
"""

from __future__ import annotations

import argparse
import json
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
from argot.research.signal.base import REGISTRY
from argot.research.signal.scorers.ast_structural import (
    AstStructuralScorer,  # noqa: F401 — populates REGISTRY
)
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
)
from argot.validate import compute_auc

ENTRY_DIR = CATALOG_DIR / "fastapi"
_REPO_ROOT = Path(__file__).parent.parent.parent.parent

# ---------------------------------------------------------------------------
# Config matrix
# ---------------------------------------------------------------------------

CONFIG_MATRIX: dict[str, list[str]] = {
    "baseline_9": ["ast_structural_ll", "ast_structural_zscore", "ast_structural_oov"],
    "ctx": ["ast_structural_ctx_ll", "ast_structural_ctx_zscore", "ast_structural_ctx_oov"],
    "cooc": ["ast_structural_cooc_ll", "ast_structural_cooc_zscore", "ast_structural_cooc_oov"],
    "full": ["ast_structural_full_oov"],
    "jepa_alone": [],  # JEPA only — no AST scorers
}

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
# JEPA encoding + scoring with caching
# ---------------------------------------------------------------------------


def _load_or_encode_jepa(
    core_chunks: list[dict[str, Any]],
    fixture_records: list[dict[str, Any]],
    specs: list[FixtureSpec],
    head_sha: str,
    cache_dir: Path,
) -> tuple[list[float], list[float]]:
    """Return (jepa_fixture_scores, jepa_z_scores), using cache when valid."""
    import torch

    from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
    from argot.research.static_chunk_audit_test import NORMALIZE_EMBEDDINGS
    from argot.train import _texts_for_records

    enc_cache = cache_dir / "jepa_encoded.pt"
    fix_cache = cache_dir / "jepa_fixture_scores.pt"
    spec_names = [s.name for s in specs]

    # Try loading encoding cache
    ctx_x: Any = None
    hunk_x: Any = None
    if enc_cache.exists():
        data = torch.load(enc_cache, weights_only=False)
        if data.get("head_sha") == head_sha and data.get("n_chunks") == len(core_chunks):
            ctx_x = data["ctx_x"]
            hunk_x = data["hunk_x"]
            print("  [cache] Loaded JEPA encoding from cache", flush=True)

    # Try loading fixture scores cache
    if fix_cache.exists() and ctx_x is not None:
        fdata = torch.load(fix_cache, weights_only=False)
        if fdata.get("specs_names") == spec_names:
            jepa_scores: list[float] = fdata["scores"]
            jepa_z: list[float] = fdata["z_scores"]
            print("  [cache] Loaded JEPA fixture scores from cache", flush=True)
            return jepa_scores, jepa_z

    device = select_device()
    norm = NORMALIZE_EMBEDDINGS

    if ctx_x is None or hunk_x is None:
        print(f"\nPre-encoding {len(core_chunks)} core chunks with {ENCODER_MODEL} ...", flush=True)
        pretrained = PretrainedEncoder(device=device, model_name=ENCODER_MODEL)
        ctx_texts, hunk_texts = _texts_for_records(core_chunks)
        t0 = time.perf_counter()
        with torch.no_grad():
            ctx_x = pretrained.encode_texts(ctx_texts, normalize_embeddings=norm).cpu()
            hunk_x = pretrained.encode_texts(hunk_texts, normalize_embeddings=norm).cpu()
        del pretrained
        print(f"  done in {time.perf_counter() - t0:.1f}s", flush=True)
        torch.save(
            {"ctx_x": ctx_x, "hunk_x": hunk_x, "n_chunks": len(core_chunks), "head_sha": head_sha},
            enc_cache,
        )
        print(f"  [cache] Saved encoding to {enc_cache}", flush=True)

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
    jepa_scores = ensemble.score_from_preencoded(ctx_fx, hunk_fx)

    jepa_auc = _auc(specs, jepa_scores)
    jepa_d_raw = _delta(specs, jepa_scores)
    jepa_z = _z_normalize(jepa_scores)
    jepa_d_z = _delta(specs, jepa_z)
    print(
        f"  jepa: AUC={jepa_auc:.4f}  delta_raw={jepa_d_raw:+.4f}  delta_z={jepa_d_z:+.4f}",
        flush=True,
    )

    torch.save(
        {"specs_names": spec_names, "scores": jepa_scores, "z_scores": jepa_z},
        fix_cache,
    )
    print(f"  [cache] Saved fixture scores to {fix_cache}", flush=True)

    return jepa_scores, jepa_z


# ---------------------------------------------------------------------------
# Per-config evaluation
# ---------------------------------------------------------------------------


def _run_config(
    config_name: str,
    scorer_names: list[str],
    core_chunks: list[dict[str, Any]],
    fixture_records: list[dict[str, Any]],
    specs: list[FixtureSpec],
    jepa_fixture_scores: list[float],
    jepa_z_scores: list[float],
    cache_dir: Path,
) -> dict[str, Any]:
    """Fit scorers, score fixtures, blend with JEPA, return result dict."""
    print(f"\n=== Config: {config_name} ===", flush=True)

    scorer_results: list[dict[str, Any]] = []

    for scorer_name in scorer_names:
        factory = REGISTRY.get(scorer_name)
        if factory is None:
            print(f"  WARNING: scorer {scorer_name!r} not in REGISTRY — skipping", flush=True)
            continue

        scorer = factory()
        t0 = time.perf_counter()
        scorer.fit(core_chunks)
        print(f"  {scorer_name}: fit in {time.perf_counter() - t0:.2f}s", flush=True)

        raw_scores = scorer.score(fixture_records)
        auc_val = _auc(specs, raw_scores)
        delta_val = _delta(specs, raw_scores)
        per_cat = _per_cat_auc(specs, raw_scores)

        # Blend sweep (z-normalized)
        ast_z = _z_normalize(raw_scores)
        best_blend: dict[str, Any] = {}
        for w_ast in BLEND_WEIGHTS:
            blended = [
                (1 - w_ast) * j + w_ast * a for j, a in zip(jepa_z_scores, ast_z, strict=False)
            ]
            blend_auc = _auc(specs, blended)
            blend_delta = _delta(specs, blended)
            label = f"jepa+{scorer_name}@{w_ast:.2f}"
            print(
                f"    {label}: AUC={blend_auc:.4f}  delta_z={blend_delta:+.4f}",
                flush=True,
            )
            if not best_blend or blend_auc > best_blend["auc"]:
                best_blend = {
                    "label": label,
                    "w_ast": w_ast,
                    "auc": blend_auc,
                    "delta": blend_delta,
                    "scores": blended,
                }

        print(
            f"  {scorer_name}: AUC={auc_val:.4f}  delta={delta_val:+.4f}  "
            f"best_blend={best_blend.get('label', 'n/a')} "
            f"AUC={best_blend.get('auc', float('nan')):.4f}",
            flush=True,
        )

        scorer_results.append(
            {
                "name": scorer_name,
                "fixture_scores": raw_scores,
                "auc": auc_val,
                "delta": delta_val,
                "per_cat_auc": {k: v for k, v in per_cat.items() if v == v},
                "best_blend": {k: v for k, v in best_blend.items() if k != "scores"},
                "best_blend_scores": best_blend.get("scores", []),
            }
        )

    # jepa_alone config — just JEPA scores
    if config_name == "jepa_alone":
        jepa_auc = _auc(specs, jepa_fixture_scores)
        jepa_delta = _delta(specs, jepa_fixture_scores)
        jepa_per_cat = _per_cat_auc(specs, jepa_fixture_scores)
        jepa_z_auc = _auc(specs, jepa_z_scores)
        jepa_z_delta = _delta(specs, jepa_z_scores)
        print(
            f"  jepa_alone: AUC={jepa_auc:.4f}  delta={jepa_delta:+.4f}  "
            f"AUC(z)={jepa_z_auc:.4f}  delta(z)={jepa_z_delta:+.4f}",
            flush=True,
        )
        scorer_results.append(
            {
                "name": "jepa",
                "fixture_scores": jepa_fixture_scores,
                "auc": jepa_auc,
                "delta": jepa_delta,
                "per_cat_auc": {k: v for k, v in jepa_per_cat.items() if v == v},
                "best_blend": {
                    "label": "jepa_alone",
                    "w_ast": 0.0,
                    "auc": jepa_auc,
                    "delta": jepa_delta,
                },
                "best_blend_scores": jepa_fixture_scores,
            }
        )

    result: dict[str, Any] = {
        "config": config_name,
        "scorers": [
            {k: v for k, v in s.items() if k != "best_blend_scores"} for s in scorer_results
        ],
    }

    out_path = cache_dir / f"{config_name}_results.json"
    out_path.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"  Saved {out_path}", flush=True)

    return {"config": config_name, "scorers": scorer_results}


# ---------------------------------------------------------------------------
# Report writing
# ---------------------------------------------------------------------------


def _write_summary(
    specs: list[FixtureSpec],
    head_sha: str,
    n_chunks: int,
    n_core_train: int,
    jepa_fixture_scores: list[float],
    jepa_z_scores: list[float],
    config_results: list[dict[str, Any]],
    cache_dir: Path,
) -> Path:
    categories = sorted({s.category for s in specs})
    n_break = sum(s.is_break for s in specs)
    n_ctrl = sum(not s.is_break for s in specs)

    # Flatten all scorer rows for the headline table
    rows: list[tuple[str, float, float]] = []  # (label, auc, delta_z)
    # JEPA alone
    jepa_z_auc = _auc(specs, jepa_z_scores)
    jepa_z_delta = _delta(specs, jepa_z_scores)
    rows.append(("jepa (z)", jepa_z_auc, jepa_z_delta))

    for cfg in config_results:
        for s in cfg["scorers"]:
            if s["name"] == "jepa":
                continue  # already included
            raw_auc = s["auc"]
            raw_delta = s["delta"]
            rows.append((s["name"], raw_auc, raw_delta))
            bb = s.get("best_blend")
            if bb and bb.get("label") and bb["label"] != "jepa_alone":
                rows.append((bb["label"], bb["auc"], bb["delta"]))

    # Best overall
    best_row = max(rows, key=lambda r: r[1] if r[1] == r[1] else 0.0)

    md: list[str] = [
        "# Phase 10 — AST Config Matrix Evaluation",
        "",
        "## Setup",
        "",
        f"- Encoder: `{ENCODER_MODEL}`",
        f"- JEPA ensemble: `EnsembleInfoNCE(n={ENSEMBLE_N}, beta={WINNER_BETA}, tau={WINNER_TAU})`",
        f"- FastAPI HEAD: `{head_sha}`",
        f"- Static chunks: {n_chunks} total, {n_core_train} core used for training",
        f"- Fixtures: {len(specs)} total ({n_break} breaks, {n_ctrl} controls)",
        "- Configs: baseline_9, ctx, cooc, full, jepa_alone",
        "- Blend weights tested: 0.25 / 0.50 / 0.75 fraction AST",
        "",
        "## Headline Results",
        "",
        f"**Best overall: `{best_row[0]}` AUC={best_row[1]:.4f}  Δ(z)={best_row[2]:+.4f}**",
        "",
        "### All scorers + best blends",
        "",
        "| Scorer | AUC | Δ(z) |",
        "|--------|----:|-----:|",
    ]
    for label, auc_v, delta_v in rows:
        auc_s = f"{auc_v:.4f}" if auc_v == auc_v else "n/a"
        delta_s = f"{delta_v:+.4f}" if delta_v == delta_v else "n/a"
        marker = " **←best**" if label == best_row[0] else ""
        md.append(f"| {label}{marker} | {auc_s} | {delta_s} |")

    # Per-category AUC for best scorer
    md += [
        "",
        "## Per-category AUC",
        "",
        f"Best scorer: `{best_row[0]}`",
        "",
    ]

    # Collect per-cat data for jepa + all scorers
    scorer_labels: list[str] = ["jepa (z)"]
    scorer_scores_map: dict[str, list[float]] = {"jepa (z)": jepa_z_scores}

    for cfg in config_results:
        for s in cfg["scorers"]:
            if s["name"] == "jepa":
                continue
            scorer_labels.append(s["name"])
            scorer_scores_map[s["name"]] = s["fixture_scores"]
            bb = s.get("best_blend")
            if bb and bb.get("label") and bb["label"] != "jepa_alone":
                lbl = bb["label"]
                scorer_labels.append(lbl)
                scorer_scores_map[lbl] = s.get("best_blend_scores", [])

    header = "| Category |" + "".join(f" {n} |" for n in scorer_labels)
    sep = "|----------|" + "".join("------:|" for _ in scorer_labels)
    md += [header, sep]
    for cat in categories:
        row = f"| {cat} |"
        for lbl in scorer_labels:
            sc = scorer_scores_map.get(lbl, [])
            if not sc:
                row += " n/a |"
            else:
                cat_auc = _per_cat_auc(specs, sc).get(cat, float("nan"))
                row += f" {cat_auc:.4f} |" if cat_auc == cat_auc else " n/a |"
        md.append(row)

    md += [
        "",
        "## Per-config Best",
        "",
        "| Config | Best scorer | AUC | Best blend | Blend AUC |",
        "|--------|------------|----:|-----------|----------:|",
    ]
    for cfg in config_results:
        cfg_name = cfg["config"]
        scorers = cfg["scorers"]
        if not scorers:
            continue
        valid_scorers = [s for s in scorers if s["auc"] == s["auc"]]
        if not valid_scorers:
            continue
        best_s = max(valid_scorers, key=lambda x: x["auc"])
        bb = best_s.get("best_blend", {})
        blend_lbl = bb.get("label", "n/a") if bb else "n/a"
        blend_auc = bb.get("auc", float("nan")) if bb else float("nan")
        blend_s = f"{blend_auc:.4f}" if blend_auc == blend_auc else "n/a"
        md.append(
            f"| {cfg_name} | {best_s['name']} | {best_s['auc']:.4f} | {blend_lbl} | {blend_s} |"
        )

    out = cache_dir / "eval_summary.md"
    out.write_text("\n".join(md) + "\n", encoding="utf-8")
    print(f"\nSummary written to {out}", flush=True)
    return out


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--config",
        default="all",
        help="baseline_9|ctx|cooc|full|jepa_alone|all",
    )
    parser.add_argument(
        "--fixtures-only",
        action="store_true",
        help="Only validate fixture loading, skip scoring",
    )
    args = parser.parse_args()

    # --fixtures-only mode: just validate loading
    if args.fixtures_only:
        specs = load_manifest(ENTRY_DIR)
        print(f"Loaded manifest: {len(specs)} fixtures", flush=True)
        ok = 0
        err = 0
        for s in specs:
            try:
                rec = fixture_to_record(ENTRY_DIR, s)
                source = (ENTRY_DIR / s.file).read_text(encoding="utf-8")
                lines = source.splitlines()
                rec["_source_lines"] = lines[s.hunk_start_line - 1 : s.hunk_end_line]
                ok += 1
            except Exception as exc:
                print(f"  ERROR loading {s.name}: {exc}", flush=True)
                err += 1
        n_break = sum(s.is_break for s in specs)
        n_ctrl = sum(not s.is_break for s in specs)
        categories = sorted({s.category for s in specs})
        print(
            f"OK={ok}  ERR={err}  breaks={n_break}  controls={n_ctrl}",
            flush=True,
        )
        print(f"Categories: {categories}", flush=True)
        sys.exit(0 if err == 0 else 1)

    t_start = time.perf_counter()

    # Setup cache dir
    cache_dir = _REPO_ROOT / ".argot" / "phase10_results"
    cache_dir.mkdir(parents=True, exist_ok=True)

    # Determine which configs to run
    if args.config == "all":
        configs_to_run = list(CONFIG_MATRIX.keys())
    elif args.config in CONFIG_MATRIX:
        configs_to_run = [args.config]
    else:
        choices = ", ".join(CONFIG_MATRIX)
        msg = f"ERROR: unknown config {args.config!r}. Choose from: all, {choices}"
        print(msg, file=sys.stderr)
        sys.exit(1)

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

    # 4. Load fixtures — attach _source_lines so AST scorer gets parseable Python
    specs: list[FixtureSpec] = load_manifest(ENTRY_DIR)  # type: ignore[no-redef]
    fixture_records = []
    for s in specs:
        rec = fixture_to_record(ENTRY_DIR, s)
        source = (ENTRY_DIR / s.file).read_text(encoding="utf-8")
        lines = source.splitlines()
        rec["_source_lines"] = lines[s.hunk_start_line - 1 : s.hunk_end_line]
        fixture_records.append(rec)
    print(f"Loaded {len(specs)} fixtures", flush=True)

    # 5. Encode + train JEPA (with caching)
    jepa_fixture_scores, jepa_z_scores = _load_or_encode_jepa(
        core_chunks, fixture_records, specs, head_sha, cache_dir
    )

    # 6. Run each config
    all_config_results: list[dict[str, Any]] = []
    for cfg_name in configs_to_run:
        scorer_names = CONFIG_MATRIX[cfg_name]
        result = _run_config(
            config_name=cfg_name,
            scorer_names=scorer_names,
            core_chunks=core_chunks,
            fixture_records=fixture_records,
            specs=specs,
            jepa_fixture_scores=jepa_fixture_scores,
            jepa_z_scores=jepa_z_scores,
            cache_dir=cache_dir,
        )
        all_config_results.append(result)

    elapsed = time.perf_counter() - t_start
    print(f"\nTotal elapsed: {elapsed:.0f}s", flush=True)

    # 7. Write summary
    summary_path = _write_summary(
        specs=specs,
        head_sha=head_sha,
        n_chunks=len(chunks),
        n_core_train=len(core_chunks),
        jepa_fixture_scores=jepa_fixture_scores,
        jepa_z_scores=jepa_z_scores,
        config_results=all_config_results,
        cache_dir=cache_dir,
    )
    print(f"Phase 10 complete. Summary: {summary_path}", flush=True)


if __name__ == "__main__":
    main()
