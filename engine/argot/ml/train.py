"""Era-14 Phase 3 — pooled XGBoost training over engineered features.

Reads the per-hunk feature JSONLs emitted by ``argot.ml.features`` (one file
per corpus, found under ``engine/.era14-features/``), engineers a small set of
numeric features, trains two XGBoost classifiers (Set A: full pooled, Set B:
Stage-4 operating regime — rows where the production scorer returned no
flag), and reports cross-validated AUC + per-corpus AUC on held-out folds.

This is a research/training entry point, not part of the production scoring
pipeline. It is invoked manually via ``uv run python -m argot.ml.train``.
"""

from __future__ import annotations

import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import joblib  # type: ignore[import-untyped]
import numpy as np
import xgboost as xgb  # type: ignore[import-untyped]
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import StratifiedKFold, cross_val_predict, cross_val_score

# ---------------------------------------------------------------------------
# Paths + corpora
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parents[3]
FEATURES_DIR = REPO_ROOT / "engine" / ".era14-features"
ARTIFACTS_DIR = REPO_ROOT / ".era14-features"
CORPORA = ("fastapi", "rich", "faker", "hono", "ink", "faker-js")

# Pre-registered XGBoost hyperparameters (Phase 3 spec — DO NOT TUNE).
XGB_PARAMS: dict[str, Any] = {
    "n_estimators": 100,
    "max_depth": 4,
    "learning_rate": 0.1,
    "objective": "binary:logistic",
    "random_state": 0,
    "eval_metric": "auc",
    "tree_method": "hist",
}

# Phase-2 evidence: drop redundant features.
DROPPED_FEATURES: tuple[str, ...] = (
    "n_distinct_callees",  # bit-for-bit identical to hunk_callee_bag_size
    "stage1_flagged",  # r=0.963 with import_score (binarisation)
)

# Top-5 AST node types — picked by pooled frequency across the training set.
# Computed once below; this constant is the *output* recorded in the memo.
AST_TOP_N = 5


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Row:
    corpus: str
    is_break: bool
    fixture_id: str | None
    scorer_reason: str | None
    features: dict[str, Any]


def _load_corpus(corpus: str) -> list[Row]:
    path = FEATURES_DIR / f"{corpus}.jsonl"
    rows: list[Row] = []
    with path.open() as f:
        for line in f:
            d = json.loads(line)
            rows.append(
                Row(
                    corpus=d["corpus"],
                    is_break=bool(d["is_break"]),
                    fixture_id=d.get("fixture_id"),
                    scorer_reason=d["features"].get("scorer_reason"),
                    features=d["features"],
                )
            )
    return rows


def load_all() -> list[Row]:
    out: list[Row] = []
    for c in CORPORA:
        out.extend(_load_corpus(c))
    return out


# ---------------------------------------------------------------------------
# Feature engineering
# ---------------------------------------------------------------------------

# Numeric base features kept after dedup. Booleans coerced to 0/1.
BASE_NUMERIC = (
    "adjusted_bpe",
    "bpe_score",
    "import_score",
    "cluster_id",
    "cluster_jaccard_to_centroid",
    "n_unattested_callees",
    "n_attested_root_only",
    "n_cluster_absent_callees",
    "hunk_callee_bag_size",
    "file_callee_bag_size",
    "hunk_callees_in_file_fraction",
    "hunk_file_callee_jaccard",
    "n_returns",
    "n_throws",
    "n_awaits",
    "max_nesting_depth",
    "n_distinct_identifiers",
    "parse_fragment_flag",  # bool
    "stage2_flagged",  # bool
)


def _coerce(v: Any) -> float:
    if isinstance(v, bool):
        return 1.0 if v else 0.0
    if v is None:
        return 0.0
    return float(v)


def pick_top_ast_types(rows: list[Row], n: int = AST_TOP_N) -> list[str]:
    """Return the n most frequent AST node types pooled across all rows."""
    counter: Counter[str] = Counter()
    for r in rows:
        for k, v in r.features.get("ast_node_type_counts", {}).items():
            counter[k] += int(v)
    return [k for k, _ in counter.most_common(n)]


def build_feature_matrix(
    rows: list[Row], ast_top: list[str]
) -> tuple[np.ndarray, np.ndarray, list[str], list[str]]:
    """Build (X, y, feature_names, corpora) for the given rows."""
    feat_names = list(BASE_NUMERIC) + ["n_total_ast_nodes"] + [f"ast_{t}" for t in ast_top]
    X = np.zeros((len(rows), len(feat_names)), dtype=np.float64)
    y = np.zeros(len(rows), dtype=np.int64)
    corpora = []
    for i, r in enumerate(rows):
        for j, f in enumerate(BASE_NUMERIC):
            X[i, j] = _coerce(r.features.get(f))
        ast_counts = r.features.get("ast_node_type_counts", {}) or {}
        X[i, len(BASE_NUMERIC)] = float(sum(ast_counts.values()))
        for k, t in enumerate(ast_top):
            X[i, len(BASE_NUMERIC) + 1 + k] = float(ast_counts.get(t, 0))
        y[i] = 1 if r.is_break else 0
        corpora.append(r.corpus)
    return X, y, feat_names, corpora


# ---------------------------------------------------------------------------
# Set A / Set B selection
# ---------------------------------------------------------------------------


def split_set_b(rows: list[Row]) -> list[Row]:
    """Stage-4 operating regime: rows where stages 1-3 returned no flag.

    Concretely: ``scorer_reason`` is ``"none"`` or ``None``.
    """
    return [
        r
        for r in rows
        if r.scorer_reason is None or r.scorer_reason == "none"
    ]


# ---------------------------------------------------------------------------
# Training + evaluation
# ---------------------------------------------------------------------------


def _fit_full(X: np.ndarray, y: np.ndarray) -> xgb.XGBClassifier:
    model = xgb.XGBClassifier(**XGB_PARAMS)
    model.fit(X, y)
    return model


def cv_auc(X: np.ndarray, y: np.ndarray, *, n_splits: int = 5) -> tuple[float, float, np.ndarray]:
    """Return (mean_auc, std_auc, per_fold) using stratified k-fold CV."""
    cv = StratifiedKFold(n_splits=n_splits, shuffle=True, random_state=0)
    model = xgb.XGBClassifier(**XGB_PARAMS)
    scores = cross_val_score(model, X, y, scoring="roc_auc", cv=cv, n_jobs=1)
    return float(scores.mean()), float(scores.std()), scores


def cv_predict_proba(
    X: np.ndarray, y: np.ndarray, *, n_splits: int = 5
) -> np.ndarray:
    """Out-of-fold predicted probabilities."""
    cv = StratifiedKFold(n_splits=n_splits, shuffle=True, random_state=0)
    model = xgb.XGBClassifier(**XGB_PARAMS)
    return cross_val_predict(model, X, y, cv=cv, method="predict_proba", n_jobs=1)[:, 1]


def per_corpus_oof_auc(
    X: np.ndarray, y: np.ndarray, corpora: list[str]
) -> dict[str, tuple[float, int, int]]:
    """Per-corpus AUC computed from out-of-fold predictions on the pooled set."""
    proba = cv_predict_proba(X, y)
    out: dict[str, tuple[float, int, int]] = {}
    arr_corp = np.array(corpora)
    for c in CORPORA:
        mask = arr_corp == c
        n = int(mask.sum())
        n_pos = int(y[mask].sum())
        if n == 0 or n_pos == 0 or n_pos == n:
            out[c] = (float("nan"), n, n_pos)
            continue
        out[c] = (float(roc_auc_score(y[mask], proba[mask])), n, n_pos)
    return out


def feature_importance(model: xgb.XGBClassifier, feat_names: list[str]) -> list[tuple[str, float]]:
    booster = model.get_booster()
    raw = booster.get_score(importance_type="gain")
    # Map ``f0``/``f1``/... back to names.
    name_score: dict[str, float] = {feat_names[int(k[1:])]: float(v) for k, v in raw.items()}
    return sorted(name_score.items(), key=lambda x: -x[1])


# ---------------------------------------------------------------------------
# Driver — invoked via ``uv run python -m argot.ml.train``
# ---------------------------------------------------------------------------


def main() -> None:  # noqa: C901 — driver is intentionally straight-line
    ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)

    # ----- load --------------------------------------------------------
    rows_a = load_all()
    rows_b = split_set_b(rows_a)
    n_a = len(rows_a)
    n_a_pos = sum(1 for r in rows_a if r.is_break)
    n_b = len(rows_b)
    n_b_pos = sum(1 for r in rows_b if r.is_break)
    fakerjs_residuals = [
        r for r in rows_b if r.corpus == "faker-js" and r.is_break
    ]
    print(f"Set A: total={n_a}  breaks={n_a_pos}  controls={n_a - n_a_pos}")
    print(
        f"Set B: total={n_b}  breaks={n_b_pos}  controls={n_b - n_b_pos}  "
        f"(faker-js residuals in B: {len(fakerjs_residuals)})"
    )

    # ----- AST top-5 (pooled across Set A) -----------------------------
    ast_top = pick_top_ast_types(rows_a, n=AST_TOP_N)
    print(f"Top-5 AST node types (pooled): {ast_top}")

    # ----- build matrices ---------------------------------------------
    X_a, y_a, feat_names, corp_a = build_feature_matrix(rows_a, ast_top)
    X_b, y_b, _, corp_b = build_feature_matrix(rows_b, ast_top)
    print(f"Feature count: {len(feat_names)}")
    print("Features (in order):")
    for i, f in enumerate(feat_names):
        print(f"  [{i:2d}] {f}")

    # ----- save feature manifest --------------------------------------
    manifest = {
        "features": feat_names,
        "dropped": list(DROPPED_FEATURES),
        "ast_top_n": AST_TOP_N,
        "ast_top_types": ast_top,
        "xgb_params": XGB_PARAMS,
    }
    with (ARTIFACTS_DIR / "feature_list.json").open("w") as f:
        json.dump(manifest, f, indent=2)

    # ----- 5-fold CV AUC ----------------------------------------------
    auc_a, std_a, folds_a = cv_auc(X_a, y_a)
    auc_b, std_b, folds_b = cv_auc(X_b, y_b)
    print(f"\nSet A 5-fold CV AUC: mean={auc_a:.4f}  std={std_a:.4f}  folds={folds_a.tolist()}")
    print(f"Set B 5-fold CV AUC: mean={auc_b:.4f}  std={std_b:.4f}  folds={folds_b.tolist()}")

    # ----- kill-switch verdict ----------------------------------------
    pass_a = auc_a > 0.85
    pass_b = auc_b > 0.70
    if pass_a and pass_b:
        verdict = "PASS"
    elif pass_a and not pass_b:
        verdict = "WEAK"
    else:
        verdict = "FAIL"
    print(f"\nKILL-SWITCH verdict: {verdict}  (Set A>0.85: {pass_a}  Set B>0.70: {pass_b})")

    # ----- per-corpus OOF AUC -----------------------------------------
    print("\nPer-corpus OOF AUC (Set A):")
    for c, (auc, n, npos) in per_corpus_oof_auc(X_a, y_a, corp_a).items():
        print(f"  {c:10s}  n={n:4d}  pos={npos:3d}  auc={auc:.4f}")
    print("\nPer-corpus OOF AUC (Set B):")
    for c, (auc, n, npos) in per_corpus_oof_auc(X_b, y_b, corp_b).items():
        print(f"  {c:10s}  n={n:4d}  pos={npos:3d}  auc={auc:.4f}")

    # ----- fit full models + save -------------------------------------
    model_a = _fit_full(X_a, y_a)
    model_b = _fit_full(X_b, y_b)
    joblib.dump(
        {"model": model_a, "feature_names": feat_names, "ast_top": ast_top},
        ARTIFACTS_DIR / "pooled_setA.joblib",
    )
    joblib.dump(
        {"model": model_b, "feature_names": feat_names, "ast_top": ast_top},
        ARTIFACTS_DIR / "pooled_setB.joblib",
    )

    # ----- top-10 feature importance ----------------------------------
    print("\nTop-10 features by gain (Set A):")
    for f, g in feature_importance(model_a, feat_names)[:10]:
        print(f"  {f:35s}  {g:10.3f}")
    print("\nTop-10 features by gain (Set B):")
    for f, g in feature_importance(model_b, feat_names)[:10]:
        print(f"  {f:35s}  {g:10.3f}")

    # ----- residual faker-js predicted probabilities ------------------
    proba_b = cv_predict_proba(X_b, y_b)
    arr_corp_b = np.array(corp_b)
    fakerjs_mask = arr_corp_b == "faker-js"
    fakerjs_pos = fakerjs_mask & (y_b == 1)
    fakerjs_neg = fakerjs_mask & (y_b == 0)
    fakerjs_neg_proba = proba_b[fakerjs_neg]
    n_neg = int(fakerjs_neg.sum())

    print("\nResidual faker-js fixtures — OOF predicted prob (sorted desc):")
    residual_rows = [
        (r.fixture_id, p) for r, p, m in zip(rows_b, proba_b, fakerjs_pos, strict=False) if m
    ]
    residual_rows.sort(key=lambda x: -x[1])
    for fid, p in residual_rows:
        # rank vs faker-js controls (0 = highest scoring control above this row)
        n_above = int((fakerjs_neg_proba > p).sum())
        # threshold = p; FP rate at this threshold (controls strictly >= p)
        fp_at_p = float((fakerjs_neg_proba >= p).sum()) / max(1, n_neg)
        print(
            f"  {fid:35s}  proba={p:.4f}  controls_above={n_above}/{n_neg}  "
            f"fp_rate@thresh={fp_at_p*100:.2f}%"
        )

    # ----- threshold sweep: catch >=2 residuals at faker-js FP <= 0.9% --
    target_fp = 0.009
    # Pick the k-th highest control prob — anything above it has FP rate <= k/n_neg.
    sorted_neg = np.sort(fakerjs_neg_proba)[::-1]
    # Smallest count of controls allowed above threshold:
    max_controls_above = int(np.floor(target_fp * n_neg))
    if max_controls_above < len(sorted_neg):
        thresh = float(sorted_neg[max_controls_above]) + 1e-12
    else:
        thresh = float(sorted_neg[-1]) + 1e-12
    catches = sum(1 for _, p in residual_rows if p > thresh)
    fp_at_thresh = float((fakerjs_neg_proba > thresh).sum()) / max(1, n_neg)
    print(
        f"\nAt threshold {thresh:.4f} (faker-js FP <= {target_fp*100:.1f}%, actual "
        f"{fp_at_thresh*100:.2f}%): catches {catches}/{len(residual_rows)} residuals."
    )


if __name__ == "__main__":
    main()
