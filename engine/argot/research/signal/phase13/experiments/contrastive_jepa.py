"""Phase 13 quick experiment: contrastive-JEPA on FastAPI.

JEPA_A: JepaCustomScorer (depth=6, mlp_dim=1024) trained on FastAPI corpus
JEPA_B: identity predictor — pretrained encoder, no fine-tuning

Score: max over embedding dims of (err_A[d] - err_B[d])
  err_A[d] = per-dim MSE(predictor_A(ctx_embed), hunk_embed)
  err_B[d] = per-dim squared-diff(ctx_embed, hunk_embed)  [identity predictor]

Also logs mean and top-5-mean aggregations in case max saturates.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_jepa.py \\
        --out docs/research/scoring/signal/phase13/experiments/contrastive_jepa_fastapi.md

"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import torch
import torch.nn.functional as F  # noqa: N812

from argot.acceptance.runner import fixture_to_record, load_manifest
from argot.dataset import Language as Lang
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.bootstrap import auc_from_scores
from argot.research.signal.scorers.jepa_custom import JepaCustomScorer
from argot.tokenize import tokenize_lines
from argot.train import ModelBundle

_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
_CORPUS_PATH = _FASTAPI_DIR / "corpus_file_only.jsonl"
_BATCH = 64


def load_fastapi_corpus(corpus_path: Path = _CORPUS_PATH) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with corpus_path.open() as fh:
        for line in fh:
            if line.strip():
                records.append(json.loads(line))
    return records


def train_jepa_a(corpus: list[dict[str, Any]]) -> ModelBundle:
    """Train JEPA_A predictor on the target repo corpus."""
    print(f"  Training JEPA_A on {len(corpus)} records ...", flush=True)
    scorer = JepaCustomScorer(
        epochs=20,
        lr=1e-4,
        lr_schedule="flat",
        predictor_overrides={"depth": 6, "mlp_dim": 1024},
        random_seed=0,
        aggregation="mean",
        corpus_cap=2000,
    )
    scorer.fit(corpus)
    bundle: ModelBundle | None = scorer._bundle
    assert bundle is not None, "JepaCustomScorer did not produce a bundle"
    return bundle


def _hunk_text(record: dict[str, Any]) -> str:
    texts = [tok["text"] for tok in record["hunk_tokens"]]
    if len(texts) < 3:
        fixture_path = Path(record["_fixture_path"])
        lang: Lang = "python"
        source = fixture_path.read_text(encoding="utf-8", errors="replace")
        lines = source.splitlines(keepends=True)
        texts = [t.text for t in tokenize_lines(lines, lang, 0, len(lines))]
    return " ".join(texts)


def score_contrastive(
    bundle_a: ModelBundle,
    records: list[dict[str, Any]],
) -> list[float]:
    """Return max-aggregated per-dim contrastive score (err_A - err_B) for each record."""
    if not records:
        return []

    encoder: PretrainedEncoder = bundle_a.vectorizer
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [_hunk_text(r) for r in records]

    with torch.no_grad():
        ctx_x = encoder.encode_texts(ctx_texts).cpu()  # (N, D)
        hunk_x = encoder.encode_texts(hunk_texts).cpu()  # (N, D)

    scores: list[float] = []
    bundle_a.model.eval()
    with torch.no_grad():
        for start in range(0, len(records), _BATCH):
            end = start + _BATCH
            ctx_b = ctx_x[start:end]
            hunk_b = hunk_x[start:end]
            z_pred = bundle_a.model.predict(ctx_b)  # (B, D)
            err_a = F.mse_loss(z_pred, hunk_b, reduction="none")  # (B, D)
            err_b = (ctx_b - hunk_b) ** 2  # (B, D)
            diff = err_a - err_b  # (B, D)
            scores.extend(diff.max(dim=-1).values.tolist())

    return scores


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
    if auc >= 0.90:
        return (
            f"AUC {auc:.4f} ≥ 0.90: contrastive-JEPA clears the FastAPI gate. "
            "The per-dimension (err_A − err_B) formulation transfers the contrastive "
            "log-ratio intuition to JEPA embedding space. Proceed to click validation."
        )
    return (
        f"AUC {auc:.4f} < 0.90: FastAPI gate not cleared. "
        "The contrastive-JEPA formulation is not working — err_A and err_B may be "
        "too correlated, cancelling the signal. Do not proceed to click."
    )


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores: list[float],
    is_break: list[bool],
    categories: list[str],
    corpus_name: str = "FastAPI",
    prior_auc_label: str = "ast_contrastive_max (FastAPI)",
    prior_auc_value: str = "0.9742",
) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        f"# Phase 13 — Contrastive-JEPA Experiment ({corpus_name}, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | AUC |",
        "|---|---|---|",
        "| tfidf_anomaly (production) | FastAPI | 0.6968 |",
        f"| {prior_auc_label} | {corpus_name} | {prior_auc_value} |",
        f"| **contrastive_jepa (this run)** | **{corpus_name}** | **{overall_auc:.4f}** |",
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


def run(
    fastapi_dir: Path = _FASTAPI_DIR,
    corpus_path: Path = _CORPUS_PATH,
    out: Path | None = None,
    device: torch.device | None = None,
) -> float:
    del device  # device selection is handled by PretrainedEncoder internally

    corpus = load_fastapi_corpus(corpus_path)
    bundle_a = train_jepa_a(corpus)

    specs = load_manifest(fastapi_dir)
    records = [fixture_to_record(fastapi_dir, spec) for spec in specs]

    is_break = [spec.is_break for spec in specs]
    categories = [spec.category for spec in specs]
    names = [spec.name for spec in specs]

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 31, f"Expected 31 breaks, got {n_breaks}"
    assert n_ctrls == 20, f"Expected 20 controls, got {n_ctrls}"

    scores = score_contrastive(bundle_a, records)

    break_scores = [s for s, b in zip(scores, is_break, strict=False) if b]
    ctrl_scores = [s for s, b in zip(scores, is_break, strict=False) if not b]
    overall_auc = auc_from_scores(break_scores, ctrl_scores)

    per_cat = _per_category_auc(scores, is_break, categories, ctrl_scores)

    print("tfidf_anomaly (production):              0.6968")
    print("ast_contrastive_max on FastAPI:          0.9742")
    print(f"contrastive_jepa on FastAPI (this run): {overall_auc:.4f}")

    if overall_auc >= 0.90:
        print("PASS: FastAPI gate ≥ 0.90")
    else:
        print("FAIL: FastAPI gate not cleared")

    if out is not None:
        _write_report(out, overall_auc, per_cat, names, scores, is_break, categories)

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive-JEPA experiment on FastAPI")
    parser.add_argument(
        "--fastapi-dir",
        default=str(_FASTAPI_DIR),
        help="Path to FastAPI catalog directory",
    )
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    run(
        fastapi_dir=Path(args.fastapi_dir),
        out=Path(args.out) if args.out else None,
        device=select_device(),
    )


if __name__ == "__main__":
    main()
