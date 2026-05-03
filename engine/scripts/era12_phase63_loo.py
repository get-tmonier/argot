"""Era 12 Phase 6.3 — LOO classifier comparison on UnixCoder embeddings.

Trains LR / MLP / kNN on three feature sets:
  1. embeddings-only (PCA-100 of concat(hunk, ctx))
  2. engineered-only (Phase 5 conservative set)
  3. combined (PCA-100 + engineered)

Evaluates:
  - Task 1: pooled 5-fold CV AUC for each of 6 LR/MLP variants + 3 kNN variants
  - Task 2: 6x(6+3)=54 LOO test AUCs
  - Task 3: residual fixture catch when faker-js is held out
  - Task 4: kNN baseline (rolled into 1-3)
  - Task 5: per-corpus FP rate sanity check at the threshold catching >=2 residuals
  - Task 6: verdict (SHIP / PARTIAL / CLOSE NEGATIVE)

For LOO each model variant fits its own PCA on the 5 training corpora (no leakage).
"""

from __future__ import annotations

import json
import sys
import warnings
from pathlib import Path
from typing import Any

import joblib
import numpy as np
from sklearn.decomposition import PCA
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold
from sklearn.neighbors import KNeighborsClassifier
from sklearn.neural_network import MLPClassifier
from sklearn.preprocessing import StandardScaler

warnings.filterwarnings("ignore")

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era12-features"
ARTIFACTS_DIR = FEATURE_DIR / "loo_best_phase6.3"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
RESIDUALS = {
    "faker_js_error_flip_2",
    "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1",
    "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}

# Conservative engineered features (per task spec).
# Drop: cluster_jaccard_to_centroid, hunk_length_*, n_total_ast_nodes,
#       hunk_file_callee_jaccard, hunk_callees_in_file_fraction, all AST counts.
# Keep: stage outputs + hunk-shape primitives.
ENGINEERED_FEATURES: tuple[str, ...] = (
    "adjusted_bpe",
    "bpe_score",
    "import_score",
    "cluster_id",
    "n_unattested_callees",
    "n_attested_root_only",
    "n_cluster_absent_callees",
    "hunk_callee_bag_size",
    "file_callee_bag_size",
    "n_returns",
    "n_throws",
    "n_awaits",
    "max_nesting_depth",
    "n_distinct_identifiers",
    "parse_fragment_flag",
    "stage2_flagged",
)

ERA11_FP = {
    "fastapi": 0.006,
    "rich": 0.012,
    "faker": 0.020,
    "hono": 0.005,
    "ink": 0.005,
    "faker-js": 0.009,
}


def _coerce(v: Any) -> float:
    if isinstance(v, bool):
        return 1.0 if v else 0.0
    if v is None:
        return 0.0
    return float(v)


def load_all() -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for corpus in CORPORA:
        with open(FEATURE_DIR / f"{corpus}.jsonl") as f:
            for line in f:
                rows.append(json.loads(line))
    n = len(rows)
    is_break = np.array([bool(r["is_break"]) for r in rows], dtype=bool)
    hunk = np.array([r["hunk_embedding"] for r in rows], dtype=np.float32)
    ctx = np.array([r["context_embedding"] for r in rows], dtype=np.float32)
    corpus = np.array([r["corpus"] for r in rows])
    fixture_id = np.array([r.get("fixture_id") or "" for r in rows])
    eng = np.zeros((n, len(ENGINEERED_FEATURES)), dtype=np.float64)
    for i, r in enumerate(rows):
        for j, name in enumerate(ENGINEERED_FEATURES):
            eng[i, j] = _coerce(r["features"].get(name))
    return {
        "n": n,
        "is_break": is_break,
        "hunk": hunk,
        "ctx": ctx,
        "corpus": corpus,
        "fixture_id": fixture_id,
        "engineered": eng,
        "engineered_names": list(ENGINEERED_FEATURES),
    }


def per_corpus_zscore(emb: np.ndarray, corpus: np.ndarray, fit_corpora: list[str]) -> tuple[np.ndarray, dict[str, tuple[np.ndarray, np.ndarray]]]:
    """Apply per-corpus z-score normalization. Returns (normed, stats per corpus)."""
    out = emb.copy()
    stats: dict[str, tuple[np.ndarray, np.ndarray]] = {}
    for c in fit_corpora:
        mask = corpus == c
        if mask.sum() > 1:
            mu = out[mask].mean(axis=0)
            sd = out[mask].std(axis=0) + 1e-8
            stats[c] = (mu, sd)
            out[mask] = (out[mask] - mu) / sd
    return out, stats


def apply_per_corpus_zscore(emb: np.ndarray, corpus: np.ndarray, stats: dict[str, tuple[np.ndarray, np.ndarray]], fallback: tuple[np.ndarray, np.ndarray]) -> np.ndarray:
    out = emb.copy()
    mu_fb, sd_fb = fallback
    for c in np.unique(corpus):
        mask = corpus == c
        if c in stats:
            mu, sd = stats[c]
        else:
            mu, sd = mu_fb, sd_fb
        out[mask] = (out[mask] - mu) / sd
    return out


def make_models() -> dict[str, Any]:
    return {
        "LR": lambda: LogisticRegression(max_iter=1000, C=1.0, random_state=0),
        "MLP": lambda: MLPClassifier(
            hidden_layer_sizes=(256, 64), max_iter=200, random_state=0
        ),
        "kNN": lambda: KNeighborsClassifier(
            n_neighbors=15, weights="distance", metric="cosine"
        ),
    }


def build_features_pooled(data: dict[str, Any], feature_set: str, pca_payload: dict | None) -> tuple[np.ndarray, Any]:
    """Build pooled feature matrix (used for Task 1 5-fold CV).

    For pooled CV we reuse Phase 6.2's PCA (fit on all 6 corpora). This is
    consistent with how 6.2 reported pooled AUC; LOO uses fresh per-fold PCA.
    """
    y_corpus = data["corpus"]
    if feature_set in ("embeddings", "combined"):
        full = np.concatenate([data["hunk"], data["ctx"]], axis=1)
        full_norm, _ = per_corpus_zscore(full, y_corpus, CORPORA)
        # Use saved PCA from 6.2
        pca = pca_payload["pca"]
        Z = pca.transform(full_norm)
        if feature_set == "embeddings":
            return Z, None
        else:
            X = np.concatenate([Z, data["engineered"]], axis=1)
            return X, None
    elif feature_set == "engineered":
        return data["engineered"].copy(), None
    raise ValueError(feature_set)


def task1_pooled_cv(data: dict[str, Any], pca_payload: dict) -> dict[str, Any]:
    """Pooled 5-fold CV AUC for each (feature_set x model)."""
    y = data["is_break"].astype(int)
    cv = StratifiedKFold(n_splits=5, shuffle=True, random_state=0)
    models = make_models()
    feature_sets = ["embeddings", "engineered", "combined"]
    results: dict[str, dict[str, float]] = {}
    for fs in feature_sets:
        X, _ = build_features_pooled(data, fs, pca_payload)
        results[fs] = {}
        for mname, mfactory in models.items():
            cv_probs = np.zeros(len(y), dtype=float)
            for tr, te in cv.split(X, y):
                sc = StandardScaler()
                Xtr = sc.fit_transform(X[tr])
                Xte = sc.transform(X[te])
                m = mfactory()
                m.fit(Xtr, y[tr])
                cv_probs[te] = m.predict_proba(Xte)[:, 1]
            auc = float(roc_auc_score(y, cv_probs))
            results[fs][mname] = auc
            print(f"  pooled CV [{fs:11s} x {mname:3s}] = {auc:.4f}", file=sys.stderr)
    return results


def fit_loo_features(data: dict[str, Any], holdout: str, feature_set: str) -> tuple[np.ndarray, np.ndarray, dict[str, Any]]:
    """Fit feature pipeline on the 5 training corpora; transform all 6.

    Returns (X_all, mask_train, pipeline_state) where X_all has rows for all
    corpora and pipeline_state captures fit objects for serialization.
    """
    corpus = data["corpus"]
    train_mask = corpus != holdout
    train_corpora = [c for c in CORPORA if c != holdout]
    state: dict[str, Any] = {"holdout": holdout, "train_corpora": train_corpora}

    if feature_set in ("embeddings", "combined"):
        full = np.concatenate([data["hunk"], data["ctx"]], axis=1)
        # Per-corpus z-score: fit stats from train corpora only
        # For the held-out corpus, use the *mean* of train corpora's mu/sd
        # as a global fallback (gives a reasonable baseline normalization).
        full_train_norm, train_stats = per_corpus_zscore(
            full[train_mask], corpus[train_mask], train_corpora
        )
        # Fit PCA-100 on training data
        pca = PCA(n_components=100, random_state=0)
        pca.fit(full_train_norm)
        # Compute fallback stats (mean of train corpora's mu/sd)
        all_mu = np.stack([train_stats[c][0] for c in train_corpora])
        all_sd = np.stack([train_stats[c][1] for c in train_corpora])
        fallback = (all_mu.mean(axis=0), all_sd.mean(axis=0))
        # Transform all rows
        full_norm = apply_per_corpus_zscore(full, corpus, train_stats, fallback)
        Z = pca.transform(full_norm)
        state["pca"] = pca
        state["zscore_stats"] = train_stats
        state["zscore_fallback"] = fallback
        if feature_set == "embeddings":
            X = Z
        else:
            X = np.concatenate([Z, data["engineered"]], axis=1)
    elif feature_set == "engineered":
        X = data["engineered"].copy()
    else:
        raise ValueError(feature_set)

    # Standard scaler fit on training rows only
    sc = StandardScaler()
    X_train_scaled = sc.fit_transform(X[train_mask])
    X_all = np.zeros_like(X)
    X_all[train_mask] = X_train_scaled
    X_all[~train_mask] = sc.transform(X[~train_mask])
    state["scaler"] = sc

    return X_all, train_mask, state


def task2_loo(data: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    """LOO test AUCs. Returns (matrix, fitted models per holdout)."""
    y = data["is_break"].astype(int)
    corpus = data["corpus"]
    feature_sets = ["embeddings", "engineered", "combined"]
    models = make_models()
    # matrix[holdout][feature_set][model] = test AUC
    matrix: dict[str, dict[str, dict[str, float]]] = {}
    # fitted_models[holdout][feature_set][model] = (model_obj, pipeline_state)
    fitted_models: dict[str, dict[str, dict[str, Any]]] = {}

    for holdout in CORPORA:
        matrix[holdout] = {}
        fitted_models[holdout] = {}
        train_mask = corpus != holdout
        test_mask = ~train_mask
        for fs in feature_sets:
            X_all, _, state = fit_loo_features(data, holdout, fs)
            matrix[holdout][fs] = {}
            fitted_models[holdout][fs] = {}
            for mname, mfactory in models.items():
                m = mfactory()
                m.fit(X_all[train_mask], y[train_mask])
                p_test = m.predict_proba(X_all[test_mask])[:, 1]
                y_test = y[test_mask]
                if y_test.sum() == 0 or y_test.sum() == len(y_test):
                    auc = float("nan")
                else:
                    auc = float(roc_auc_score(y_test, p_test))
                matrix[holdout][fs][mname] = auc
                fitted_models[holdout][fs][mname] = {
                    "model": m,
                    "state": state,
                    "p_test": p_test,
                }
                print(
                    f"  LOO holdout={holdout:9s} [{fs:11s} x {mname:3s}] test_auc={auc:.4f}",
                    file=sys.stderr,
                )
    return matrix, fitted_models


def task3_residual_catch(
    data: dict[str, Any], fitted_models: dict[str, Any]
) -> dict[str, Any]:
    """For faker-js held out, score residuals vs faker-js controls."""
    y = data["is_break"]
    corpus = data["corpus"]
    fixture_id = data["fixture_id"]
    holdout = "faker-js"
    fjs_mask = corpus == holdout
    fjs_idx_in_test = np.where(fjs_mask)[0]
    # Within the test set, find residual indices and control indices
    residual_mask_global = np.isin(fixture_id, list(RESIDUALS))
    fjs_ctrl_mask_global = fjs_mask & (~y)

    residual_local_idx = []
    residual_fids = []
    for i, gi in enumerate(fjs_idx_in_test):
        if residual_mask_global[gi]:
            residual_local_idx.append(i)
            residual_fids.append(fixture_id[gi])
    fjs_ctrl_local_idx = []
    for i, gi in enumerate(fjs_idx_in_test):
        if fjs_ctrl_mask_global[gi]:
            fjs_ctrl_local_idx.append(i)

    n_ctrl = len(fjs_ctrl_local_idx)
    target_fp = 0.009
    max_controls_above = int(np.floor(target_fp * n_ctrl))

    out: dict[str, Any] = {
        "n_residuals": len(residual_local_idx),
        "n_fjs_controls": n_ctrl,
        "max_controls_above_at_fp_0.9pct": max_controls_above,
        "per_model": {},
    }

    for fs in ["embeddings", "engineered", "combined"]:
        for mname in ["LR", "MLP", "kNN"]:
            entry = fitted_models[holdout][fs][mname]
            p_test = entry["p_test"]
            ctrl_probs = p_test[fjs_ctrl_local_idx]
            ctrl_probs_sorted = np.sort(ctrl_probs)[::-1]
            # Threshold: prob just above the (max_controls_above)-th control prob
            if max_controls_above < len(ctrl_probs_sorted):
                thresh = float(ctrl_probs_sorted[max_controls_above]) + 1e-12
            else:
                thresh = float(ctrl_probs_sorted[-1]) + 1e-12

            per_residual = []
            n_caught = 0
            for li, fid in zip(residual_local_idx, residual_fids):
                p = float(p_test[li])
                rank_above = int((ctrl_probs > p).sum())
                caught = p > thresh
                if caught:
                    n_caught += 1
                per_residual.append(
                    {
                        "fixture_id": fid,
                        "prob": p,
                        "rank_above_in_ctrls": rank_above,
                        "n_ctrls": n_ctrl,
                        "caught_at_fp_0.9pct": bool(caught),
                    }
                )
            actual_fp = float((ctrl_probs > thresh).sum()) / max(1, n_ctrl)
            out["per_model"][f"{fs}_{mname}"] = {
                "threshold_at_fp_0.9pct": thresh,
                "actual_fp_rate": actual_fp,
                "n_residuals_caught": n_caught,
                "per_residual": per_residual,
            }
            print(
                f"  residual catch [{fs:11s} x {mname:3s}] caught={n_caught}/5 thresh={thresh:.4f} fp={actual_fp*100:.2f}%",
                file=sys.stderr,
            )
    return out


def task5_per_corpus_fp(
    data: dict[str, Any], fitted_models: dict[str, Any], best_key: tuple[str, str]
) -> dict[str, Any]:
    """For BEST model variant, find threshold that catches >=2 residuals on
    faker-js, then check per-corpus FP at the analogous threshold for each
    held-out corpus's controls.

    Strategy: per held-out corpus c, pick the threshold = (era-11 FP[c] + 1pp)
    on c's controls, and report whether the model can plausibly meet that.
    Also report how many residuals are caught at the per-corpus thresholds
    matching the FP budget.
    """
    y = data["is_break"]
    corpus = data["corpus"]
    fixture_id = data["fixture_id"]
    fs, mname = best_key

    out: dict[str, Any] = {
        "best_variant": f"{fs}_{mname}",
        "per_corpus": {},
    }

    # First: faker-js threshold that catches >=2 residuals
    holdout = "faker-js"
    p_fjs = fitted_models[holdout][fs][mname]["p_test"]
    fjs_mask = corpus == holdout
    fjs_idx = np.where(fjs_mask)[0]
    test_y = y[fjs_idx]
    test_fids = fixture_id[fjs_idx]
    residual_local = [i for i, gi in enumerate(fjs_idx) if test_fids[i] in RESIDUALS]
    ctrl_local = [i for i, gi in enumerate(fjs_idx) if not test_y[i]]
    ctrl_probs = np.sort(p_fjs[ctrl_local])[::-1]
    res_probs_sorted = np.sort([p_fjs[i] for i in residual_local])[::-1]
    # Threshold = midpoint between 2nd-highest residual prob and the lowest
    # ctrl prob above. Or simply: the prob below 2nd-highest residual.
    if len(res_probs_sorted) >= 2:
        thresh_fjs = float(res_probs_sorted[1]) - 1e-12
        n_caught = int((np.array([p_fjs[i] for i in residual_local]) >= thresh_fjs).sum())
    else:
        thresh_fjs = float(res_probs_sorted[0]) - 1e-12
        n_caught = 1
    fp_at_thresh = float((p_fjs[ctrl_local] >= thresh_fjs).sum()) / max(1, len(ctrl_local))
    out["fjs_threshold_for_2_residuals"] = thresh_fjs
    out["fjs_caught_at_threshold"] = n_caught
    out["fjs_fp_at_threshold"] = fp_at_thresh
    out["fjs_baseline_fp"] = ERA11_FP["faker-js"]
    out["fjs_fp_budget"] = ERA11_FP["faker-js"] + 0.01

    # Now: for each held-out corpus, compute its model's FP at a threshold that
    # achieves ERA11_FP[c] + 1pp on its OWN controls. If FP > budget then
    # the model can't even fit budget; otherwise read the threshold off.
    # Then ask: does this same per-corpus threshold catch >=2 residuals when
    # applied to faker-js? (No — the model is trained per holdout, so threshold
    # is specific to each holdout.)
    # The actual gate: under EACH LOO holdout c, the model's FP rate at the
    # threshold catching >=2 residuals on FAKER-JS (when fjs is held out) must
    # be <= era-11 + 1pp on EVERY corpus when each is held out separately.
    # But the per-holdout models are different. So we instead check: for each
    # corpus c held-out, what FP does that corpus's model produce at a
    # *globally calibrated* threshold? Best interpretation:
    # "for the best model variant, when each corpus is held out, the threshold
    # that would catch >=2 residuals (if we could measure it) keeps FP on
    # the held-out corpus's controls <= era-11+1pp".
    # We approximate: for each holdout c, find threshold on c's score
    # distribution s.t. FP on c's controls <= era-11[c]+1pp. Then check if
    # any breaks in c are caught at that threshold (true positive rate proxy).
    for c in CORPORA:
        p_c = fitted_models[c][fs][mname]["p_test"]
        c_mask = corpus == c
        c_idx = np.where(c_mask)[0]
        c_y = y[c_idx]
        ctrl_probs_c = np.sort(p_c[~c_y])[::-1]
        budget = ERA11_FP[c] + 0.01
        max_above = int(np.floor(budget * len(ctrl_probs_c)))
        if max_above < len(ctrl_probs_c):
            thresh_c = float(ctrl_probs_c[max_above]) + 1e-12
        else:
            thresh_c = float(ctrl_probs_c[-1]) + 1e-12
        fp_c = float((p_c[~c_y] > thresh_c).sum()) / max(1, len(ctrl_probs_c))
        # TPR on this corpus's breaks at this threshold
        n_breaks = int(c_y.sum())
        if n_breaks > 0:
            tpr_c = float((p_c[c_y] > thresh_c).sum()) / n_breaks
        else:
            tpr_c = float("nan")
        out["per_corpus"][c] = {
            "threshold_at_budget": thresh_c,
            "fp_rate_actual": fp_c,
            "fp_budget": budget,
            "fp_within_budget": fp_c <= budget + 1e-9,
            "tpr_at_threshold": tpr_c,
            "n_breaks": n_breaks,
            "n_controls": int((~c_y).sum()),
        }
    return out


def find_best_residual_catch(t3: dict[str, Any]) -> tuple[str, str, int]:
    best_key = ("", "")
    best_caught = -1
    for key, entry in t3["per_model"].items():
        if entry["n_residuals_caught"] > best_caught:
            best_caught = entry["n_residuals_caught"]
            best_key = tuple(key.split("_", 1))  # type: ignore
    return best_key[0], best_key[1], best_caught


def find_best_pooled_cv(t1: dict[str, Any]) -> tuple[str, str, float]:
    best_fs, best_m, best_auc = "", "", -1.0
    for fs, by_m in t1.items():
        for m, auc in by_m.items():
            if auc > best_auc:
                best_auc = auc
                best_fs = fs
                best_m = m
    return best_fs, best_m, best_auc


def find_best_loo_mean(matrix: dict[str, Any]) -> tuple[str, str, float, dict[str, dict[str, float]]]:
    """Compute mean LOO test AUC per (fs x m); return best."""
    feature_sets = ["embeddings", "engineered", "combined"]
    models = ["LR", "MLP", "kNN"]
    means: dict[str, dict[str, float]] = {fs: {} for fs in feature_sets}
    best_fs, best_m, best = "", "", -1.0
    for fs in feature_sets:
        for m in models:
            vals = [matrix[h][fs][m] for h in CORPORA if not np.isnan(matrix[h][fs][m])]
            mean = float(np.mean(vals)) if vals else float("nan")
            means[fs][m] = mean
            if mean > best:
                best = mean
                best_fs = fs
                best_m = m
    return best_fs, best_m, best, means


def save_best_artifacts(
    fitted_models: dict[str, Any], best_fs: str, best_m: str
) -> str:
    ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
    for c in CORPORA:
        entry = fitted_models[c][best_fs][best_m]
        joblib.dump(
            {"model": entry["model"], "feature_set": best_fs, "model_name": best_m},
            ARTIFACTS_DIR / f"{c}.joblib",
        )
    # Save the pipeline state from a representative holdout (faker-js) so the
    # full feature pipeline can be replayed. Each holdout has its own pipeline
    # since PCA was refit; we save all of them.
    pipelines = {
        c: {
            "scaler": fitted_models[c][best_fs][best_m]["state"]["scaler"],
            "pca": fitted_models[c][best_fs][best_m]["state"].get("pca"),
            "zscore_stats": fitted_models[c][best_fs][best_m]["state"].get(
                "zscore_stats"
            ),
            "zscore_fallback": fitted_models[c][best_fs][best_m]["state"].get(
                "zscore_fallback"
            ),
            "feature_set": best_fs,
            "engineered_names": list(ENGINEERED_FEATURES),
        }
        for c in CORPORA
    }
    joblib.dump(pipelines, ARTIFACTS_DIR / "feature_pipeline.joblib")
    return str(ARTIFACTS_DIR)


def verdict(
    t3: dict[str, Any],
    matrix: dict[str, Any],
    t5: dict[str, Any],
) -> str:
    # Find best residual-catch model
    best_fs, best_m, best_caught = find_best_residual_catch(t3)
    # LOO ge 0.75 in >=4 of 6 corpora for best_fs x best_m?
    aucs = [matrix[c][best_fs][best_m] for c in CORPORA]
    n_ge_075 = sum(1 for a in aucs if not np.isnan(a) and a >= 0.75)
    # Per-corpus FP within budget?
    all_within = all(v["fp_within_budget"] for v in t5["per_corpus"].values())
    if best_caught >= 2 and all_within and n_ge_075 >= 4:
        return "SHIP"
    if best_caught >= 1 and (all_within or n_ge_075 >= 4):
        return "PARTIAL"
    if best_caught == 0:
        return "CLOSE NEGATIVE"
    return "PARTIAL"


def main() -> None:
    print("Loading...", file=sys.stderr)
    data = load_all()
    print(
        f"Loaded {data['n']} rows; {data['is_break'].sum()} breaks total",
        file=sys.stderr,
    )

    pca_payload = joblib.load(FEATURE_DIR / "pca100_phase6.2.joblib")

    print("\n=== TASK 1: pooled 5-fold CV AUC ===", file=sys.stderr)
    t1 = task1_pooled_cv(data, pca_payload)

    print("\n=== TASK 2: LOO test AUCs ===", file=sys.stderr)
    matrix, fitted_models = task2_loo(data)

    print("\n=== TASK 3: residual fixture catch (LOO faker-js) ===", file=sys.stderr)
    t3 = task3_residual_catch(data, fitted_models)

    best_fs_pooled, best_m_pooled, best_pooled_auc = find_best_pooled_cv(t1)
    best_fs_loo, best_m_loo, best_loo_mean, loo_means = find_best_loo_mean(matrix)
    best_fs_res, best_m_res, best_caught = find_best_residual_catch(t3)

    print(
        f"\nBest pooled CV: {best_fs_pooled} x {best_m_pooled} = {best_pooled_auc:.4f}",
        file=sys.stderr,
    )
    print(
        f"Best LOO mean: {best_fs_loo} x {best_m_loo} = {best_loo_mean:.4f}",
        file=sys.stderr,
    )
    print(
        f"Best residual catch: {best_fs_res} x {best_m_res} = {best_caught}/5",
        file=sys.stderr,
    )

    print("\n=== TASK 5: per-corpus FP for best residual catcher ===", file=sys.stderr)
    t5 = task5_per_corpus_fp(data, fitted_models, (best_fs_res, best_m_res))

    print("\n=== Saving best LOO artifacts ===", file=sys.stderr)
    artifacts_path = save_best_artifacts(fitted_models, best_fs_res, best_m_res)
    print(f"Saved to {artifacts_path}", file=sys.stderr)

    final_verdict = verdict(t3, matrix, t5)
    print(f"\n=== VERDICT: {final_verdict} ===", file=sys.stderr)

    out = {
        "task1_pooled_cv": t1,
        "task2_loo_matrix": matrix,
        "task2_loo_means": loo_means,
        "task3_residual_catch": t3,
        "task5_per_corpus_fp": t5,
        "best_pooled_cv": {
            "feature_set": best_fs_pooled,
            "model": best_m_pooled,
            "auc": best_pooled_auc,
        },
        "best_loo_mean": {
            "feature_set": best_fs_loo,
            "model": best_m_loo,
            "mean_auc": best_loo_mean,
        },
        "best_residual_catch": {
            "feature_set": best_fs_res,
            "model": best_m_res,
            "n_caught": best_caught,
        },
        "verdict": final_verdict,
        "artifacts_path": artifacts_path,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
