"""Era 14 Phase 6.2 — UnixCoder embedding KILL-SWITCH probe.

Tests whether UnixCoder semantic embeddings carry signal to predict `is_break`,
specifically signal that's distinguishable on the 5 residual faker-js fixtures.

Outputs JSON results to stdout for the memo writer to consume.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import joblib
import numpy as np
from sklearn.decomposition import PCA
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold
from sklearn.preprocessing import StandardScaler

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era14-features"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
RESIDUALS = {
    "faker_js_error_flip_2",
    "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1",
    "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}


def load_all() -> dict:
    rows = []
    for corpus in CORPORA:
        with open(FEATURE_DIR / f"{corpus}.jsonl") as f:
            for line in f:
                d = json.loads(line)
                rows.append(d)
    n = len(rows)
    is_break = np.array([bool(r["is_break"]) for r in rows])
    hunk = np.array([r["hunk_embedding"] for r in rows], dtype=np.float32)
    ctx = np.array([r["context_embedding"] for r in rows], dtype=np.float32)
    corpus = np.array([r["corpus"] for r in rows])
    fixture_id = np.array([r.get("fixture_id") or "" for r in rows])
    cluster = np.array(
        [r["features"].get("cluster_id") if r["features"].get("cluster_id") is not None else -1
         for r in rows], dtype=int
    )
    file_path = np.array([r.get("file_path") or "" for r in rows])
    return {
        "rows": rows, "n": n, "is_break": is_break,
        "hunk": hunk, "ctx": ctx, "corpus": corpus,
        "fixture_id": fixture_id, "cluster": cluster, "file_path": file_path,
    }


def per_dim_aucs(emb: np.ndarray, y: np.ndarray) -> np.ndarray:
    """Returns per-dimension AUC, auto-flipped (max(auc, 1-auc))."""
    aucs = np.zeros(emb.shape[1])
    for j in range(emb.shape[1]):
        a = roc_auc_score(y, emb[:, j])
        aucs[j] = max(a, 1 - a)
    return aucs


def auc_histogram(aucs: np.ndarray) -> dict:
    bins = [(0.0, 0.55), (0.55, 0.65), (0.65, 0.75), (0.75, 0.85), (0.85, 1.01)]
    out = {}
    for lo, hi in bins:
        cnt = int(((aucs >= lo) & (aucs < hi)).sum())
        out[f"[{lo:.2f},{hi:.2f})"] = cnt
    return out


def task1(data: dict) -> dict:
    print("=== TASK 1: per-dim AUC ===", file=sys.stderr)
    y = data["is_break"]
    hunk_aucs = per_dim_aucs(data["hunk"], y)
    ctx_aucs = per_dim_aucs(data["ctx"], y)

    top_h = np.argsort(-hunk_aucs)[:20]
    top_c = np.argsort(-ctx_aucs)[:20]

    all_aucs = np.concatenate([hunk_aucs, ctx_aucs])
    hist = auc_histogram(all_aucs)

    n_above_065 = int((all_aucs > 0.65).sum())
    n_h_above_065 = int((hunk_aucs > 0.65).sum())
    n_c_above_065 = int((ctx_aucs > 0.65).sum())

    return {
        "hunk_top20": [(int(i), float(hunk_aucs[i])) for i in top_h],
        "ctx_top20": [(int(i), float(ctx_aucs[i])) for i in top_c],
        "histogram_all_1536": hist,
        "n_dims_above_0.65_pooled": n_above_065,
        "n_hunk_dims_above_0.65": n_h_above_065,
        "n_ctx_dims_above_0.65": n_c_above_065,
        "best_hunk_dim_auc": float(hunk_aucs[top_h[0]]),
        "best_ctx_dim_auc": float(ctx_aucs[top_c[0]]),
        "_hunk_aucs": hunk_aucs,  # for diagnostic use
        "_ctx_aucs": ctx_aucs,
    }


def task2(data: dict) -> dict:
    print("=== TASK 2: PCA + LR ===", file=sys.stderr)
    y = data["is_break"]
    # Stack hunk + ctx (1536 dims), per-corpus normalize before PCA
    full = np.concatenate([data["hunk"], data["ctx"]], axis=1)

    # Per-corpus standardization (z-score within corpus)
    full_norm = full.copy()
    for c in CORPORA:
        mask = data["corpus"] == c
        if mask.sum() > 1:
            mu = full_norm[mask].mean(axis=0)
            sd = full_norm[mask].std(axis=0) + 1e-8
            full_norm[mask] = (full_norm[mask] - mu) / sd

    pca = PCA(n_components=100, random_state=0)
    Z = pca.fit_transform(full_norm)
    explained = float(pca.explained_variance_ratio_.sum())

    pca_aucs = per_dim_aucs(Z, y)
    top_pca = np.argsort(-pca_aucs)[:20]

    # 5-fold CV LR on PCA-100
    cv = StratifiedKFold(n_splits=5, shuffle=True, random_state=0)
    cv_probs = np.zeros(len(y))
    for tr, te in cv.split(Z, y):
        sc = StandardScaler().fit(Z[tr])
        Ztr = sc.transform(Z[tr])
        Zte = sc.transform(Z[te])
        lr = LogisticRegression(max_iter=2000, C=1.0, random_state=0).fit(Ztr, y[tr])
        cv_probs[te] = lr.predict_proba(Zte)[:, 1]
    lr_auc = float(roc_auc_score(y, cv_probs))

    # Save the fitted pca model + per-corpus stats for reuse
    save_payload = {
        "pca": pca,
        "explained_variance_ratio": pca.explained_variance_ratio_,
        "n_components": 100,
        "feature_layout": "concat(hunk_embedding, context_embedding) -> 1536",
        "normalization": "per-corpus z-score before PCA",
        "corpora": CORPORA,
    }
    out_path = FEATURE_DIR / "pca100_phase6.2.joblib"
    joblib.dump(save_payload, out_path)

    return {
        "pca100_explained_variance": explained,
        "pca_top20_components": [(int(i), float(pca_aucs[i])) for i in top_pca],
        "lr_pca100_cv5_auc": lr_auc,
        "best_pca_component_auc": float(pca_aucs[top_pca[0]]),
        "top3_pca_component_aucs": [float(pca_aucs[top_pca[i]]) for i in range(3)],
        "saved_to": str(out_path),
    }


def task3(data: dict) -> dict:
    print("=== TASK 3: residual fixture probe ===", file=sys.stderr)
    y = data["is_break"]
    hunk = data["hunk"]
    fixture_id = data["fixture_id"]
    corpus = data["corpus"]
    cluster = data["cluster"]
    file_path = data["file_path"]

    # cosine similarity helper
    def normalize(x: np.ndarray) -> np.ndarray:
        norms = np.linalg.norm(x, axis=1, keepdims=True) + 1e-12
        return x / norms

    hunk_n = normalize(hunk)

    # Identify residuals
    residual_mask = np.isin(fixture_id, list(RESIDUALS))
    residual_idx = np.where(residual_mask)[0]
    print(f"residuals found: {len(residual_idx)}", file=sys.stderr)

    # faker-js controls (corpus=faker-js AND is_break=False)
    fjs_mask = corpus == "faker-js"
    fjs_ctrl_mask = fjs_mask & (~y)
    fjs_ctrl_idx = np.where(fjs_ctrl_mask)[0]
    all_ctrl_idx = np.where(~y)[0]
    print(f"faker-js controls: {len(fjs_ctrl_idx)}, all controls: {len(all_ctrl_idx)}", file=sys.stderr)

    # all break fixtures (all corpora) for "neighbor type" classification
    all_break_idx = np.where(y)[0]

    # Per residual: 30 nearest neighbors (a) within faker-js controls, (b) all controls
    per_residual_neighbors = {}
    # Also pool with breaks INCLUDED — i.e. find 30 NN among (faker-js controls + faker-js breaks excluding self)
    # and (all controls + all breaks excluding self). The task asks: how many neighbors are
    # is_break=True vs is_break=False. So pool both groups.

    fjs_pool_idx = np.where(fjs_mask)[0]  # all faker-js rows
    all_pool_idx = np.arange(len(y))  # all rows

    for ridx in residual_idx:
        fid = fixture_id[ridx]
        v = hunk_n[ridx:ridx + 1]  # 1×768

        def nn_in_pool(pool_idx: np.ndarray, k: int = 30):
            # exclude self
            pool = pool_idx[pool_idx != ridx]
            sims = (hunk_n[pool] @ v.T).flatten()
            order = np.argsort(-sims)[:k]
            chosen = pool[order]
            return chosen, sims[order]

        # Pool A = faker-js (controls + breaks)
        nn_a, sims_a = nn_in_pool(fjs_pool_idx, 30)
        n_break_a = int(y[nn_a].sum())
        n_ctrl_a = 30 - n_break_a

        # Pool B = all rows (controls + breaks, all corpora)
        nn_b, sims_b = nn_in_pool(all_pool_idx, 30)
        n_break_b = int(y[nn_b].sum())
        n_ctrl_b = 30 - n_break_b

        # Among the breaks in pool B, are they other catalog fixtures?
        break_neighbors_b = [(fixture_id[i], corpus[i]) for i in nn_b if y[i]]

        per_residual_neighbors[fid] = {
            "pool_fjs_breaks_in_top30": n_break_a,
            "pool_fjs_ctrls_in_top30": n_ctrl_a,
            "pool_all_breaks_in_top30": n_break_b,
            "pool_all_ctrls_in_top30": n_ctrl_b,
            "break_neighbors_pool_all": break_neighbors_b[:10],  # first 10 for memo
            "max_sim_to_any_break_pool_all": float(sims_b[y[nn_b]].max()) if n_break_b else None,
            "mean_sim_top30_pool_all": float(sims_b.mean()),
        }

    # Cluster centroid distance feature (cosine distance from each row to its file's cluster centroid)
    # cluster centroid = mean embedding of cluster's attested controls (within same file? or same corpus+cluster?)
    # The features.cluster_id is per-corpus. The "file's cluster" => use (corpus, cluster_id) as the key
    # and centroid = mean of attested controls in that cluster.
    # Attested controls = is_break=False AND in that corpus+cluster.

    # Build centroids per (corpus, cluster_id)
    centroids = {}
    for c in CORPORA:
        clus_in_c = set(cluster[(corpus == c) & (cluster >= 0)].tolist())
        for cid in clus_in_c:
            mask = (corpus == c) & (cluster == cid) & (~y)
            if mask.sum() > 0:
                centroids[(c, int(cid))] = hunk_n[mask].mean(axis=0)

    # Per row: compute cosine distance from hunk_n[i] to its (corpus, cluster) centroid (if exists)
    dist = np.full(len(y), np.nan)
    for i in range(len(y)):
        key = (corpus[i], int(cluster[i]))
        if key in centroids:
            cen = centroids[key]
            cen_n = cen / (np.linalg.norm(cen) + 1e-12)
            sim = float(hunk_n[i] @ cen_n)
            dist[i] = 1.0 - sim

    # Pooled AUC: distance from cluster centroid as is_break predictor
    valid_mask = ~np.isnan(dist)
    pooled_dist_auc = float(roc_auc_score(y[valid_mask], dist[valid_mask]))

    # faker-js-only AUC for sanity
    fjs_valid = valid_mask & fjs_mask
    fjs_dist_auc = float(roc_auc_score(y[fjs_valid], dist[fjs_valid])) if y[fjs_valid].sum() > 0 else None

    # For each residual: rank vs faker-js controls
    fjs_ctrl_dists = dist[fjs_ctrl_mask & valid_mask]
    fjs_ctrl_dists_sorted = np.sort(fjs_ctrl_dists)
    n_ctrls = len(fjs_ctrl_dists_sorted)

    residual_centroid_distances = {}
    for ridx in residual_idx:
        fid = fixture_id[ridx]
        d = float(dist[ridx])
        # percentile = fraction of faker-js controls with dist <= d
        pct = float((fjs_ctrl_dists_sorted <= d).sum()) / n_ctrls if n_ctrls > 0 else None
        residual_centroid_distances[fid] = {
            "distance_to_cluster_centroid": d,
            "percentile_among_fjs_controls": pct,  # 1.0 = max, > 0.9 == top 10% anomalous
            "above_90th_pct_of_fjs_controls": pct is not None and pct > 0.90,
        }

    # 90th percentile of fjs control distances
    p90 = float(np.percentile(fjs_ctrl_dists, 90))
    p95 = float(np.percentile(fjs_ctrl_dists, 95))

    n_residuals_above_p90 = sum(
        1 for v in residual_centroid_distances.values() if v["above_90th_pct_of_fjs_controls"]
    )

    return {
        "per_residual_neighbors": per_residual_neighbors,
        "centroid_distance_pooled_auc": pooled_dist_auc,
        "centroid_distance_fjs_auc": fjs_dist_auc,
        "fjs_control_p90_distance": p90,
        "fjs_control_p95_distance": p95,
        "fjs_control_count": n_ctrls,
        "residual_centroid_distances": residual_centroid_distances,
        "n_residuals_above_90th_pct": n_residuals_above_p90,
    }


def task5(data: dict, t1: dict, t2: dict) -> dict:
    print("=== TASK 5: vs engineered ===", file=sys.stderr)
    return {
        "best_engineered_feature_auc": 0.886,  # from Phase 5 (n_unattested_callees)
        "best_embedding_dim_auc": max(t1["best_hunk_dim_auc"], t1["best_ctx_dim_auc"]),
        "lr_pca100_embeddings_auc": t2["lr_pca100_cv5_auc"],
        "lr_engineered_phase3_6b_auc": 0.87,  # roughly conservative model from Phase 3.6b
    }


def main():
    data = load_all()
    print(f"Loaded {data['n']} rows; {data['is_break'].sum()} breaks", file=sys.stderr)

    t1 = task1(data)
    t2 = task2(data)
    t3 = task3(data)
    t5 = task5(data, t1, t2)

    # Verdict logic (pre-registered)
    pooled_pass = t1["n_dims_above_0.65_pooled"] >= 5
    pca_pass = t2["lr_pca100_cv5_auc"] > 0.85 or any(a > 0.70 for a in t2["top3_pca_component_aucs"])
    residual_pass = t3["n_residuals_above_90th_pct"] >= 1

    if not pooled_pass and not pca_pass:
        verdict = "NO SIGNAL"
    elif not pooled_pass or not pca_pass:
        # Either gate failed
        if t3["n_residuals_above_90th_pct"] == 0:
            verdict = "NO SIGNAL"
        else:
            verdict = "WEAK SIGNAL"
    else:
        # Both pooled gates pass
        if t3["n_residuals_above_90th_pct"] >= 2:
            verdict = "STRONG SIGNAL"
        elif t3["n_residuals_above_90th_pct"] == 1:
            verdict = "WEAK SIGNAL"
        else:
            verdict = "NO SIGNAL"

    # Strip nondeterministic numpy arrays for JSON output
    t1_out = {k: v for k, v in t1.items() if not k.startswith("_")}
    out = {
        "task1": t1_out,
        "task2": t2,
        "task3": t3,
        "task5": t5,
        "verdict": verdict,
        "gates": {
            "pooled_dims_gate (>=5 dims AUC>0.65)": pooled_pass,
            "pca_lr_gate (LR>0.85 OR top3 PCA>0.70)": pca_pass,
            "residual_anomaly_gate (>=1 of 5 above p90)": residual_pass,
        },
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
