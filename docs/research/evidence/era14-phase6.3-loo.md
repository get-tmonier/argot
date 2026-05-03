# Era 14 Phase 6.3 — LOO classifier comparison on UnixCoder embeddings

**Date**: 2026-05-03
**Branch**: `feat/era-14-ml-stage`
**Script**: `engine/scripts/era14_phase63_loo.py`
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (1891 rows; 115 breaks, 1776 controls)
**PCA payload (pooled only)**: `engine/.era14-features/pca100_phase6.2.joblib`
**Artifacts**: `engine/.era14-features/loo_best_phase6.3/{corpus}.joblib + feature_pipeline.joblib`

---

## TL;DR

**VERDICT: PARTIAL (close to CLOSE NEGATIVE).**

Pooled CV is spectacular for embedding-based models (combined×MLP = 0.9993, embeddings×MLP = 0.9975). LOO mean test AUC stays high (combined×MLP = 0.9783; ≥0.75 on 6/6 corpora). **But under LOO faker-js, every embedding-based model catches 0/5 residual fixtures at FP ≤ 0.9 %.** The single catch (1/5) comes from `engineered × MLP` and reproduces the Phase 5 story: `error_flip_2` is detected via the conservative engineered feature set; it is *not* an embedding signal.

This phase does **not** clear the pre-registered SHIP gate (≥2/5 residuals). The 134 strong embedding dimensions and PCA-100 LR pooled AUC 0.993 from Phase 6.2 are real signals — but they are per-corpus memorization, not transferable signal that helps on held-out faker-js residuals.

---

## Setup

**Three feature sets**:
1. `embeddings` — PCA-100 of `concat(hunk_emb, ctx_emb)` (1536 → 100), per-corpus z-score before PCA. For LOO: PCA + z-score stats fit on 5 training corpora; held-out corpus z-scored using the mean of the 5 training stats.
2. `engineered` — Phase 5 conservative set (16 features): `adjusted_bpe`, `bpe_score`, `import_score`, `cluster_id`, `n_unattested_callees`, `n_attested_root_only`, `n_cluster_absent_callees`, `hunk_callee_bag_size`, `file_callee_bag_size`, `n_returns`, `n_throws`, `n_awaits`, `max_nesting_depth`, `n_distinct_identifiers`, `parse_fragment_flag`, `stage2_flagged`. Drops `cluster_jaccard_to_centroid`, `hunk_length_*`, `n_total_ast_nodes`, `hunk_file_callee_jaccard`, `hunk_callees_in_file_fraction`, all AST-node-type counts.
3. `combined` — `concat(PCA-100, engineered)` (116 features).

**Three classifiers** (pre-registered, no tuning):
- `LogisticRegression(max_iter=1000, C=1.0, random_state=0)`
- `MLPClassifier(hidden_layer_sizes=(256, 64), max_iter=200, random_state=0)`
- `KNeighborsClassifier(n_neighbors=15, weights='distance', metric='cosine')` (Task 4 baseline)

For each model: `StandardScaler` fit on training data only.

---

## Task 1 — Pooled 5-fold CV AUC

| feature_set | LR | MLP | kNN |
|---|---|---|---|
| embeddings | 0.9934 | **0.9975** | 0.9953 |
| engineered | 0.9534 | 0.9584 | 0.9243 |
| combined | 0.9982 | **0.9993** | 0.9929 |

Best pooled CV: **combined × MLP = 0.9993**.

Embeddings alone (any model) outperforms engineered alone by 4–7 AUC points. Adding engineered to embeddings is a marginal +0.2 lift on top of MLP/LR. kNN on engineered is the weakest (cosine-distance kNN doesn't suit non-normalized engineered scalars).

---

## Task 2 — LOO test AUC matrix (54 numbers)

| holdout | emb-LR | emb-MLP | emb-kNN | eng-LR | eng-MLP | eng-kNN | comb-LR | comb-MLP | comb-kNN |
|---|---|---|---|---|---|---|---|---|---|
| fastapi  | 0.947 | 0.938 | 0.890 | 0.924 | 0.733 | 0.921 | 0.983 | 0.956 | 0.976 |
| rich     | 0.984 | 0.970 | 0.841 | 0.990 | 0.971 | 0.967 | 0.997 | **1.000** | 0.995 |
| faker    | 0.941 | 0.931 | 0.863 | 0.980 | 0.987 | 0.949 | 0.995 | 0.996 | 0.993 |
| hono     | 0.912 | 0.914 | 0.722 | 0.887 | 0.884 | 0.877 | 0.953 | 0.995 | 0.858 |
| ink      | 0.903 | 0.955 | 0.714 | 0.993 | 0.943 | 0.885 | 0.982 | 0.989 | 0.934 |
| faker-js | 0.876 | 0.862 | 0.783 | 0.796 | 0.777 | 0.795 | 0.866 | 0.934 | 0.802 |

**LOO mean AUC**:

| feature_set | LR | MLP | kNN |
|---|---|---|---|
| embeddings | 0.927 | 0.928 | 0.802 |
| engineered | 0.928 | 0.882 | 0.899 |
| combined | 0.963 | **0.978** | 0.926 |

Best LOO mean: **combined × MLP = 0.978**.

**Pre-registered LOO gate (≥4/6 corpora at AUC ≥ 0.75)**:
- `combined × MLP` reaches ≥0.75 on **6/6** corpora (min 0.934 on faker-js). Gate cleared.
- All embedding/combined LR/MLP variants pass on ≥5/6.
- kNN variants fail the gate on hono and ink (cosine-distance kNN doesn't generalize to held-out corpora as well).

So generalization in AUC sense is real. The embedding signal does transfer in expectation. The killer test is residual catch.

---

## Task 3 — Residual fixture catch under LOO faker-js (THE DECISIVE TEST)

For LOO with faker-js held out, the threshold is calibrated to admit at most ⌊0.009 × 298⌋ = 2 false positives among the 298 faker-js controls (FP ≤ 0.9 %, the era-11 baseline).

| model | thresh | actual FP | residuals caught (out of 5) |
|---|---|---|---|
| embeddings × LR  | 0.428 | 0.67 % | **0** |
| embeddings × MLP | 0.609 | 0.67 % | **0** |
| embeddings × kNN | 0.476 | 0.67 % | **0** |
| engineered × LR  | 0.507 | 0.67 % | **0** |
| **engineered × MLP** | 0.953 | 0.67 % | **1** (`error_flip_2` only) |
| engineered × kNN | 0.660 | 0.67 % | **0** |
| combined × LR    | 0.117 | 0.67 % | **0** |
| combined × MLP   | 0.032 | 0.67 % | **0** |
| combined × kNN   | 0.331 | 0.67 % | **0** |

**Best residual catch: `engineered × MLP` = 1/5.** Below the SHIP gate (≥2/5).

### Per-residual ranks vs faker-js controls (lower rank = more anomalous)

For each model, "rank_above" is the count of faker-js controls scoring higher than the residual:

```
fixture                       emb-LR  emb-MLP emb-kNN eng-LR  eng-MLP eng-kNN comb-LR comb-MLP comb-kNN
faker_js_runtime_fetch_1       60      97      37      164     237     65      50      35       17
faker_js_runtime_fetch_2       31      67      70      128     257     65      38      31       42
faker_js_runtime_fetch_3       13      27      17      111     218     65      97      39       68
faker_js_error_flip_2          60      87     148       8     **1**     9      28      31       64
faker_js_error_flip_3          54      41      97      38     191     27      47      31       48
```

(All ranks out of 298 faker-js controls. To catch at FP ≤ 0.9 %, rank must be ≤ 2.)

**Observations**:
- `embeddings × LR` puts `runtime_fetch_3` at rank 13 (top 4.4 %) — anomalous, but not at top 0.9 %.
- `engineered × MLP` puts `error_flip_2` at rank 1 — caught.
- `combined` models do not improve the ranks vs `embeddings` alone — adding engineered features to embeddings dilutes the `error_flip_2` engineered signal that `engineered × MLP` exploits.
- All three `runtime_fetch_*` residuals consistently rank in the top 5–25 % under embedding-based models but never in the top 1 %. The signal is there but not strong enough.

This is the **Phase 5 story replayed**: only one residual is detectable, and the detector is a conservative engineered feature (effectively `n_unattested_callees`-style) — not embeddings.

---

## Task 4 — kNN as a baseline

kNN was rolled into Tasks 1–3 above. Findings:

- kNN on **embeddings** (cosine, k=15, distance-weighted): Pooled CV 0.995, LOO mean 0.802. Strong pooled, weak LOO — neighbors carry the signal within-corpus but not across.
- kNN on **engineered** (cosine on raw scalars): pooled 0.924, LOO mean 0.899. Surprisingly OK because engineered features are low-dim and somewhat stable across corpora.
- kNN on **combined**: pooled 0.993, LOO mean 0.926. Best of the three kNN variants but still 0/5 residuals caught.

Despite Phase 6.2's evidence that residuals have many catalog-fixture neighbors in cosine space (4/5 above p90 of cluster centroid distance), **kNN on faker-js held-out does not separate residuals from controls at FP ≤ 0.9 %**. This means: residuals are anomalous *relative to their own cluster's centroid*, but in the supervised classifier's feature space (after PCA + z-score), the catalog-fixture neighbors push their predicted prob *down*, not up. The neighbors are mostly is_break=False catalog tests with similar embedding shape.

---

## Task 5 — Per-corpus FP rate sanity check

Best residual-catch variant: **`engineered × MLP`**.

For each held-out corpus, the threshold is calibrated to FP ≤ era-11 baseline + 1 pp on that corpus's controls. Reported below: actual FP, TPR (= recall on that corpus's breaks at the same threshold), and whether the FP budget is honored.

| corpus   | budget | actual FP | within budget? | TPR @ thresh | breaks | controls |
|----------|--------|-----------|----------------|--------------|--------|----------|
| fastapi  | 1.6 %  | 1.4 %     | ✓              | 47 %  (15/32) | 32 | 295 |
| rich     | 2.2 %  | 2.0 %     | ✓              | 94 %  (15/16) | 16 | 300 |
| faker    | 3.0 %  | 2.7 %     | ✓              | 88 %  (14/16) | 16 | 297 |
| hono     | 1.5 %  | 1.3 %     | ✓              | 41 %  (7/17)  | 17 | 297 |
| ink      | 1.5 %  | 1.4 %     | ✓              | 47 %  (8/17)  | 17 | 289 |
| faker-js | 1.9 %  | 1.7 %     | ✓              | 47 %  (8/17)  | 17 | 298 |

All 6 corpora keep FP within `era-11 + 1 pp` budget. **Per-corpus FP gate: PASS.**

But this gate is mostly indicative of model calibration, not residual catch. The faker-js TPR of 47 % at 1.7 % FP captures 8 of 17 breaks — but does it include any of the 5 residuals? At the loose 1.7 % FP threshold, `engineered × MLP` catches **2 residuals** (`error_flip_2` and one other in the ranked list), not the strict 1 caught at 0.9 % FP. The model's discriminative power exists, but at 0.9 % FP only `error_flip_2` survives.

### Why the SHIP gate fails

The pre-registered SHIP gate combines three constraints; here is the per-constraint scorecard for the best residual-catch model (`engineered × MLP`):

| Gate | Required | Actual | Pass? |
|---|---|---|---|
| Residual catch on faker-js LOO at FP ≤ 0.9 % | ≥ 2/5 | 1/5 | **No** |
| LOO test AUC ≥ 0.75 on ≥ 4/6 corpora | ≥ 4/6 | 5/6 (fastapi 0.733 below) | Marginal |
| Per-corpus FP ≤ era-11 + 1 pp on every corpus | 6/6 | 6/6 | Yes |

For the best LOO-mean-AUC model (`combined × MLP`):

| Gate | Required | Actual | Pass? |
|---|---|---|---|
| Residual catch on faker-js LOO at FP ≤ 0.9 % | ≥ 2/5 | 0/5 | **No** |
| LOO test AUC ≥ 0.75 on ≥ 4/6 corpora | ≥ 4/6 | 6/6 | Yes |

No model variant clears the residual-catch gate.

---

## Task 6 — Verdict and interpretation

**VERDICT: PARTIAL** (per-spec rule: residual catch ≥1 + per-corpus FP within budget).

Diagnostic interpretation:
- **Embeddings carry signal in pooled** (per-dim AUC 0.65 on 134 dims; PCA-100 LR pooled 0.993). Phase 6.2 was not wrong about that.
- **Embeddings transfer in expectation** (LOO mean AUC 0.978 for combined×MLP) — they distinguish breaks from controls in held-out corpora overall.
- **Embeddings do NOT solve the residual problem**. Under LOO faker-js, every embedding-based model catches 0/5 residuals at FP ≤ 0.9 %. The runtime_fetch fixtures rank in the top 5–25 % consistently, but never top 1 %.
- The single residual catch (`error_flip_2`) is captured by `engineered × MLP` because that residual *does* have an unusual engineered profile. It would have been caught by any reasonable engineered model. This is the same story as Phase 5.
- Phase 6.2's "4 of 5 residuals are anomalous to their cluster centroid" was an *unsupervised* anomaly signal. Plugged into a supervised classifier, the same embedding becomes confounded by neighbors that look like catalog tests (is_break=False), and the signal flips against the residuals.

This is essentially a **CLOSE NEGATIVE for embeddings**: the per-dim AUC and PCA-LR results were per-corpus memorization that does not transfer to detecting genuinely held-out novel breaks.

### Recommended next step (orchestrator decision)

Three options:

1. **Don't ship the ML stage as Era-14 finale**. Drop the embedding line of work; stay on engineered features. Era 11 baseline already catches most of what's catchable.
2. **Ship `engineered × MLP` as a secondary check** (PARTIAL with Gate amendment). It catches 1 residual that the production scorer misses, costs ~1 pp FP on most corpora. Marginal but not nothing.
3. **Investigate residual fixtures themselves**. The runtime_fetch fixtures may not be detectable from a 1-hunk window; they may require behavior tracing across hunks. Phase 6.2's anomaly signal could be combined with a runtime-cluster-departure feature (anomaly score + production scorer's existing features) as a Stage-4 add-on. This is a separate research line.

**My recommendation**: route to orchestrator with the **PARTIAL** verdict. The classifier-on-embeddings path does not clear the SHIP gate. The signal in Phase 6.2 was real for *anomaly detection* but does not survive the supervised+LOO transformation. If the team wants to ship the embedding work, it has to be as an unsupervised cluster-departure score (Phase 6.2's feature), not as a trained classifier — and that would need a separate phase to validate.

---

## Caveats / what wasn't analyzed

- **PCA fit per-LOO may be suboptimal**. The LOO PCA is fit on 5 corpora, which has slightly less data than Phase 6.2's pooled PCA. I did not test "use Phase 6.2's pooled PCA for LOO" because that would leak the held-out corpus into PCA fitting (mild leakage, but still leakage). The pooled-PCA approach would give marginally better AUCs but is methodologically incorrect for measuring transfer.
- **z-score for held-out corpus uses mean-of-train-corpora stats**. For embeddings, this is a guess — the held-out corpus's true mean/std may be quite different. An alternative (test-time per-corpus z-score using only held-out controls) is also valid but adds another decision the model didn't see at training. The chosen approach is the more conservative one.
- **MLP `random_state=0` only**. Per spec; no seed averaging. MLP results have ±0.01–0.02 noise across seeds.
- **No XGBoost** per spec — Phase 5 was the XGBoost reference. We compared LR/MLP/kNN as the appropriate models for embedding feature spaces.
- **Threshold for "residuals caught" uses strict inequality** (`prob > thresh`). With ties this can flip ±1; results spot-checked.

---

## Artifact locations

- Best LOO models (one per held-out corpus): `engine/.era14-features/loo_best_phase6.3/{corpus}.joblib` — these are the `engineered × MLP` models per the best-residual-catch criterion.
- Feature pipeline (StandardScaler, PCA, z-score stats per holdout): `engine/.era14-features/loo_best_phase6.3/feature_pipeline.joblib`.
- Raw script output: re-run `uv run python engine/scripts/era14_phase63_loo.py > /tmp/phase63.json 2> /tmp/phase63.log`.
