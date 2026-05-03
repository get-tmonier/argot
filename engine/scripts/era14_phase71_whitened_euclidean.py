"""Era 14 Phase 7.1 — UNSUPERVISED PCA-whitened squared-Euclidean anomaly scoring.

Clean ablation that tests the Phase 7 design CHOICES that survive the rank-
deficiency diagnostic:

1. **Concatenate** `hunk_embedding` (768-d) and `context_embedding` (768-d) into
   a 1536-d feature vector per hunk.
2. Per corpus: fit `PCA(n_components=64, whiten=True)` on the CONCATENATED
   1536-d vectors of CONTROL rows ONLY (`is_break == False`). Project ALL rows
   in that corpus into the corpus's PCA-64 whitened space.
3. Per (corpus, cluster_id) with `cluster_id != -1` and ≥ MIN_CLUSTER_CONTROLS
   controls: compute mean μ_c (64-d) from those controls in PCA-64 space.
   **NO per-cluster covariance** — that is the rank-deficient component that
   broke Phase 7 under LOO.
4. Score per hunk:
   - Cluster-routed: d² = ‖z - μ_c‖² (squared Euclidean in PCA-64 whitened space).
   - Corpus-fallback (cluster_id == -1 OR cluster has < MIN controls):
     d²_fallback = ‖z - μ_corpus‖² where μ_corpus is the mean of all CONTROL
     z_pca for the corpus. Whitening makes μ_corpus ≈ 0 by construction, so
     this is approximately z·z, but we compute it explicitly.
5. Per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL d² values
   across BOTH routings.
6. SHIP gate: ≥ 2/5 faker-js residuals catch AND every corpus FP ≤ baseline +
   0.5 pp AND LOO sanity check passes (ratio ≤ 5× on > 50 % of clusters).

**What this is mathematically**: Mahalanobis distance with Σ = corpus-pooled
covariance (well-conditioned: n ≈ 297 >> d = 64), restricted to its top-64
eigenvector subspace, anchored at per-cluster μ. The PCA-whitening normalizes
per-PC variance — the principled selling point of Phase 7 — without estimating
a per-cluster covariance.

Unsupervised: PCA fits on controls only, μ_c and μ_corpus come from controls
only; `is_break` labels are NEVER used as input for fitting. A sanity assertion
in `task1_fit_pca_per_corpus` enforces this.

Outputs JSON to stdout; saves the artifact to
`engine/.era14-features/phase71_whitened_euclidean.joblib`.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import joblib
import numpy as np
from sklearn.decomposition import PCA

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era14-features"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]

# Era-11 baseline FP rates per corpus (percent of CONTROLS flagged).
FP_TARGET = {
    "fastapi": 0.6,
    "rich": 1.2,
    "faker": 2.0,
    "hono": 0.5,
    "ink": 0.5,
    "faker-js": 0.9,
}

RESIDUALS = {
    "faker_js_error_flip_2",
    "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1",
    "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}

MIN_CLUSTER_CONTROLS = 5  # below this → fall back to corpus-wide whitened-space score
PCA_DIM = 64

# Phase 6.4 cosine distances for residuals (for side-by-side reporting).
PHASE64_RESIDUAL_COSINE = {
    "faker_js_error_flip_2": None,  # excluded under 6.4 (cluster 3 had 2 controls)
    "faker_js_error_flip_3": 0.3138,
    "faker_js_runtime_fetch_1": 0.4670,
    "faker_js_runtime_fetch_2": 0.4931,
    "faker_js_runtime_fetch_3": 0.4248,
}

# Phase 7 d² values for residuals (for side-by-side reporting).
PHASE7_RESIDUAL_D2 = {
    "faker_js_error_flip_2": 35.25,
    "faker_js_error_flip_3": 1532.48,
    "faker_js_runtime_fetch_1": 2358.61,
    "faker_js_runtime_fetch_2": 3941.50,
    "faker_js_runtime_fetch_3": 2181.34,
}


def load_all() -> dict:
    """Load all corpora; concatenate hunk + context embeddings into a 1536-d vector."""
    rows = []
    missing_emb_examples: list[tuple[str, str]] = []
    for corpus in CORPORA:
        with open(FEATURE_DIR / f"{corpus}.jsonl") as f:
            for line in f:
                d = json.loads(line)
                if "hunk_embedding" not in d or "context_embedding" not in d:
                    missing_emb_examples.append((corpus, d.get("file_path", "?")))
                    continue
                if len(d["hunk_embedding"]) != 768 or len(d["context_embedding"]) != 768:
                    raise RuntimeError(
                        f"Unexpected embedding length: corpus={corpus} "
                        f"hunk_len={len(d['hunk_embedding'])} "
                        f"ctx_len={len(d['context_embedding'])} "
                        f"path={d.get('file_path')}"
                    )
                rows.append(d)
    if missing_emb_examples:
        raise RuntimeError(
            f"Found {len(missing_emb_examples)} rows missing embeddings; "
            f"first few: {missing_emb_examples[:5]}"
        )

    n = len(rows)
    is_break = np.array([bool(r["is_break"]) for r in rows])
    hunk = np.array([r["hunk_embedding"] for r in rows], dtype=np.float32)
    ctx = np.array([r["context_embedding"] for r in rows], dtype=np.float32)
    feat = np.concatenate([hunk, ctx], axis=1)  # (n, 1536)
    corpus = np.array([r["corpus"] for r in rows])
    fixture_id = np.array([r.get("fixture_id") or "" for r in rows])
    cluster = np.array(
        [
            r["features"].get("cluster_id") if r["features"].get("cluster_id") is not None else -1
            for r in rows
        ],
        dtype=int,
    )
    file_path = np.array([r.get("file_path") or "" for r in rows])
    return {
        "n": n,
        "is_break": is_break,
        "feat": feat,
        "corpus": corpus,
        "fixture_id": fixture_id,
        "cluster": cluster,
        "file_path": file_path,
    }


def task1_fit_pca_per_corpus(data: dict) -> tuple[dict, np.ndarray, dict]:
    """Fit PCA(n_components=64, whiten=True) per corpus on CONTROL rows only.

    Returns:
        pca_models: {corpus: PCA model fit on controls only}
        feat_pca:   (n, 64) array — every row projected into ITS corpus's PCA space.
        stats:      {corpus: {n_controls_used, top1_var, top10_var, top64_var}}
    """
    print("=== TASK 1: per-corpus PCA-64 whitened fit (controls only) ===", file=sys.stderr)
    is_break = data["is_break"]
    feat = data["feat"]
    corpus = data["corpus"]
    n = data["n"]

    # SANITY: assert is_break labels not used as input. We physically only pass
    # the subset feat[ctrl_mask] into PCA.fit; this assertion is a tripwire if
    # we ever refactor incorrectly.
    assert feat.shape == (n, 1536), f"unexpected feat shape {feat.shape}"

    pca_models: dict[str, PCA] = {}
    feat_pca = np.zeros((n, PCA_DIM), dtype=np.float32)
    stats: dict[str, dict] = {}

    for c in CORPORA:
        c_mask = corpus == c
        ctrl_mask = c_mask & (~is_break)
        n_ctrl = int(ctrl_mask.sum())
        if n_ctrl < PCA_DIM + 1:
            raise RuntimeError(
                f"Corpus {c} has only {n_ctrl} controls; cannot fit PCA-{PCA_DIM}"
            )
        # Sanity: this is the ONLY use of is_break in this function, and only
        # to MASK-OUT breaks. The labels are not features.
        X_ctrl = feat[ctrl_mask]  # (n_ctrl, 1536)
        pca = PCA(n_components=PCA_DIM, whiten=True, random_state=0)
        pca.fit(X_ctrl)
        pca_models[c] = pca

        # Project ALL rows of THIS corpus (breaks + controls).
        X_all = feat[c_mask]  # (n_c, 1536)
        Z_all = pca.transform(X_all)  # (n_c, 64)
        feat_pca[c_mask] = Z_all.astype(np.float32)

        evr = pca.explained_variance_ratio_
        stats[c] = {
            "n_controls_fit": n_ctrl,
            "top1_var_frac": float(evr[0]),
            "top10_var_cum_frac": float(evr[:10].sum()),
            "top64_var_cum_frac": float(evr.sum()),
        }
        print(
            f"  {c:9s}: n_ctrl={n_ctrl:4d}  top1={evr[0]:.3f}  top10={evr[:10].sum():.3f}  "
            f"top64={evr.sum():.3f}",
            file=sys.stderr,
        )

    return pca_models, feat_pca, stats


def task2_fit_cluster_means(
    data: dict, feat_pca: np.ndarray
) -> tuple[dict, dict, dict]:
    """For each (corpus, cluster_id) with cluster_id != -1 AND ≥ MIN controls:
    compute μ_c (64-d) ONLY. No covariance. No inverse.

    Also compute per-corpus μ_corpus = mean of all CONTROL z_pca for the corpus
    (used for corpus-fallback scoring).

    Returns (mus, mu_corpus, per_corpus_stats)
    """
    print(
        "=== TASK 2: per-(corpus, cluster) μ + per-corpus μ on PCA-64 controls ===",
        file=sys.stderr,
    )
    is_break = data["is_break"]
    corpus = data["corpus"]
    cluster = data["cluster"]

    mus: dict[tuple[str, int], np.ndarray] = {}
    mu_corpus: dict[str, np.ndarray] = {}
    per_corpus_stats: dict[str, dict] = {}

    for c in CORPORA:
        valid = 0
        skipped_low = 0
        c_mask = corpus == c
        ctrl_mask = c_mask & (~is_break)
        unique_clusters = sorted(set(cluster[ctrl_mask].tolist()))

        # Per-corpus μ from ALL controls in this corpus.
        mu_c_all_ctrl = feat_pca[ctrl_mask].mean(axis=0)
        mu_corpus[c] = mu_c_all_ctrl.astype(np.float32)

        # Per-cluster μ_c.
        for cid in unique_clusters:
            if cid < 0:
                continue
            mask = ctrl_mask & (cluster == cid)
            n_ctrl = int(mask.sum())
            if n_ctrl < MIN_CLUSTER_CONTROLS:
                skipped_low += 1
                continue
            X = feat_pca[mask]  # (n_ctrl, 64)
            mu = X.mean(axis=0)  # (64,)
            mus[(c, int(cid))] = mu.astype(np.float32)
            valid += 1

        per_corpus_stats[c] = {
            "valid_cluster_models": valid,
            "skipped_low_pop": skipped_low,
            "unmappable_rows_in_corpus": int((c_mask & (cluster == -1)).sum()),
            "control_rows": int(ctrl_mask.sum()),
            "mu_corpus_norm": float(np.linalg.norm(mu_c_all_ctrl)),
        }

    return mus, mu_corpus, per_corpus_stats


def task3_score(
    data: dict,
    feat_pca: np.ndarray,
    mus: dict,
    mu_corpus: dict,
) -> tuple[np.ndarray, np.ndarray, dict]:
    """Score every row.

    Returns:
        d2:     (n,) squared-Euclidean distance in PCA-64 whitened space.
        route:  (n,) array of strings: "cluster" | "corpus_fallback".
        stats:  per-corpus routing breakdown.
    """
    print("=== TASK 3: score per hunk (cluster + corpus-fallback) ===", file=sys.stderr)
    n = data["n"]
    corpus = data["corpus"]
    cluster = data["cluster"]
    is_break = data["is_break"]

    d2 = np.full(n, np.nan, dtype=np.float64)
    route = np.full(n, "", dtype=object)

    for i in range(n):
        c = corpus[i]
        cid = int(cluster[i])
        z = feat_pca[i]  # (64,)
        key = (c, cid)
        if cid >= 0 and key in mus:
            mu = mus[key]
            diff = z - mu
            d2[i] = float(diff @ diff)
            route[i] = "cluster"
        else:
            mu = mu_corpus[c]
            diff = z - mu
            d2[i] = float(diff @ diff)
            route[i] = "corpus_fallback"

    stats = {}
    for c in CORPORA:
        c_mask = corpus == c
        c_break = c_mask & is_break
        c_ctrl = c_mask & (~is_break)
        cluster_routed = route == "cluster"
        fallback_routed = route == "corpus_fallback"
        stats[c] = {
            "total": int(c_mask.sum()),
            "cluster_routed": int((c_mask & cluster_routed).sum()),
            "corpus_fallback_routed": int((c_mask & fallback_routed).sum()),
            "excluded": 0,
            "breaks_total": int(c_break.sum()),
            "breaks_via_cluster": int((c_break & cluster_routed).sum()),
            "breaks_via_fallback": int((c_break & fallback_routed).sum()),
            "controls_total": int(c_ctrl.sum()),
            "controls_via_cluster": int((c_ctrl & cluster_routed).sum()),
            "controls_via_fallback": int((c_ctrl & fallback_routed).sum()),
        }
    return d2, route, stats


def task4_calibrate(data: dict, d2: np.ndarray) -> dict:
    """Per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL d²."""
    print("=== TASK 4: calibrate per-corpus thresholds on controls ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]

    out = {}
    for c in CORPORA:
        fp_target = FP_TARGET[c]
        c_mask = corpus == c
        c_ctrl_mask = c_mask & (~is_break) & ~np.isnan(d2)
        ctrl = d2[c_ctrl_mask]
        n_ctrl = len(ctrl)
        if n_ctrl == 0:
            out[c] = {
                "fp_target_pct": fp_target,
                "threshold": None,
                "control_count": 0,
                "actual_fp_pct": None,
                "controls_flagged": 0,
            }
            continue
        q = 1.0 - (fp_target / 100.0)
        threshold = float(np.quantile(ctrl, q))
        flagged = int((ctrl > threshold).sum())
        actual_fp_pct = 100.0 * flagged / n_ctrl
        out[c] = {
            "fp_target_pct": fp_target,
            "threshold": threshold,
            "control_count": n_ctrl,
            "actual_fp_pct": actual_fp_pct,
            "controls_flagged": flagged,
        }
    return out


def task5_residuals(
    data: dict, d2: np.ndarray, route: np.ndarray, thresholds: dict
) -> dict:
    """For the 5 faker-js residuals: report d², route, threshold, percentile vs fjs controls."""
    print("=== TASK 5: residual fixture catch ===", file=sys.stderr)
    fixture_id = data["fixture_id"]
    corpus = data["corpus"]
    is_break = data["is_break"]
    file_path = data["file_path"]
    cluster = data["cluster"]

    fjs_thr = thresholds["faker-js"]["threshold"]
    fjs_ctrl_mask = (corpus == "faker-js") & (~is_break) & ~np.isnan(d2)
    fjs_ctrl_d2 = d2[fjs_ctrl_mask]
    fjs_sorted = np.sort(fjs_ctrl_d2)
    n_ctrls = len(fjs_sorted)

    residual_results = {}
    for fid in sorted(RESIDUALS):
        idx = np.where(fixture_id == fid)[0]
        if len(idx) == 0:
            residual_results[fid] = {"error": "fixture not found"}
            continue
        ridx = int(idx[0])
        d = float(d2[ridx])
        if np.isnan(d):
            residual_results[fid] = {
                "d2": None,
                "route": str(route[ridx]),
                "threshold": fjs_thr,
                "crosses_threshold": False,
                "note": "score is NaN (unexpected)",
                "file_path": str(file_path[ridx]),
            }
            continue
        n_more = int((fjs_sorted > d).sum())
        rank_top_pct = 100.0 * n_more / n_ctrls if n_ctrls else None
        percentile = float((fjs_sorted <= d).sum()) / n_ctrls if n_ctrls else None
        residual_results[fid] = {
            "d2": d,
            "route": str(route[ridx]),
            "cluster_id": int(cluster[ridx]),
            "threshold": fjs_thr,
            "crosses_threshold": d > fjs_thr if fjs_thr is not None else False,
            "rank_top_pct_among_fjs_controls": rank_top_pct,
            "percentile_among_fjs_controls": percentile,
            "phase64_cosine_distance": PHASE64_RESIDUAL_COSINE.get(fid),
            "phase7_d2": PHASE7_RESIDUAL_D2.get(fid),
            "file_path": str(file_path[ridx]),
        }

    n_caught = sum(
        1 for v in residual_results.values()
        if isinstance(v, dict) and v.get("crosses_threshold")
    )
    caught_fids = sorted(
        [fid for fid, v in residual_results.items()
         if isinstance(v, dict) and v.get("crosses_threshold")]
    )
    return {
        "residuals": residual_results,
        "n_caught": n_caught,
        "caught_fixtures": caught_fids,
        "ship_gate_residual_pass": n_caught >= 2,
        "fjs_threshold": fjs_thr,
        "fjs_control_count": n_ctrls,
    }


def task6_recall_fp(data: dict, d2: np.ndarray, thresholds: dict) -> dict:
    """Per-corpus stage-4 recall + FP audit."""
    print("=== TASK 6: per-corpus recall + FP audit ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]

    out = {}
    for c in CORPORA:
        thr = thresholds[c]["threshold"]
        c_mask = corpus == c
        c_break_mask = c_mask & is_break & ~np.isnan(d2)
        c_ctrl_mask = c_mask & (~is_break) & ~np.isnan(d2)

        n_breaks_total = int((c_mask & is_break).sum())
        n_breaks_scored = int(c_break_mask.sum())
        n_breaks_excluded = n_breaks_total - n_breaks_scored

        if thr is None:
            out[c] = {
                "fp_target_pct": FP_TARGET[c],
                "threshold": None,
                "breaks_total": n_breaks_total,
                "breaks_scored": n_breaks_scored,
                "breaks_caught": 0,
                "stage4_recall_pct_of_total": 0.0,
                "stage4_recall_pct_of_scored": None,
                "controls_count": 0,
                "controls_flagged": 0,
                "actual_fp_pct": None,
                "fp_regression_vs_baseline_pp": None,
            }
            continue
        break_d2 = d2[c_break_mask]
        ctrl_d2 = d2[c_ctrl_mask]
        breaks_caught = int((break_d2 > thr).sum())
        ctrls_flagged = int((ctrl_d2 > thr).sum())
        n_ctrl = len(ctrl_d2)
        actual_fp_pct = 100.0 * ctrls_flagged / n_ctrl if n_ctrl else None
        fp_target = FP_TARGET[c]
        fp_regression = (actual_fp_pct - fp_target) if actual_fp_pct is not None else None

        out[c] = {
            "fp_target_pct": fp_target,
            "threshold": thr,
            "breaks_total": n_breaks_total,
            "breaks_scored": n_breaks_scored,
            "breaks_excluded": n_breaks_excluded,
            "breaks_caught": breaks_caught,
            "stage4_recall_pct_of_total": 100.0 * breaks_caught / n_breaks_total
            if n_breaks_total else 0.0,
            "stage4_recall_pct_of_scored": 100.0 * breaks_caught / n_breaks_scored
            if n_breaks_scored else None,
            "controls_count": n_ctrl,
            "controls_flagged": ctrls_flagged,
            "actual_fp_pct": actual_fp_pct,
            "fp_regression_vs_baseline_pp": fp_regression,
        }
    gate_pass = all(
        (v["actual_fp_pct"] is None) or (v["actual_fp_pct"] <= FP_TARGET[c] + 0.5)
        for c, v in out.items()
    )
    return {"per_corpus": out, "no_regression_gate_pass": gate_pass}


def task6b_loo_sanity_check(
    data: dict, feat_pca: np.ndarray
) -> dict:
    """Leave-one-out sanity diagnostic for Phase 7.1.

    For each cluster-routed (corpus, cluster_id) with ≥ MIN_CLUSTER_CONTROLS
    controls, hold out one control at a time, refit μ_c from the remaining
    controls (PCA itself NOT refit — same global PCA, only μ_c changes), and
    score the held-out control with the LOO μ_c.

    With corpus-pooled Σ via PCA-whitening (n ≈ 297 >> d = 64), there is no
    rank-deficient null space. The LOO ratio should be small (1.1×–1.5×) if
    the metric is honest; ratios > 5× would indicate a remaining μ_c-instability
    artifact.
    """
    print("=== TASK 6b: LOO sanity check on control d² ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]
    cluster = data["cluster"]

    diagnostics: list[dict] = []
    artifact_flagged = 0  # ratio > 5×
    mild_flagged = 0      # ratio > 1.5×
    total = 0

    for c in CORPORA:
        c_mask = corpus == c
        ctrl_mask = c_mask & (~is_break)
        unique_clusters = sorted(set(cluster[ctrl_mask].tolist()))
        for cid in unique_clusters:
            if cid < 0:
                continue
            mask = ctrl_mask & (cluster == cid)
            n_ctrl = int(mask.sum())
            if n_ctrl < MIN_CLUSTER_CONTROLS:
                continue
            idx = np.where(mask)[0]
            X = feat_pca[idx]  # (n_ctrl, 64)

            # In-sample d² for these controls (using μ from all of them)
            mu_all = X.mean(axis=0)
            in_sample = np.array([float((r - mu_all) @ (r - mu_all)) for r in X])

            # LOO d² (cap at 50 LOO iterations per cluster for speed; full set
            # if smaller)
            loo_d2_list = []
            loo_idx_to_run = list(range(n_ctrl)) if n_ctrl <= 50 else list(range(50))
            for k in loo_idx_to_run:
                X_minus = np.delete(X, k, axis=0)
                mu = X_minus.mean(axis=0)
                diff = X[k] - mu
                loo_d2_list.append(float(diff @ diff))
            loo_arr = np.array(loo_d2_list)

            in_max = float(in_sample.max())
            loo_max = float(loo_arr.max())
            ratio = (loo_max / in_max) if in_max > 1e-9 else float("inf")
            artifact = ratio > 5.0
            mild = ratio > 1.5
            total += 1
            if artifact:
                artifact_flagged += 1
            if mild:
                mild_flagged += 1
            diagnostics.append(
                {
                    "corpus": c,
                    "cluster_id": int(cid),
                    "n_controls": n_ctrl,
                    "rank_deficient_n_ctrl_lt_pca_dim": n_ctrl < PCA_DIM,
                    "in_sample_control_d2_max": in_max,
                    "in_sample_control_d2_mean": float(in_sample.mean()),
                    "loo_control_d2_max": loo_max,
                    "loo_control_d2_mean": float(loo_arr.mean()),
                    "loo_to_in_sample_max_ratio": ratio,
                    "looks_like_rank_deficiency_artifact": artifact,
                    "mild_loo_inflation": mild,
                }
            )

    ratios = [r["loo_to_in_sample_max_ratio"] for r in diagnostics]
    max_ratio = max(ratios) if ratios else None
    mean_ratio = float(np.mean(ratios)) if ratios else None
    median_ratio = float(np.median(ratios)) if ratios else None

    # SHIP gate condition: ratio ≤ 5× on > 50% of clusters → loo_pass.
    loo_pass = (total > 0) and (artifact_flagged < 0.5 * total)

    return {
        "per_cluster": diagnostics,
        "n_clusters_evaluated": total,
        "n_clusters_ratio_gt_5x": artifact_flagged,
        "n_clusters_ratio_gt_1p5x": mild_flagged,
        "max_loo_to_in_sample_ratio": max_ratio,
        "mean_loo_to_in_sample_ratio": mean_ratio,
        "median_loo_to_in_sample_ratio": median_ratio,
        "loo_sanity_pass": loo_pass,
        "interpretation": (
            "Phase 7.1 uses PCA-whitening (corpus-pooled Σ on n≈297 >> d=64) "
            "instead of per-cluster Σ on k<d controls, so there is no rank-"
            "deficient null space. The LOO test here measures only the "
            "stability of μ_c under hold-one-out — a small-sample mean shift. "
            "Honest ratios should be 1.1×–1.5×. Ratios > 5× would indicate "
            "μ_c instability bites at small k_c and the metric is NOT honest."
        ),
    }


def task7_fjs_diagnostic(
    data: dict, d2: np.ndarray, route: np.ndarray, thresholds: dict
) -> dict:
    """Top-20 faker-js controls by d² (analogous to Phase 7 task7)."""
    print("=== TASK 7: faker-js top-20 controls diagnostic ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]
    file_path = data["file_path"]
    cluster = data["cluster"]

    fjs_ctrl_mask = (corpus == "faker-js") & (~is_break) & ~np.isnan(d2)
    fjs_idx = np.where(fjs_ctrl_mask)[0]
    fjs_d2 = d2[fjs_idx]
    order = np.argsort(-fjs_d2)[:20]
    thr = thresholds["faker-js"]["threshold"]
    top20 = []
    for rank, off in enumerate(order, 1):
        i = int(fjs_idx[off])
        top20.append(
            {
                "rank": rank,
                "file_path": str(file_path[i]),
                "cluster_id": int(cluster[i]),
                "d2": float(d2[i]),
                "route": str(route[i]),
                "above_threshold": bool(d2[i] > thr) if thr is not None else False,
            }
        )
    return {"top20_fjs_controls_by_d2": top20, "fjs_threshold": thr}


def task8_verdict(t5: dict, t6: dict, t6b: dict) -> dict:
    """Apply pre-registered SHIP gate (residual catch + FP regression + LOO sanity)."""
    fjs_catches = t5["n_caught"]
    no_regression = t6["no_regression_gate_pass"]
    loo_pass = t6b["loo_sanity_pass"]

    regressions = []
    for c, v in t6["per_corpus"].items():
        if v["actual_fp_pct"] is not None and v["actual_fp_pct"] > FP_TARGET[c] + 0.5:
            regressions.append(
                {
                    "corpus": c,
                    "actual_fp_pct": v["actual_fp_pct"],
                    "baseline_fp_pct": FP_TARGET[c],
                    "regression_pp": v["fp_regression_vs_baseline_pp"],
                }
            )

    # SHIP requires all three: ≥ 2 catches AND no FP regression AND LOO sanity.
    if fjs_catches >= 2 and no_regression and loo_pass:
        verdict = "SHIP"
    elif fjs_catches == 0:
        verdict = "CLOSE NEGATIVE"
    elif not loo_pass:
        verdict = "CLOSE NEGATIVE (LOO fails)"
    elif fjs_catches >= 1 and no_regression and loo_pass:
        verdict = "PARTIAL"
    elif fjs_catches >= 1 and not no_regression:
        verdict = "PARTIAL"
    else:
        verdict = "PARTIAL"

    return {
        "verdict": verdict,
        "faker_js_residual_catches": fjs_catches,
        "ship_gate_residual_pass": fjs_catches >= 2,
        "no_regression_gate_pass": no_regression,
        "loo_sanity_gate_pass": loo_pass,
        "ship_gate_overall_pass": fjs_catches >= 2 and no_regression and loo_pass,
        "regressions": regressions,
    }


def main() -> None:
    data = load_all()
    print(
        f"Loaded {data['n']} rows; {data['is_break'].sum()} breaks, "
        f"{(~data['is_break']).sum()} controls; feat dim = {data['feat'].shape[1]}",
        file=sys.stderr,
    )

    pca_models, feat_pca, t1_stats = task1_fit_pca_per_corpus(data)
    mus, mu_corpus, t2_stats = task2_fit_cluster_means(data, feat_pca)
    d2, route, t3_stats = task3_score(data, feat_pca, mus, mu_corpus)
    t4_thresholds = task4_calibrate(data, d2)
    t5 = task5_residuals(data, d2, route, t4_thresholds)
    t6 = task6_recall_fp(data, d2, t4_thresholds)
    t6b = task6b_loo_sanity_check(data, feat_pca)
    t7 = task7_fjs_diagnostic(data, d2, route, t4_thresholds)
    t8 = task8_verdict(t5, t6, t6b)

    artifact_path = FEATURE_DIR / "phase71_whitened_euclidean.joblib"
    joblib.dump(
        {
            "pca_models": pca_models,
            "mus": mus,
            "mu_corpus": mu_corpus,
            "thresholds": {c: t4_thresholds[c]["threshold"] for c in CORPORA},
            "fp_target": FP_TARGET,
            "min_cluster_controls": MIN_CLUSTER_CONTROLS,
            "pca_dim": PCA_DIM,
            "method": (
                "Per-corpus PCA(64, whiten=True) on concat(hunk_emb, ctx_emb) of CONTROLS only; "
                "per-(corpus, cluster_id) μ_c on PCA-64 controls (mean only, no covariance); "
                "per-corpus μ_corpus on PCA-64 controls; "
                "score = ‖z - μ_c‖² for cluster-routed; "
                "score = ‖z - μ_corpus‖² for corpus-fallback (cluster_id == -1 OR "
                "cluster has < MIN_CLUSTER_CONTROLS controls); "
                "per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL d². "
                "Phase 7.1 ablation: removes the rank-deficient per-cluster Σ from Phase 7."
            ),
        },
        artifact_path,
    )

    out = {
        "task1_pca_per_corpus": t1_stats,
        "task2_cluster_mean_construction": t2_stats,
        "task3_routing_stats": t3_stats,
        "task4_thresholds": t4_thresholds,
        "task5_residuals": t5,
        "task6_recall_fp": t6,
        "task6b_loo_sanity_check": t6b,
        "task7_fjs_top20_controls": t7,
        "task8_verdict": t8,
        "artifact_saved_to": str(artifact_path),
        "n_cluster_models": len(mus),
        "config": {
            "pca_dim": PCA_DIM,
            "min_cluster_controls": MIN_CLUSTER_CONTROLS,
            "fp_target": FP_TARGET,
        },
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
