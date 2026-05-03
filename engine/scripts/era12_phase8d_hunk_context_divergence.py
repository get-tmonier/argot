"""Era 12 Phase 8d — hunk-vs-context cosine divergence.

For each row: score = 1 - cosine(hunk_embedding, context_embedding).

Hypothesis: in normal control hunks, the hunk and its surrounding 512-token
context window share semantic content (same module, same call patterns), so
cosine ≈ high. In breaks like runtime_fetch_2, the hunk introduces semantics
(fetch / network) that the surrounding host file does not share, so the two
embeddings diverge → cosine lower → score higher.

Single scalar per hunk. Zero new forward passes — embeddings already exist.
Per-corpus threshold calibrated at era-11 baseline FP, same shape as Phase 6.4.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era12-features"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]

FP_TARGET = {
    "fastapi": 0.6, "rich": 1.2, "faker": 2.0,
    "hono": 0.5, "ink": 0.5, "faker-js": 0.9,
}

RESIDUALS = {
    "faker_js_error_flip_2", "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1", "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}


def l2(x: np.ndarray) -> np.ndarray:
    n = np.linalg.norm(x, axis=-1, keepdims=True) + 1e-12
    return x / n


def main() -> None:
    rows = []
    for c in CORPORA:
        with open(FEATURE_DIR / f"{c}.jsonl") as f:
            for line in f:
                rows.append(json.loads(line))

    n = len(rows)
    is_break = np.array([bool(r["is_break"]) for r in rows])
    corpus = np.array([r["corpus"] for r in rows])
    fid = np.array([r.get("fixture_id") or "" for r in rows])
    file_path = np.array([r.get("file_path") or "" for r in rows])

    hunk = np.array([r["hunk_embedding"] for r in rows], dtype=np.float32)
    ctx = np.array([r["context_embedding"] for r in rows], dtype=np.float32)
    print(f"Loaded {n} rows; {is_break.sum()} breaks", file=sys.stderr)

    hunk_n = l2(hunk)
    ctx_n = l2(ctx)

    # Score: 1 - cosine(hunk, context). Higher = more divergent = more anomalous.
    cos = (hunk_n * ctx_n).sum(axis=-1)
    score = 1.0 - cos
    print(
        f"score range: min={score.min():.4f} mean={score.mean():.4f} max={score.max():.4f}",
        file=sys.stderr,
    )

    # Pooled AUC (breaks vs controls) — sanity check.
    from sklearn.metrics import roc_auc_score  # type: ignore[import-not-found]

    pooled_auc = float(roc_auc_score(is_break.astype(int), score))
    print(f"pooled AUC (breaks vs controls): {pooled_auc:.4f}", file=sys.stderr)

    # Per-corpus AUC.
    per_corpus_auc = {}
    for c in CORPORA:
        m = corpus == c
        if m.sum() == 0:
            continue
        y = is_break[m].astype(int)
        s = score[m]
        if y.sum() in (0, len(y)):
            per_corpus_auc[c] = None
            continue
        per_corpus_auc[c] = float(roc_auc_score(y, s))

    # Per-corpus thresholds.
    thresholds = {}
    for c in CORPORA:
        ctrl_mask = (corpus == c) & (~is_break)
        ctrl_scores = score[ctrl_mask]
        n_ctrl = len(ctrl_scores)
        if n_ctrl == 0:
            thresholds[c] = None
            continue
        q = 1 - FP_TARGET[c] / 100
        thr = float(np.quantile(ctrl_scores, q))
        flagged = int((ctrl_scores > thr).sum())
        thresholds[c] = {
            "fp_target_pct": FP_TARGET[c],
            "threshold": thr,
            "n_controls": n_ctrl,
            "controls_flagged": flagged,
            "actual_fp_pct": 100 * flagged / n_ctrl,
        }

    # Per-corpus recall + FP.
    per_corpus = {}
    for c in CORPORA:
        thr_info = thresholds[c]
        if thr_info is None:
            continue
        thr = thr_info["threshold"]
        cm = corpus == c
        breaks_m = cm & is_break
        ctrl_m = cm & (~is_break)
        n_breaks = int(breaks_m.sum())
        n_ctrl = int(ctrl_m.sum())
        breaks_caught = int((score[breaks_m] > thr).sum())
        ctrls_flagged = int((score[ctrl_m] > thr).sum())
        per_corpus[c] = {
            "threshold": thr,
            "fp_target_pct": FP_TARGET[c],
            "breaks_total": n_breaks,
            "breaks_caught": breaks_caught,
            "stage4_recall_pct": 100 * breaks_caught / n_breaks if n_breaks else 0.0,
            "actual_fp_pct": 100 * ctrls_flagged / n_ctrl if n_ctrl else None,
            "fp_regression_pp": (100 * ctrls_flagged / n_ctrl - FP_TARGET[c])
            if n_ctrl else None,
        }

    # Residual catch (faker-js).
    fjs_thr = thresholds["faker-js"]["threshold"]
    fjs_ctrl_mask = (corpus == "faker-js") & (~is_break)
    fjs_ctrl_scores = np.sort(score[fjs_ctrl_mask])
    n_fjs = len(fjs_ctrl_scores)

    residuals_out = {}
    for r in sorted(RESIDUALS):
        idx = np.where(fid == r)[0]
        if len(idx) == 0:
            residuals_out[r] = {"error": "not found"}
            continue
        i = int(idx[0])
        s = float(score[i])
        n_more = int((fjs_ctrl_scores > s).sum())
        residuals_out[r] = {
            "score": s,
            "threshold": fjs_thr,
            "crosses": s > fjs_thr,
            "rank_top_pct": 100 * n_more / n_fjs,
            "percentile_among_controls": float((fjs_ctrl_scores <= s).sum()) / n_fjs,
            "file_path": str(file_path[i]),
        }

    n_caught = sum(1 for v in residuals_out.values() if isinstance(v, dict) and v.get("crosses"))
    no_reg = all(
        (v.get("actual_fp_pct") is None) or (v["actual_fp_pct"] <= FP_TARGET[c] + 0.5)
        for c, v in per_corpus.items()
    )

    out = {
        "method": "1 - cosine(hunk_embedding, context_embedding)",
        "pooled_auc_breaks_vs_controls": pooled_auc,
        "per_corpus_auc": per_corpus_auc,
        "thresholds": thresholds,
        "per_corpus_recall_fp": per_corpus,
        "residuals": residuals_out,
        "n_residual_catch": n_caught,
        "ship_gate_pass": n_caught >= 2,
        "no_regression_gate_pass": no_reg,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
