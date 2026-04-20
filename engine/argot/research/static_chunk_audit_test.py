"""Feasibility test: fully git-history-free audit via AST-extracted static chunks.

No commit corpus needed. Extracts FunctionDef/AsyncFunctionDef/ClassDef bodies from
FastAPI HEAD, trains the Stage 6 winner ensemble on those static chunks, then scores
them to surface anomalies.

Run as: uv run python -m argot.research.static_chunk_audit_test
"""

from __future__ import annotations

import ast
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, cast

import numpy as np
import torch
from sklearn.metrics import precision_recall_curve, roc_curve

from argot.acceptance.runner import (
    CATALOG_DIR,
    FixtureSpec,
    fixture_to_record,
    load_manifest,
)
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.scorers.jepa_infonce import JepaInfoNCEScorer
from argot.tokenize import language_for_path, tokenize_lines
from argot.train import _texts_for_records
from argot.validate import compute_auc, compute_percentiles, score_from_tensors

# Stage 6 winner config (b01_t01_w0)
WINNER_BETA: float = 0.1
WINNER_TAU: float = 0.1
WINNER_WARMUP: int = 0
ENSEMBLE_N: int = 3

# UnixCoder: RoBERTa-based, 768-dim, HF direct backend (mean-pool last hidden state).
# normalize_embeddings=False: JEPA predictor ends with LayerNorm (magnitude ≈ sqrt(768) ≈ 27).
# Unit-sphere targets (magnitude=1) create a 27× scale gap that collapses MSE signal — every
# chunk scores ≈ 0.90, delta ≈ 0. Raw UnixCoder embeddings (magnitude ≈ 43) are much closer
# to the predictor's output scale, letting JEPA learn directional prediction.
ENCODER_MODEL: str = "microsoft/unixcoder-base"
NORMALIZE_EMBEDDINGS: bool = False

FASTAPI_CLONE_DIR = Path("/tmp/argot-fastapi-static")
ENTRY_DIR = CATALOG_DIR / "fastapi"

# Baseline: commit-hunk holdout scorer on the 2000-record FastAPI catalog corpus
BASELINE_CORPUS_MEAN: float = 0.6907
BASELINE_P95: float = 1.2123
BASELINE_N: int = 2000
BASELINE_BREAK_MEAN: float = 1.2265
BASELINE_CTRL_MEAN: float = 1.1333


# ---------------------------------------------------------------------------
# Data acquisition
# ---------------------------------------------------------------------------


def _clone_or_reuse() -> str:
    """Clone FastAPI at HEAD or reuse existing clone. Returns HEAD SHA."""
    if not FASTAPI_CLONE_DIR.exists():
        print(f"Cloning FastAPI into {FASTAPI_CLONE_DIR} ...", flush=True)
        subprocess.run(
            [
                "git",
                "clone",
                "--depth",
                "1",
                "https://github.com/tiangolo/fastapi",
                str(FASTAPI_CLONE_DIR),
            ],
            check=True,
        )
    else:
        print(f"Reusing existing clone at {FASTAPI_CLONE_DIR}", flush=True)
    result = subprocess.run(
        ["git", "-C", str(FASTAPI_CLONE_DIR), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    )
    sha = result.stdout.strip()
    print(f"FastAPI HEAD: {sha}", flush=True)
    return sha


def _extract_chunks(repo_dir: Path, head_sha: str) -> list[dict[str, Any]]:
    """Walk all .py files and extract FunctionDef/AsyncFunctionDef/ClassDef chunks.

    Context = full preceding file content (lines 0..hunk_start). sentence-transformers
    truncates from the right at max_seq_length=512, so this naturally gives the encoder
    the file header (imports, class declarations) — the most discriminative context.
    A 50-line window was tested and hurt delta (0.1817 → 0.0624); full context confirmed better.

    Only extracts nodes whose parent is Module or ClassDef (no functions nested inside
    functions), eliminating correlated examples that share near-identical context.
    """
    chunks: list[dict[str, Any]] = []
    for py_file in sorted(repo_dir.rglob("*.py")):
        try:
            source = py_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        lines = source.splitlines()
        try:
            tree = ast.parse(source, filename=str(py_file))
        except SyntaxError:
            continue
        rel_path = str(py_file.relative_to(repo_dir))
        lang = language_for_path(rel_path) or "python"

        # parent map: skip nodes nested inside functions (keep module-level and class methods)
        parent: dict[int, ast.AST] = {}
        for p in ast.walk(tree):
            for child in ast.iter_child_nodes(p):
                parent[id(child)] = p

        for node in ast.walk(tree):
            if not isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                continue
            node_parent = parent.get(id(node))
            if not isinstance(node_parent, ast.Module | ast.ClassDef):
                continue
            start_line: int = node.lineno  # 1-indexed
            end_line_raw: int | None = node.end_lineno
            end_line: int = end_line_raw if end_line_raw is not None else start_line
            if end_line - start_line + 1 < 3:
                continue
            hunk_start_0 = start_line - 1  # 0-indexed
            hunk_end_0 = end_line  # 0-indexed exclusive
            # full preceding file as context — encoder sees imports and structure at file top
            ctx_tokens = tokenize_lines(lines, lang, 0, hunk_start_0)
            hunk_tokens = tokenize_lines(lines, lang, hunk_start_0, hunk_end_0)
            if not hunk_tokens:
                continue
            chunks.append(
                {
                    "_repo": "fastapi-static",
                    "author_date_iso": str(len(chunks)),  # stable ordering, no git needed
                    "language": lang,
                    "context_before": [{"text": t.text} for t in ctx_tokens],
                    "context_after": [],
                    "hunk_tokens": [{"text": t.text} for t in hunk_tokens],
                    "commit_sha": head_sha,
                    "file_path": rel_path,
                    "hunk_start_line": start_line,
                    "hunk_end_line": end_line,
                    "_chunk_name": node.name,
                    "_source_lines": lines[hunk_start_0:hunk_end_0],
                }
            )
    return chunks


# ---------------------------------------------------------------------------
# Ensemble scorer
# ---------------------------------------------------------------------------


class _EnsembleForAudit:
    """n-member ensemble of JepaInfoNCEScorer trained on static chunks."""

    def __init__(
        self,
        *,
        n: int,
        beta: float,
        tau: float,
        warmup_epochs: int,
    ) -> None:
        self._n = n
        self._beta = beta
        self._tau = tau
        self._warmup_epochs = warmup_epochs
        self._members: list[JepaInfoNCEScorer] = []

    def fit(
        self,
        corpus: list[dict[str, Any]],
        *,
        preencoded: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> None:
        self._members = []

        # File-level 80/20 holdout: shuffle files, assign 80% to train.
        # Methods from the same file never appear in both train and holdout.
        all_files = sorted({c["file_path"] for c in corpus})
        rng = random.Random(42)
        rng.shuffle(all_files)
        n_train_files = int(len(all_files) * 0.8)
        train_files = set(all_files[:n_train_files])

        train_indices = [i for i, c in enumerate(corpus) if c["file_path"] in train_files]
        train_corpus = [corpus[i] for i in train_indices]

        train_pre: tuple[torch.Tensor, torch.Tensor] | None
        if preencoded is not None:
            idx_t = torch.tensor(train_indices, dtype=torch.long)
            train_pre = (preencoded[0][idx_t], preencoded[1][idx_t])
        else:
            train_pre = None

        n_holdout_files = len(all_files) - n_train_files
        n_holdout_chunks = len(corpus) - len(train_corpus)
        print(
            f"\nFile-level holdout: {n_train_files}/{len(all_files)} files → train "
            f"({len(train_corpus)} chunks), {n_holdout_files} files held out "
            f"({n_holdout_chunks} chunks never seen during training)",
            flush=True,
        )

        for i in range(self._n):
            print(f"\n  --- Training member {i + 1}/{self._n} ---", flush=True)
            m = JepaInfoNCEScorer(
                beta=self._beta,
                tau=self._tau,
                warmup_epochs=self._warmup_epochs,
                random_seed=i,
                corpus_cap=len(train_corpus),  # no cap — train on full train split
            )
            m.fit(train_corpus, preencoded=train_pre)
            self._members.append(m)

    def score_from_preencoded(
        self,
        ctx_x: torch.Tensor,
        hunk_x: torch.Tensor,
    ) -> list[float]:
        """Score using pre-encoded tensors — avoids re-encoding with wrong model."""
        if not self._members:
            raise RuntimeError("fit() must be called before score_from_preencoded()")
        all_scores: list[list[float]] = []
        for m in self._members:
            assert m._bundle is not None  # noqa: SLF001
            scores = score_from_tensors(m._bundle, ctx_x, hunk_x)  # noqa: SLF001
            all_scores.append(scores)
        n = ctx_x.shape[0]
        return [sum(run[i] for run in all_scores) / self._n for i in range(n)]


# ---------------------------------------------------------------------------
# Pre-encoding
# ---------------------------------------------------------------------------


def _encode_records(
    records: list[dict[str, Any]],
    label: str = "records",
) -> tuple[torch.Tensor, torch.Tensor]:
    """Encode records once with the pretrained encoder."""
    device = select_device()
    pretrained = PretrainedEncoder(device=device, model_name=ENCODER_MODEL)
    ctx_texts, hunk_texts = _texts_for_records(records)
    n = len(records)
    print(
        f"Pre-encoding {n} {label} ({n * 2} texts) using {ENCODER_MODEL} ...",
        flush=True,
    )
    t0 = time.perf_counter()
    with torch.no_grad():
        ctx_x = pretrained.encode_texts(ctx_texts, normalize_embeddings=NORMALIZE_EMBEDDINGS).cpu()
        hunk_x = pretrained.encode_texts(
            hunk_texts, normalize_embeddings=NORMALIZE_EMBEDDINGS
        ).cpu()
    del pretrained
    print(f"  done in {time.perf_counter() - t0:.1f}s", flush=True)
    return (ctx_x, hunk_x)


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def _percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    idx = int(pct * len(values))
    return sorted(values)[min(idx, len(values) - 1)]


def _chunk_preview(chunk: dict[str, Any], max_lines: int = 15) -> str:
    source_lines = cast(list[str], chunk["_source_lines"])
    visible = source_lines[:max_lines]
    extra = len(source_lines) - max_lines
    tail = f"\n    ... ({extra} more lines)" if extra > 0 else ""
    return "    " + "\n    ".join(visible) + tail


def _threshold_table_rows(
    fpr_arr: Any,
    tpr_arr: Any,
    thresh_arr: Any,
    target_fprs: list[float],
) -> list[tuple[float, float, float]]:
    rows: list[tuple[float, float, float]] = []
    for t_fpr in target_fprs:
        mask = fpr_arr <= t_fpr
        if not mask.any():
            rows.append((t_fpr, 0.0, float("nan")))
        else:
            idx = int(np.where(mask)[0][-1])
            cut = float(thresh_arr[min(idx, len(thresh_arr) - 1)])
            rows.append((t_fpr, float(tpr_arr[idx]), cut))
    return rows


def _histogram_rows(
    break_arr: Any,
    ctrl_arr: Any,
    edges: Any,
) -> list[tuple[float, float, int, int]]:
    rows: list[tuple[float, float, int, int]] = []
    for i in range(len(edges) - 1):
        lo, hi = edges[i], edges[i + 1]
        if i == len(edges) - 2:
            n_brk = int(((break_arr >= lo) & (break_arr <= hi)).sum())
            n_ctl = int(((ctrl_arr >= lo) & (ctrl_arr <= hi)).sum())
        else:
            n_brk = int(((break_arr >= lo) & (break_arr < hi)).sum())
            n_ctl = int(((ctrl_arr >= lo) & (ctrl_arr < hi)).sum())
        rows.append((float(lo), float(hi), n_brk, n_ctl))
    return rows


def _print_metrics_block(break_scores: list[float], ctrl_scores: list[float]) -> None:
    """Print Phase 1 distributional metrics: AUC, threshold table, overlap histogram."""
    y_true = [1] * len(break_scores) + [0] * len(ctrl_scores)
    y_score = break_scores + ctrl_scores

    auc = compute_auc(ctrl_scores, break_scores)
    fpr_arr, tpr_arr, thresh_arr = roc_curve(y_true, y_score)

    rows = _threshold_table_rows(fpr_arr, tpr_arr, thresh_arr, [0.05, 0.10, 0.15, 0.20])

    all_arr = np.array(y_score)
    edges = np.percentile(all_arr, np.linspace(0, 100, 11))
    hist_rows = _histogram_rows(np.array(break_scores), np.array(ctrl_scores), edges)

    if auc >= 0.70:
        verdict = "✅ AUC ≥ 0.70 — continue"
    elif auc < 0.65:
        verdict = "⚠️  STOP — AUC < 0.65, signal too weak"
    else:
        verdict = "⚠️  borderline (0.65–0.70), proceed with caution"

    print("\n=== Phase 1 — Distributional Metrics ===")
    print(f"AUC (ROC): {auc:.4f}  {verdict}")
    print("\nThreshold table (target FPR → actual TPR, score cutoff):")
    for fpr_t, tpr_t, cut_t in rows:
        print(f"  FPR≤{fpr_t:.2f} → TPR={tpr_t:.3f}  cutoff={cut_t:.4f}")
    print("\nScore overlap histogram (decile buckets of combined distribution):")
    print(f"  {'bucket':>23}  {'break':>6}  {'ctrl':>6}")
    for lo, hi, n_brk, n_ctl in hist_rows:
        print(f"  [{lo:.4f}, {hi:.4f})  {n_brk:>6}  {n_ctl:>6}")


def _write_phase8_report(
    break_scores: list[float],
    ctrl_scores: list[float],
    n_chunks: int,
    head_sha: str,
    specs: list[FixtureSpec],
) -> None:
    """Write Phase 1 metrics to docs/research/scoring/signal/phase8_measurement_2026-04-20.md."""
    y_true = [1] * len(break_scores) + [0] * len(ctrl_scores)
    y_score = break_scores + ctrl_scores

    auc = compute_auc(ctrl_scores, break_scores)
    fpr_arr, tpr_arr, thresh_arr = roc_curve(y_true, y_score)
    # PR curve computed for completeness; threshold table uses ROC only
    _pr_prec, _pr_rec, _ = precision_recall_curve(y_true, y_score)

    rows = _threshold_table_rows(fpr_arr, tpr_arr, thresh_arr, [0.05, 0.10, 0.15, 0.20])

    all_arr = np.array(y_score)
    edges = np.percentile(all_arr, np.linspace(0, 100, 11))
    hist_rows = _histogram_rows(np.array(break_scores), np.array(ctrl_scores), edges)

    n_break = sum(s.is_break for s in specs)
    n_ctrl = sum(not s.is_break for s in specs)

    if auc >= 0.70:
        verdict = "✅ AUC ≥ 0.70 — continue to Phase 2"
    elif auc < 0.65:
        verdict = "⚠️ STOP — AUC < 0.65, signal too weak; consider LLM hybrid"
    else:
        verdict = "⚠️ borderline AUC (0.65–0.70), proceed with caution"

    brk_pct = compute_percentiles(break_scores)
    ctl_pct = compute_percentiles(ctrl_scores)

    md_lines = [
        "# Phase 8 Measurement — 2026-04-20",
        "",
        "## Setup",
        "",
        f"- Encoder: `{ENCODER_MODEL}`",
        f"- Ensemble: `EnsembleInfoNCE(n={ENSEMBLE_N}, beta={WINNER_BETA}, tau={WINNER_TAU})`",
        f"- FastAPI HEAD: `{head_sha}`",
        f"- Static chunks extracted: {n_chunks}",
        f"- Fixtures: {len(specs)} total ({n_break} breaks, {n_ctrl} controls)",
        "- Command: `uv run python -m argot.research.static_chunk_audit_test`",
        "",
        "## AUC",
        "",
        f"**AUC (ROC): {auc:.4f}**  {verdict}",
        "",
        "## Threshold Table",
        "",
        "| Target FPR | Actual TPR | Score Cutoff |",
        "|:----------:|:----------:|:------------:|",
    ]
    for fpr_t, tpr_t, cut_t in rows:
        md_lines.append(f"| {fpr_t:.2f} | {tpr_t:.3f} | {cut_t:.4f} |")

    md_lines += [
        "",
        "## Score Overlap Histogram (decile buckets)",
        "",
        "| Bucket | Break | Control |",
        "|--------|------:|--------:|",
    ]
    for lo, hi, n_brk, n_ctl in hist_rows:
        md_lines.append(f"| [{lo:.4f}, {hi:.4f}) | {n_brk} | {n_ctl} |")

    md_lines += [
        "",
        "## Score Percentiles",
        "",
        "**Break scores:**",
        f"min={brk_pct['min']:.4f}  p25={brk_pct['p25']:.4f}  median={brk_pct['median']:.4f}"
        f"  p75={brk_pct['p75']:.4f}  p95={brk_pct['p95']:.4f}  max={brk_pct['max']:.4f}",
        "",
        "**Control scores:**",
        f"min={ctl_pct['min']:.4f}  p25={ctl_pct['p25']:.4f}  median={ctl_pct['median']:.4f}"
        f"  p75={ctl_pct['p75']:.4f}  p95={ctl_pct['p95']:.4f}  max={ctl_pct['max']:.4f}",
    ]

    # engine/argot/research/ → ../../.. → engine/ → ../ → repo root
    _repo_root = Path(__file__).parent.parent.parent.parent
    report_dir = _repo_root / "docs" / "research" / "scoring" / "signal"
    report_dir.mkdir(parents=True, exist_ok=True)
    report_path = report_dir / "phase8_measurement_2026-04-20.md"
    report_path.write_text("\n".join(md_lines) + "\n", encoding="utf-8")
    print(f"\nPhase 1 report written to {report_path}", flush=True)


def _print_report(
    chunks: list[dict[str, Any]],
    chunk_scores: list[float],
    break_scores: list[float],
    ctrl_scores: list[float],
) -> None:
    n = len(chunk_scores)
    corpus_mean = statistics.mean(chunk_scores) if chunk_scores else 0.0
    corpus_p95 = _percentile(chunk_scores, 0.95)
    break_mean = statistics.mean(break_scores) if break_scores else 0.0
    ctrl_mean = statistics.mean(ctrl_scores) if ctrl_scores else 0.0

    print("\n" + "=" * 70)
    print("=== commit-hunk baseline ===")
    print(f"corpus mean={BASELINE_CORPUS_MEAN:.4f} p95={BASELINE_P95:.4f} n={BASELINE_N}")
    print(f"fixture break mean={BASELINE_BREAK_MEAN:.4f}  control mean={BASELINE_CTRL_MEAN:.4f}")

    print(f"\n=== static-chunk audit (encoder={ENCODER_MODEL}, file-level holdout) ===")
    print(f"static chunks extracted: {n}")
    print(f"corpus (static) mean={corpus_mean:.4f} p95={corpus_p95:.4f} n={n}")
    print(f"fixture break mean={break_mean:.4f}  control mean={ctrl_mean:.4f}")
    delta = break_mean - ctrl_mean
    prev_delta = 0.1817
    print(f"delta (break-ctrl)={delta:.4f}  (prev best={prev_delta:.4f})")

    indexed = sorted(enumerate(chunk_scores), key=lambda x: x[1], reverse=True)

    print("\n=== TOP 20 static chunks by score ===")
    for rank, (idx, score) in enumerate(indexed[:20], 1):
        chunk = chunks[idx]
        print(
            f"#{rank}  score={score:.4f}  {chunk['file_path']}"
            f"  L{chunk['hunk_start_line']}-{chunk['hunk_end_line']}"
            f"  {chunk['_chunk_name']}"
        )
        print(_chunk_preview(chunk))
        print()

    print("=== BOTTOM 5 static chunks by score ===")
    for rank, (idx, score) in enumerate(reversed(indexed[-5:]), 1):
        chunk = chunks[idx]
        print(
            f"#{rank}  score={score:.4f}  {chunk['file_path']}"
            f"  L{chunk['hunk_start_line']}-{chunk['hunk_end_line']}"
            f"  {chunk['_chunk_name']}"
        )
        print(_chunk_preview(chunk))
        print()

    top20_paths = [chunks[idx]["file_path"] for idx, _ in indexed[:20]]
    n_framework = sum(1 for p in top20_paths if p.startswith("fastapi/"))
    n_tests = sum(1 for p in top20_paths if "test" in p)
    n_docs = sum(1 for p in top20_paths if "docs" in p or "scripts" in p)

    signal_ratio = corpus_p95 / break_mean if break_mean > 0 else 0.0
    baseline_ratio = BASELINE_P95 / BASELINE_BREAK_MEAN if BASELINE_BREAK_MEAN > 0 else 0.0

    print("=== Qualitative comparison ===")
    print(
        f"Top-20 breakdown: {n_framework}/20 from fastapi/ (framework core), "
        f"{n_tests}/20 from test files, {n_docs}/20 from docs/scripts. "
        f"Signal level: static p95/break ratio={signal_ratio:.3f} vs "
        f"baseline p95/break ratio={baseline_ratio:.3f}. "
        f"Delta vs prev best (0.1817): {delta - prev_delta:+.4f}."
    )


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    t_start = time.perf_counter()

    # 1. Clone/reuse FastAPI
    head_sha = _clone_or_reuse()

    # 2. Extract static chunks from HEAD — this IS the corpus, no git history needed
    print(f"\nExtracting AST chunks from {FASTAPI_CLONE_DIR} ...", flush=True)
    t0 = time.perf_counter()
    chunks = _extract_chunks(FASTAPI_CLONE_DIR, head_sha)
    print(f"Extracted {len(chunks)} chunks in {time.perf_counter() - t0:.1f}s", flush=True)

    if not chunks:
        print("ERROR: No chunks extracted. Check clone path.", file=sys.stderr)
        sys.exit(1)

    # 3. Pre-encode all static chunks once with UnixCoder (reused across all ensemble members)
    preencoded = _encode_records(chunks, label="static chunks")

    # 4. Train ensemble on file-level 80% train split
    print(
        f"\nTraining {ENSEMBLE_N}-member ensemble on static chunks "
        f"(beta={WINNER_BETA} tau={WINNER_TAU} warmup={WINNER_WARMUP} "
        f"encoder={ENCODER_MODEL}) ...",
        flush=True,
    )
    ensemble = _EnsembleForAudit(
        n=ENSEMBLE_N,
        beta=WINNER_BETA,
        tau=WINNER_TAU,
        warmup_epochs=WINNER_WARMUP,
    )
    ensemble.fit(chunks, preencoded=preencoded)

    # 5. Pre-encode fixtures with same UnixCoder encoder and score (verifies detection signal)
    specs: list[FixtureSpec] = load_manifest(ENTRY_DIR)
    fixture_records = [fixture_to_record(ENTRY_DIR, s) for s in specs]
    fixture_preencoded = _encode_records(fixture_records, label="fixture records")
    print(f"\nScoring {len(fixture_records)} fixture records ...", flush=True)
    fixture_scores = ensemble.score_from_preencoded(*fixture_preencoded)
    break_scores = [fixture_scores[i] for i, s in enumerate(specs) if s.is_break]
    ctrl_scores = [fixture_scores[i] for i, s in enumerate(specs) if not s.is_break]

    # 6. Score all static chunks via preencoded tensors (avoids re-encoding with wrong model)
    print(f"Scoring {len(chunks)} static chunks ...", flush=True)
    chunk_scores = ensemble.score_from_preencoded(*preencoded)

    elapsed = time.perf_counter() - t_start
    print(f"\nTotal elapsed: {elapsed:.0f}s", flush=True)

    _print_report(chunks, chunk_scores, break_scores, ctrl_scores)
    _print_metrics_block(break_scores, ctrl_scores)
    _write_phase8_report(break_scores, ctrl_scores, len(chunks), head_sha, specs)


if __name__ == "__main__":
    main()
