"""Era 14 Phase 6.4 — UNSUPERVISED cluster-departure scoring on UnixCoder embeddings.

Builds per-(corpus, cluster_id) centroids from CONTROL hunks only (no catalog leak),
scores every hunk by cosine distance to its cluster centroid, calibrates per-corpus
thresholds at the era-11 baseline FP rate, then evaluates fixture catch.

Outputs JSON results to stdout for the memo writer to consume, and saves the
centroid dict to .era14-features/centroids_phase6.4.joblib.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import joblib
import numpy as np

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

MIN_CLUSTER_CONTROLS = 5  # skip clusters with fewer than this many controls


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
        "hunk": hunk,
        "corpus": corpus,
        "fixture_id": fixture_id,
        "cluster": cluster,
        "file_path": file_path,
    }


def l2_normalize(x: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(x, axis=-1, keepdims=True) + 1e-12
    return x / norms


def task1_build_centroids(data: dict) -> tuple[dict, dict]:
    """Build centroids per (corpus, cluster_id) using CONTROL embeddings only.

    Returns (centroids, stats).
      centroids[(corpus, cluster_id)] = unit-normed centroid vector (768-d)
    """
    print("=== TASK 1: build per-(corpus, cluster) centroids from controls ===", file=sys.stderr)
    is_break = data["is_break"]
    hunk = data["hunk"]
    corpus = data["corpus"]
    cluster = data["cluster"]

    hunk_n = l2_normalize(hunk)

    centroids: dict[tuple[str, int], np.ndarray] = {}
    per_corpus_stats: dict[str, dict] = {}

    for c in CORPORA:
        valid_built = 0
        skipped_low_pop = 0
        unmappable = 0  # cluster == -1

        c_mask = corpus == c
        # All cluster ids attested in this corpus among CONTROLS
        ctrl_mask = c_mask & (~is_break)
        if ctrl_mask.sum() == 0:
            per_corpus_stats[c] = {
                "valid_centroids": 0,
                "skipped_low_pop": 0,
                "unmappable_rows_in_corpus": int((c_mask & (cluster == -1)).sum()),
                "control_rows": 0,
            }
            continue

        unique_clusters = sorted(set(cluster[ctrl_mask].tolist()))
        for cid in unique_clusters:
            if cid < 0:
                unmappable += int(((cluster == cid) & ctrl_mask).sum())
                continue
            mask = ctrl_mask & (cluster == cid)
            n_controls = int(mask.sum())
            if n_controls < MIN_CLUSTER_CONTROLS:
                skipped_low_pop += 1
                continue
            # mean of unit-normalized hunk embeddings (matches Phase 6.2)
            centroid = hunk_n[mask].mean(axis=0)
            # Normalize centroid for stable cosine distance.
            centroid = centroid / (np.linalg.norm(centroid) + 1e-12)
            centroids[(c, int(cid))] = centroid.astype(np.float32)
            valid_built += 1

        per_corpus_stats[c] = {
            "valid_centroids": valid_built,
            "skipped_low_pop": skipped_low_pop,
            "unmappable_rows_in_corpus": int((c_mask & (cluster == -1)).sum()),
            "control_rows": int(ctrl_mask.sum()),
        }

    return centroids, per_corpus_stats


def task2_score_hunks(data: dict, centroids: dict) -> tuple[np.ndarray, dict]:
    """Compute cosine distance from each hunk to its (corpus, cluster) centroid.

    Returns (dist, scoring_stats). Distance is NaN when no centroid exists for that hunk's cluster.
    """
    print("=== TASK 2: score per-hunk distance to cluster centroid ===", file=sys.stderr)
    hunk_n = l2_normalize(data["hunk"])
    corpus = data["corpus"]
    cluster = data["cluster"]
    is_break = data["is_break"]
    n = data["n"]

    dist = np.full(n, np.nan, dtype=np.float32)
    for i in range(n):
        key = (corpus[i], int(cluster[i]))
        if key in centroids:
            cen = centroids[key]
            sim = float(hunk_n[i] @ cen)
            dist[i] = 1.0 - sim

    stats = {}
    for c in CORPORA:
        c_mask = corpus == c
        c_break = c_mask & is_break
        c_ctrl = c_mask & (~is_break)
        scored = ~np.isnan(dist)
        stats[c] = {
            "total": int(c_mask.sum()),
            "scored": int((c_mask & scored).sum()),
            "excluded": int((c_mask & ~scored).sum()),
            "breaks_total": int(c_break.sum()),
            "breaks_scored": int((c_break & scored).sum()),
            "breaks_excluded": int((c_break & ~scored).sum()),
            "controls_total": int(c_ctrl.sum()),
            "controls_scored": int((c_ctrl & scored).sum()),
            "controls_excluded": int((c_ctrl & ~scored).sum()),
        }
    return dist, stats


def task3_calibrate(data: dict, dist: np.ndarray) -> dict:
    """For each corpus, set threshold = (1 - FP_target/100)-quantile of CONTROL distances."""
    print("=== TASK 3: calibrate per-corpus thresholds on controls ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]

    out = {}
    for c in CORPORA:
        fp_target = FP_TARGET[c]
        c_mask = corpus == c
        c_ctrl_mask = c_mask & (~is_break) & ~np.isnan(dist)
        ctrl_dists = dist[c_ctrl_mask]
        n_ctrl = len(ctrl_dists)
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
        # numpy default: linear interpolation. We need >threshold => flagged.
        threshold = float(np.quantile(ctrl_dists, q))
        # Flagged controls: dist > threshold (strict, by convention)
        flagged = int((ctrl_dists > threshold).sum())
        actual_fp_pct = 100.0 * flagged / n_ctrl
        out[c] = {
            "fp_target_pct": fp_target,
            "threshold": threshold,
            "control_count": n_ctrl,
            "actual_fp_pct": actual_fp_pct,
            "controls_flagged": flagged,
        }
    return out


def task4_residuals(data: dict, dist: np.ndarray, thresholds: dict) -> dict:
    """For faker-js residuals: report distance, threshold, percentile rank."""
    print("=== TASK 4: residual fixture catch ===", file=sys.stderr)
    fixture_id = data["fixture_id"]
    corpus = data["corpus"]
    is_break = data["is_break"]
    file_path = data["file_path"]

    fjs_threshold = thresholds["faker-js"]["threshold"]
    fjs_ctrl_mask = (corpus == "faker-js") & (~is_break) & ~np.isnan(dist)
    fjs_ctrl_dists = dist[fjs_ctrl_mask]
    fjs_ctrl_sorted = np.sort(fjs_ctrl_dists)
    n_ctrls = len(fjs_ctrl_sorted)

    residual_results = {}
    for fid in sorted(RESIDUALS):
        idx = np.where(fixture_id == fid)[0]
        if len(idx) == 0:
            residual_results[fid] = {"error": "fixture not found in dataset"}
            continue
        # Just take the first instance if multiple — fixture_ids should be unique-per-row anyway
        ridx = int(idx[0])
        d = float(dist[ridx])
        if np.isnan(d):
            residual_results[fid] = {
                "distance": None,
                "threshold": fjs_threshold,
                "crosses_threshold": False,
                "rank_top_pct": None,
                "note": "no centroid for this hunk's cluster (excluded)",
                "file_path": str(file_path[ridx]),
            }
            continue
        # rank: fraction of fjs controls strictly more anomalous than this residual
        # top X% means: only X% of controls have larger distance
        n_more_anomalous = int((fjs_ctrl_sorted > d).sum())
        rank_top_pct = 100.0 * n_more_anomalous / n_ctrls if n_ctrls else None
        # percentile = fraction of controls with dist <= d
        percentile = float((fjs_ctrl_sorted <= d).sum()) / n_ctrls if n_ctrls else None
        residual_results[fid] = {
            "distance": d,
            "threshold": fjs_threshold,
            "crosses_threshold": d > fjs_threshold if fjs_threshold is not None else False,
            "rank_top_pct": rank_top_pct,
            "percentile_among_controls": percentile,
            "file_path": str(file_path[ridx]),
        }

    n_caught = sum(
        1 for v in residual_results.values() if isinstance(v, dict) and v.get("crosses_threshold")
    )
    return {
        "residuals": residual_results,
        "n_caught": n_caught,
        "ship_gate_pass": n_caught >= 2,
        "fjs_threshold": fjs_threshold,
        "fjs_control_count": n_ctrls,
    }


def task5_recall_fp(data: dict, dist: np.ndarray, thresholds: dict) -> dict:
    """For each corpus: count fixtures crossing threshold (recall) and control FP rate."""
    print("=== TASK 5: recall + FP audit per corpus ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]

    out = {}
    for c in CORPORA:
        thr = thresholds[c]["threshold"]
        c_mask = corpus == c
        c_break_mask = c_mask & is_break & ~np.isnan(dist)
        c_ctrl_mask = c_mask & (~is_break) & ~np.isnan(dist)

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

        break_dists = dist[c_break_mask]
        ctrl_dists = dist[c_ctrl_mask]
        breaks_caught = int((break_dists > thr).sum())
        ctrls_flagged = int((ctrl_dists > thr).sum())
        n_ctrl = len(ctrl_dists)
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
            if n_breaks_total
            else 0.0,
            "stage4_recall_pct_of_scored": 100.0 * breaks_caught / n_breaks_scored
            if n_breaks_scored
            else None,
            "controls_count": n_ctrl,
            "controls_flagged": ctrls_flagged,
            "actual_fp_pct": actual_fp_pct,
            "fp_regression_vs_baseline_pp": fp_regression,
        }
    # Pre-registered no-regression gate: per-corpus FP for THIS stage alone <= baseline + 0.5pp
    gate_pass = all(
        (v["actual_fp_pct"] is None) or (v["actual_fp_pct"] <= FP_TARGET[c] + 0.5)
        for c, v in out.items()
    )
    return {"per_corpus": out, "no_regression_gate_pass": gate_pass}


def task6_fjs_diagnostic(data: dict, dist: np.ndarray, thresholds: dict) -> dict:
    """For faker-js: dump top-20 controls by embedding_distance (closest to threshold)."""
    print("=== TASK 6: faker-js top control distances diagnostic ===", file=sys.stderr)
    is_break = data["is_break"]
    corpus = data["corpus"]
    file_path = data["file_path"]
    cluster = data["cluster"]

    fjs_ctrl_mask = (corpus == "faker-js") & (~is_break) & ~np.isnan(dist)
    fjs_ctrl_idx = np.where(fjs_ctrl_mask)[0]
    fjs_ctrl_dists = dist[fjs_ctrl_idx]
    order = np.argsort(-fjs_ctrl_dists)[:20]
    top20 = []
    thr = thresholds["faker-js"]["threshold"]
    for rank, off in enumerate(order, 1):
        i = int(fjs_ctrl_idx[off])
        top20.append(
            {
                "rank": rank,
                "file_path": str(file_path[i]),
                "cluster_id": int(cluster[i]),
                "distance": float(dist[i]),
                "above_threshold": bool(dist[i] > thr) if thr is not None else False,
            }
        )
    return {"top20_fjs_controls_by_distance": top20, "fjs_threshold": thr}


def task7_verdict(t4: dict, t5: dict) -> dict:
    """Apply pre-registered verdict logic."""
    fjs_catches = t4["n_caught"]
    no_regression = t5["no_regression_gate_pass"]

    # Identify any per-corpus regressions
    regressions = []
    for c, v in t5["per_corpus"].items():
        if v["actual_fp_pct"] is not None and v["actual_fp_pct"] > FP_TARGET[c] + 0.5:
            regressions.append(
                {
                    "corpus": c,
                    "actual_fp_pct": v["actual_fp_pct"],
                    "baseline_fp_pct": FP_TARGET[c],
                    "regression_pp": v["fp_regression_vs_baseline_pp"],
                }
            )

    if fjs_catches >= 2 and no_regression:
        verdict = "SHIP"
    elif fjs_catches >= 1 and not no_regression:
        verdict = "PARTIAL"
    elif fjs_catches >= 1 and no_regression:
        # ≥1 catch but not 2 — partial under task spec (orchestrator decision)
        verdict = "PARTIAL"
    elif fjs_catches == 0:
        verdict = "CLOSE NEGATIVE"
    else:
        verdict = "PARTIAL"

    return {
        "verdict": verdict,
        "faker_js_residual_catches": fjs_catches,
        "ship_gate_pass": fjs_catches >= 2,
        "no_regression_gate_pass": no_regression,
        "regressions": regressions,
    }


def main() -> None:
    data = load_all()
    print(
        f"Loaded {data['n']} rows; {data['is_break'].sum()} breaks, "
        f"{(~data['is_break']).sum()} controls",
        file=sys.stderr,
    )

    centroids, t1_stats = task1_build_centroids(data)
    dist, t2_stats = task2_score_hunks(data, centroids)
    t3 = task3_calibrate(data, dist)
    t4 = task4_residuals(data, dist, t3)
    t5 = task5_recall_fp(data, dist, t3)
    t6 = task6_fjs_diagnostic(data, dist, t3)
    t7 = task7_verdict(t4, t5)

    # Save centroids for downstream use.
    centroids_path = FEATURE_DIR / "centroids_phase6.4.joblib"
    joblib.dump(
        {
            "centroids": centroids,
            "min_cluster_controls": MIN_CLUSTER_CONTROLS,
            "fp_target": FP_TARGET,
            "thresholds": {c: t3[c]["threshold"] for c in CORPORA},
            "method": "L2-normed mean of CONTROL hunk_embedding per (corpus, cluster_id); "
            "score = 1 - cosine(hunk_n, centroid_n)",
        },
        centroids_path,
    )

    out = {
        "task1_centroid_construction": t1_stats,
        "task2_scoring_stats": t2_stats,
        "task3_thresholds": t3,
        "task4_residuals": t4,
        "task5_recall_fp": t5,
        "task6_fjs_top20_controls": t6,
        "task7_verdict": t7,
        "centroids_saved_to": str(centroids_path),
        "n_total_centroids": len(centroids),
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
