"""Era 12 Phase 8f — MAX-z ensemble over (Phase 6.4, Phase 8d) anomaly scores.

Combines two complementary unsupervised signals:
  - 6.4 distance: 1 - cos(hunk_n, cluster_centroid_n) per row
  - 8d distance: 1 - cos(hunk_n, context_n) per row (hunk-vs-its-own-context)

Each row scored, z-normalised against its own corpus's CONTROL distribution
(unsupervised — no break labels touch the normalisation), then per row we
take MAX(z_6.4, z_8d). Per-corpus threshold = (1 - FP_target/100)-quantile
of CONTROL max-z. Higher max-z = more anomalous.

Hypothesis: 6.4 catches ``runtime_fetch_2`` (semantic divergence from cluster
centroid). 8d ranks ``error_flip_2/3`` higher than 6.4 does (alignment of
hunk to file context). Their union should catch ≥2/5 residuals if their
errors are uncorrelated, at FP cost ≤ ~2× target.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

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

MIN_CLUSTER = 5  # match Phase 6.4


def l2(x: np.ndarray) -> np.ndarray:
    return x / (np.linalg.norm(x, axis=-1, keepdims=True) + 1e-12)


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
    cluster = np.array(
        [r["features"].get("cluster_id") if r["features"].get("cluster_id") is not None
         else -1 for r in rows], dtype=int)
    file_path = np.array([r.get("file_path") or "" for r in rows])

    hunk = np.array([r["hunk_embedding"] for r in rows], dtype=np.float32)
    ctx = np.array([r["context_embedding"] for r in rows], dtype=np.float32)
    print(f"Loaded {n} rows", file=sys.stderr)

    hunk_n = l2(hunk)
    ctx_n = l2(ctx)

    # --- Phase 6.4 distance: 1 - cos(hunk, cluster_centroid) ----------------
    centroids: dict[tuple[str, int], np.ndarray] = {}
    for c in CORPORA:
        cmask = corpus == c
        for cid in sorted(set(cluster[cmask].tolist())):
            if cid < 0:
                continue
            mask = cmask & (~is_break) & (cluster == cid)
            if mask.sum() < MIN_CLUSTER:
                continue
            cent = hunk_n[mask].mean(axis=0)
            cent = cent / (np.linalg.norm(cent) + 1e-12)
            centroids[(c, int(cid))] = cent.astype(np.float32)
    print(f"Built {len(centroids)} cluster centroids", file=sys.stderr)

    dist_64 = np.full(n, np.nan, dtype=np.float32)
    for i in range(n):
        key = (corpus[i], int(cluster[i]))
        if key in centroids:
            dist_64[i] = 1.0 - float(hunk_n[i] @ centroids[key])

    # --- Phase 8d distance: 1 - cos(hunk, context) --------------------------
    dist_8d = 1.0 - (hunk_n * ctx_n).sum(axis=-1)

    # --- z-score each metric per-corpus against CONTROLS only ---------------
    z_64 = np.full(n, np.nan, dtype=np.float32)
    z_8d = np.full(n, np.nan, dtype=np.float32)
    for c in CORPORA:
        cmask = corpus == c
        ctrl = cmask & (~is_break)
        # 6.4: only rows that have a centroid contribute / get scored
        ctrl_64 = ctrl & ~np.isnan(dist_64)
        if ctrl_64.sum() > 1:
            mu = float(dist_64[ctrl_64].mean())
            sd = float(dist_64[ctrl_64].std()) or 1e-6
            z_64[cmask & ~np.isnan(dist_64)] = (
                dist_64[cmask & ~np.isnan(dist_64)] - mu
            ) / sd
        ctrl_8d = ctrl
        if ctrl_8d.sum() > 1:
            mu2 = float(dist_8d[ctrl_8d].mean())
            sd2 = float(dist_8d[ctrl_8d].std()) or 1e-6
            z_8d[cmask] = (dist_8d[cmask] - mu2) / sd2

    # --- ensemble: max-z (NaN-safe) -----------------------------------------
    # If a row has no 6.4 score (no centroid), fall back to 8d only.
    max_z = np.where(
        np.isnan(z_64), z_8d,
        np.maximum(z_64, z_8d)
    )

    # --- per-corpus threshold (1 - FP/100)-quantile of CONTROL max_z --------
    per_corpus = {}
    for c in CORPORA:
        cmask = corpus == c
        ctrl = cmask & (~is_break)
        ctrl_z = max_z[ctrl]
        ctrl_z = ctrl_z[~np.isnan(ctrl_z)]
        if len(ctrl_z) == 0:
            continue
        q = 1 - FP_TARGET[c] / 100
        thr = float(np.quantile(ctrl_z, q))
        n_ctrl = int(len(ctrl_z))
        cf = int((ctrl_z > thr).sum())
        actual_fp = 100 * cf / n_ctrl
        breaks = cmask & is_break
        bz = max_z[breaks]
        nb = int(breaks.sum())
        bc = int(np.nansum(bz > thr))
        per_corpus[c] = {
            "fp_target_pct": FP_TARGET[c],
            "threshold_max_z": thr,
            "n_controls": n_ctrl,
            "controls_flagged": cf,
            "actual_fp_pct": actual_fp,
            "fp_regression_pp": actual_fp - FP_TARGET[c],
            "breaks_total": nb,
            "breaks_caught": bc,
            "stage4_recall_pct": 100 * bc / nb if nb else 0.0,
        }

    # --- residual catch test (faker-js) -------------------------------------
    fjs_thr = per_corpus["faker-js"]["threshold_max_z"]
    fjs_ctrl = max_z[(corpus == "faker-js") & (~is_break)]
    fjs_ctrl = np.sort(fjs_ctrl[~np.isnan(fjs_ctrl)])
    n_fjs = len(fjs_ctrl)

    residuals = {}
    for r in sorted(RESIDUALS):
        idx = np.where(fid == r)[0]
        if len(idx) == 0:
            residuals[r] = {"missing": True}
            continue
        i = int(idx[0])
        residuals[r] = {
            "z_64": float(z_64[i]) if not np.isnan(z_64[i]) else None,
            "z_8d": float(z_8d[i]) if not np.isnan(z_8d[i]) else None,
            "max_z": float(max_z[i]) if not np.isnan(max_z[i]) else None,
            "threshold": fjs_thr,
            "crosses": bool(max_z[i] > fjs_thr) if not np.isnan(max_z[i]) else False,
            "rank_top_pct": (
                100 * int((fjs_ctrl > max_z[i]).sum()) / n_fjs
                if not np.isnan(max_z[i]) and n_fjs else None
            ),
            "file_path": str(file_path[i]),
        }

    n_caught = sum(1 for v in residuals.values() if v.get("crosses"))
    no_reg = all(
        v["actual_fp_pct"] <= FP_TARGET[c] + 0.5
        for c, v in per_corpus.items()
    )

    # --- pooled AUC for sanity ----------------------------------------------
    from sklearn.metrics import roc_auc_score  # type: ignore[import-not-found]

    pooled = {}
    for label, arr in [("max_z", max_z), ("z_64", z_64), ("z_8d", z_8d)]:
        m = ~np.isnan(arr)
        if m.sum() == 0 or len(set(is_break[m].tolist())) < 2:
            pooled[label] = None
            continue
        pooled[label] = float(roc_auc_score(is_break[m].astype(int), arr[m]))

    out = {
        "method": "max(z(1 - cos(hunk, centroid)), z(1 - cos(hunk, context))) per corpus",
        "n_rows": int(n),
        "n_centroids": len(centroids),
        "pooled_aucs": pooled,
        "per_corpus": per_corpus,
        "residuals": residuals,
        "n_residual_catch": n_caught,
        "ship_gate_pass": n_caught >= 2 and no_reg,
        "no_regression_gate_pass": no_reg,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
