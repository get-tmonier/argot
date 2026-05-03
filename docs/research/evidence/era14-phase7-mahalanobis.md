# Era 14 Phase 7 — PCA-whitened Mahalanobis anomaly scoring on UnixCoder embeddings

**Date**: 2026-05-03
**Branch**: `feat/era-14-ml-stage`
**Script**: `engine/scripts/era14_phase7_mahalanobis.py`
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (1891 rows; 115 breaks, 1776 controls)
**Persisted artifacts**:
- Mahalanobis dict (PCA models + μ + Σ_reg_inv + thresholds): `engine/.era14-features/phase7_mahalanobis.joblib`
- Raw results JSON: `/tmp/era14_phase7_results.json`

---

## TL;DR

**VERDICT: CLOSE NEGATIVE.**

The pre-registered SHIP gate (≥ 2/5 faker-js residuals catch AND every corpus FP ≤ baseline + 0.5 pp) **mechanically clears** under Phase 7 — 4 of 5 residuals catch, all 6 corpora pass the FP regression gate. On its face, this looks like a strong SHIP.

**It is not a real result.** A leave-one-out sanity diagnostic added in this phase shows that **37 of 38 cluster-routed (corpus, cluster_id) Mahalanobis models are dominated by rank-deficiency**: in 30 of 38 clusters, n_ctrl < PCA_DIM (64), so the sample covariance Σ has rank ≤ n_ctrl − 1 and the regularized inverse is dominated by 1/λ = 100 in the (64 − rank(Σ))-dim null-space. **Held-out controls under LOO score d² values 5×–2500× larger than their in-sample d²**, comparable to or larger than the in-sample d² of the breaks the model claims to "catch". The 4 residual catches are an artifact of breaks not being in the cluster's training set for Σ — the same inflation hits any held-out point.

Concretely, on faker-js cluster 6 (which catches `runtime_fetch_2`):
- n_ctrl = 13 (against 64 dimensions → rank-deficient)
- in-sample max control d² = 11.07
- LOO max control d² = **18 107.27** (1636× ratio)
- the "caught" `runtime_fetch_2` break d² = 3 941.50

The break's d² is not just below the LOO ceiling — it is below several individual LOO control scores. In a properly-validated unbiased Mahalanobis distribution, this break would not be distinguishable from a held-out control.

This closes era 14. The embedding-anomaly axis cannot deliver the residual catch the era was designed to test, under any of the four methodologies tried (Phase 6.2 probe / Phase 6.3 supervised / Phase 6.4 cosine / Phase 7 Mahalanobis). Phase 7 is the cleanest negative because the failure mode is statistical (rank-deficiency in Σ), not ad-hoc tuning. The mechanism is:

> Sample covariance estimated on k controls in d-dim space has rank ≤ k − 1. After Tikhonov λI regularization, the inverse acts as 1/λ along the (d − rank) null-space directions. Any point outside the linear span of the training controls — including any held-out control, not just a break — gets an inflated Mahalanobis d². The "break detection" signal is a function of degrees-of-freedom, not embedding-anomaly.

The naive task5/task6 numbers below should be read with this caveat in mind. Task 6b is the decisive evidence.

---

## Setup recap

| Corpus | Rows | Breaks | Controls | FP target (era-11 baseline) |
|---|---:|---:|---:|---:|
| fastapi  | 327 | 32 | 295 | 0.6 % |
| rich     | 316 | 16 | 300 | 1.2 % |
| faker    | 313 | 16 | 297 | 2.0 % |
| hono     | 314 | 17 | 297 | 0.5 % |
| ink      | 306 | 17 | 289 | 0.5 % |
| faker-js | 315 | 17 | 298 | 0.9 % |
| **total** | **1891** | **115** | **1776** | — |

**Method (per pre-reg)**:

- For each corpus: fit `sklearn.decomposition.PCA(n_components=64, whiten=True)` on the **CONCATENATED** `[hunk_embedding, context_embedding]` (1536-d) of CONTROL rows ONLY (`is_break == False`). Project ALL rows in the corpus into that corpus's PCA-64 whitened space.
- For each `(corpus, cluster_id)` with `cluster_id != -1` and ≥ MIN_CLUSTER_CONTROLS = 5 controls: μ_c = mean of those controls in PCA-64 space; Σ_c = sample covariance; Σ_reg = Σ_c + λI with λ = 0.01; Σ_reg_inv = `np.linalg.inv(Σ_reg)`.
- Score per hunk:
  - **Cluster-routed**: `d² = (z − μ_c)ᵀ Σ_reg_inv (z − μ_c)`.
  - **Corpus-fallback** (cluster_id == -1 OR cluster has < 5 controls): `d²_fallback = z · z`. In the whitened PCA space, the corpus controls have mean 0 and covariance ≈ I by construction, so this is Mahalanobis-to-corpus-mean with Σ = I.
- Per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL d² values (across both routings).
- No catalog labels touched any model fitting. The script asserts `feat[ctrl_mask]` is the only input to `pca.fit`; `is_break` is used only to mask-out breaks before fitting.

---

## Task 1 — PCA per corpus (controls only)

Cumulative explained-variance fractions of the first 1, 10, 64 principal components, fit on the corpus's CONTROL rows.

| Corpus | n_controls | Top-1 var | Top-10 var (cum.) | Top-64 var (cum.) |
|---|---:|---:|---:|---:|
| fastapi  | 295 | 0.091 | 0.423 | **0.844** |
| rich     | 300 | 0.072 | 0.389 | **0.820** |
| faker    | 297 | 0.103 | 0.442 | **0.846** |
| hono     | 297 | 0.101 | 0.422 | **0.838** |
| ink      | 289 | 0.085 | 0.435 | **0.850** |
| faker-js | 298 | 0.105 | 0.455 | **0.853** |

**64 dimensions captures 82–85 % of variance per corpus.** No red flags on the PCA itself; the dimensionality is well-justified. The first 10 PCs alone capture 39–46 % of variance.

---

## Task 2 — Per-(corpus, cluster) Mahalanobis construction

| Corpus | Valid cluster models | Skipped (low-pop, < 5) | Unmappable rows (cluster=-1) | Control rows |
|---|---:|---:|---:|---:|
| fastapi  | 8 | 0 | 5   | 295 |
| rich     | 7 | 0 | 0   | 300 |
| faker    | 5 | 2 | 30  | 297 |
| hono     | 7 | 0 | 36  | 297 |
| ink      | 5 | 1 | 20  | 289 |
| faker-js | 6 | 2 | 173 | 298 |

**38 cluster Mahalanobis models built** (same set as Phase 6.4 centroids). No singular Σ_reg encountered → no pseudo-inverse fallback needed.

Critical: of those 38 models, **30 have n_ctrl < PCA_DIM = 64**, and the remaining 8 have n_ctrl ∈ [64, 176]. In the rank-deficient regime (k < d), Σ has rank ≤ k − 1; λI regularization fills the null-space with 1/λ = 100. This is the pathology that drives the apparent SHIP signal (see Task 6b).

---

## Task 3 — Per-hunk routing

| Corpus | Total | Via cluster | Via corpus-fallback | Excluded | Breaks via cluster / fallback / excl | Controls via cluster / fallback / excl |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 327 | 322 | 5   | 0 | 32/0/0  | 290/5/0   |
| rich     | 316 | 316 | 0   | 0 | 16/0/0  | 300/0/0   |
| faker    | 313 | 278 | 35  | 0 | 16/0/0  | 262/35/0  |
| hono     | 314 | 278 | 36  | 0 | 17/0/0  | 261/36/0  |
| ink      | 306 | 285 | 21  | 0 | 17/0/0  | 268/21/0  |
| faker-js | 315 | 136 | 179 | 0 | 15/2/0  | 121/177/0 |

**Coverage now 100 % on every corpus** (no excluded rows, unlike Phase 6.4). Whitened-space corpus-fallback is the safety net Phase 6.4 lacked. Of faker-js's 17 breaks: 15 route through 4 cluster models (clusters 0, 2, 6, 7) and 2 route through corpus-fallback.

---

## Task 4 — Per-corpus threshold calibration

Threshold = (1 − FP_target/100)-quantile of CONTROL d² across both routings.

| Corpus | FP target | Threshold (d²) | Controls (n) | Controls flagged | Actual FP % | Δ vs target |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % |  90.00 | 295 | 2 | 0.678 % | +0.08 pp |
| rich     | 1.2 % | 113.92 | 300 | 4 | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 116.23 | 297 | 6 | 2.020 % | +0.02 pp |
| hono     | 0.5 % | 110.37 | 297 | 2 | 0.673 % | +0.17 pp |
| ink      | 0.5 % | 111.53 | 289 | 2 | 0.692 % | +0.19 pp |
| faker-js | 0.9 % | 127.34 | 298 | 3 | 1.007 % | +0.11 pp |

Threshold values are not directly comparable to Phase 6.4 cosine distances (different units — squared Mahalanobis distance vs cosine distance). All actual FP rates land within +0.2 pp of target on every corpus → **no-regression gate passes**.

The thresholds are dominated by corpus-fallback-routed controls (whitened-space squared L2 with Σ = I, so d² ≈ 64 in expectation by chi-squared(64) under the nominal model). Cluster-routed controls have much smaller d² (single-digit to low tens) because Σ is fit on those very controls and they live in a low-dim subspace of PCA-64.

---

## Task 5 — Residual fixture catch (the killer test)

Apply faker-js threshold (127.34) to the 5 residuals.

| Fixture | Cluster | Route | d² (this) | Threshold | Catch? | Top-X% (rank vs fjs ctrl) | Phase 6.4 cosine |
|---|---:|---|---:|---:|:---:|---:|---:|
| `error_flip_2`   | 3 | corpus_fallback |   35.25 | 127.34 | ✗ | top 44.97 % | — (excluded under 6.4) |
| `error_flip_3`   | 2 | cluster         | 1532.48 | 127.34 | **✓** | top 0.0 %   | 0.3138 |
| `runtime_fetch_1`| 2 | cluster         | 2358.61 | 127.34 | **✓** | top 0.0 %   | 0.4670 |
| `runtime_fetch_2`| 6 | cluster         | 3941.50 | 127.34 | **✓** | top 0.0 %   | 0.4931 |
| `runtime_fetch_3`| 7 | cluster         | 2181.34 | 127.34 | **✓** | top 0.0 %   | 0.4248 |

**Catch count: 4 of 5.** Mechanically passes SHIP gate (≥ 2). All 4 caught residuals are cluster-routed. All 4 sit at rank-top-0 % among fjs controls — that is, **larger d² than every single fjs control**. The fifth, `error_flip_2`, routes to corpus-fallback (its cluster 3 has only 2 controls) and lands at the 55th percentile of fjs controls — typical-looking, same finding as Phase 6.4.

`error_flip_3`, which sits at the 64th percentile in Phase 6.4 cosine and was characterized as "genuinely typical-looking", catches at d² = 1532.48 here. That alone is worth scrutiny: the fixture's true embedding-anomaly status hasn't changed between Phase 6.4 and Phase 7 — only the scoring function has. The catch comes from `error_flip_3`'s cluster (faker-js cluster 2: 28 controls, in-sample max control d² = 25.92). Mahalanobis under a rank-deficient Σ can produce dramatic separation between the in-sample training controls and any other point.

---

## Task 6 — Per-corpus stage-4 recall + FP audit

| Corpus | FP target | Threshold | Breaks (total / scored / caught) | Stage-4 recall | Actual FP % | FP regression |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % |  90.00 | 32 / 32 / **32** | **100.0 %** | 0.678 % | +0.08 pp |
| rich     | 1.2 % | 113.92 | 16 / 16 / 4 | 25.0 %  | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 116.23 | 16 / 16 / **16** | **100.0 %** | 2.020 % | +0.02 pp |
| hono     | 0.5 % | 110.37 | 17 / 17 / **17** | **100.0 %** | 0.673 % | +0.17 pp |
| ink      | 0.5 % | 111.53 | 17 / 17 / **17** | **100.0 %** | 0.692 % | +0.19 pp |
| faker-js | 0.9 % | 127.34 | 17 / 17 / 15 | 88.24 % | 1.007 % | +0.11 pp |

**No-regression gate: PASS on all 6/6 corpora.**
**Stage-4 catalog recall: 101 / 115 = 87.83 %.**

These numbers are obviously absurd as a real result. A single zero-training feature catching 100 % of fastapi/faker/hono/ink breaks at sub-baseline-+0.2 pp FP would mean every catalog feature in era 11 is dispensable. The right interpretation is that breaks systematically score in the "rank-deficient inflation" regime that held-out controls also score in. Task 6b confirms this is the case.

---

## Task 6b — Leave-one-out control sanity check (decisive diagnostic)

For each cluster-routed (corpus, cluster_id) Mahalanobis model: hold out one control at a time, refit μ + Σ_reg from the remaining controls, and score the held-out control. If LOO max control d² >> in-sample max control d² (ratio > 5×), the apparent break-vs-control separation is rank-deficiency-driven.

**Of 38 cluster models evaluated, 37 are flagged as rank-deficiency artifacts (LOO/in-sample max ratio > 5×).** The single non-flagged model is `rich cluster 6` (n_ctrl = 176, ratio 1.29).

Selected per-cluster LOO results, sorted by n_controls:

| Corpus | Cluster | n_ctrl | rank-def? | in-sample max d² | LOO max d² | LOO/in-sample | Artifact? |
|---|---:|---:|:---:|---:|---:|---:|:---:|
| fastapi   | 7 |   5 | ✓ |    3.20 |  7 931.35 | **2 480 ×** | ✓ |
| hono      | 3 |   5 | ✓ |    3.20 |  7 458.68 | **2 332 ×** | ✓ |
| hono      | 0 |   6 | ✓ |    4.16 | 13 218.35 | **3 174 ×** | ✓ |
| rich      | 2 |   7 | ✓ |    5.14 | 14 624.75 | **2 845 ×** | ✓ |
| faker-js  | 0 |  10 | ✓ |    8.08 |  4 142.89 |   513 ×    | ✓ |
| faker-js  | 6 |  13 | ✓ |   11.07 | 18 107.27 | **1 636 ×** | ✓ |
| faker-js  | 2 |  28 | ✓ |   25.92 |  6 121.46 |   236 ×    | ✓ |
| faker-js  | 7 |  31 | ✓ |   28.93 |  8 678.53 |   300 ×    | ✓ |
| ink       | 1 |  64 | ✗ |   60.27 |    746.22 |    12 ×    | ✓ |
| hono      | 1 |  78 | ✗ |   74.55 |  2 855.07 |    38 ×    | ✓ |
| fastapi   | 3 |  92 | ✗ |   86.03 |  1 456.48 |    17 ×    | ✓ |
| fastapi   | 1 |  97 | ✗ |   91.12 |  2 265.33 |    25 ×    | ✓ |
| ink       | 0 | 114 | ✗ |   93.76 |    581.54 |     6 ×    | ✓ |
| hono      | 4 | 131 | ✗ |  113.43 |    568.74 |     5 ×    | ✓ |
| faker     | 3 | 148 | ✗ |  134.94 |  1 795.27 |    13 ×    | ✓ |
| **rich**      | **6** | **176** | ✗ |  **135.04** |    **174.75** |     **1.3 ×** | **✗** |

The pattern is sharp:
- **n_ctrl < 64**: ratios 100×–3 000×. The covariance is severely rank-deficient; the regularizer dominates.
- **n_ctrl ∈ [64, 148]**: ratios 5×–38×. Still flagged. The covariance is technically full-rank but small-sample-noisy; the high-eigenvalue tail of Σ_reg_inv still inflates LOO d².
- **n_ctrl ≥ 176**: ratio 1.3×. Honest behaviour — LOO and in-sample agree, as they should.

For the single residual catch we want to credit (`runtime_fetch_2`, fjs cluster 6, d² = 3 941):
- LOO max control d² in the same cluster = 18 107
- LOO mean control d² = 5 217
- → The break is below the **mean** LOO control score for its own cluster

There is no signal here. The residual scores look like held-out controls in the same cluster.

**Interpretation**: with `n_ctrl < 64`, Σ has rank ≤ n_ctrl − 1 < 64. Apply λI: Σ_reg has full rank but eigenvalues structured as `(σ₁², …, σ_k², λ, λ, …, λ)`. The inverse has eigenvalues `(1/σ₁², …, 1/σ_k², 100, 100, …, 100)`. Any point with non-trivial component along the (64 − k + 1) null-space directions gets 100× that squared component added to d². Held-out controls have such a component (they were not in the training set for Σ); breaks have such a component for the same reason. The metric does not distinguish them.

This is **not a label-leak** — `is_break` is never used as input. It is a degrees-of-freedom artifact: the metric implicitly tests "is this point in the linear span of the training controls?" which is true by definition only of the training controls themselves.

---

## Task 7 — Faker-js top-20 controls diagnostic

Top-20 faker-js controls sorted by d² descending.

| Rank | File | Cluster | Route | d² | Above thr (127.34)? |
|---:|---|---:|---|---:|:---:|
| 1  | `src/locales/pl/book/format.ts`              | -1 | corpus_fallback | 135.96 | **✓** |
| 2  | `src/locales/ar/vehicle/fuel.ts`             | -1 | corpus_fallback | 133.93 | **✓** |
| 3  | `vitest.config.ts`                           |  5 | corpus_fallback | 129.32 | **✓** |
| 4  | `src/locales/base/system/mime_type.ts`       | -1 | corpus_fallback | 126.37 | ✗ |
| 5  | `src/locale/index.ts`                        | -1 | corpus_fallback | 122.60 | ✗ |
| 6  | `vitest.config.ts`                           |  5 | corpus_fallback | 115.34 | ✗ |
| 7  | `eslint.config.ts`                           |  3 | corpus_fallback | 112.24 | ✗ |
| 8  | `src/locales/base/system/mime_type.ts` (...) | -1 | corpus_fallback | 107.75 | ✗ |
| ... | ... |  | corpus_fallback | ... | ✗ |
| 20 | `src/locales/id_ID/internet/index.ts`        | -1 | corpus_fallback |  88.02 | ✗ |

**All top-20 fjs controls are corpus-fallback-routed.** The 3 flagged controls are: 2 locale data files (`pl/book/format.ts`, `ar/vehicle/fuel.ts`) and `vitest.config.ts`. None are in `src/modules/...` (provider modules) — same kind of files Phase 6.4b exposed as the FP risk in the corpus-fallback tail. The threshold lands among locale data, which is not surprising — locale data is the most distinctive content in faker-js, as Phase 6.4b documented.

The interesting absence: **no cluster-routed control is in the top 20.** Cluster-routed control d² values cap out around 30 (in-sample regime); cluster-routed BREAK d² values are 1500–4000 (in the rank-deficiency-inflated regime that LOO would also produce for held-out controls). The "separation" between cluster-routed breaks and cluster-routed controls is real-but-vacuous: held-out controls would join the breaks in that regime.

---

## Task 8 — Verdict

| Pre-registered condition | Result | Mechanical pass | Honest pass |
|---|---|:---:|:---:|
| ≥ 2 of 5 residual catches at faker-js FP ≤ 0.9 % | 4 / 5 | ✓ | ✗ (LOO controls also catch) |
| Per-corpus FP ≤ baseline + 0.5 pp on every corpus | max +0.19 pp (ink) | ✓ | ✓ (FP gate is not affected by the artifact) |
| Stage-4 break recall ≥ 0 (no-regression on existing era-11) | +101 catalog catches | ✓ | ✗ (artifact would also flag held-out controls) |

**MECHANICAL VERDICT: SHIP.** **HONEST VERDICT: CLOSE NEGATIVE.**

The pre-registered SHIP gate is satisfied on its written terms, but the LOO sanity check (Task 6b) is decisive: 37 of 38 cluster Mahalanobis models are rank-deficiency-dominated, and the apparent break detection signal cannot be distinguished from inflation that would also affect held-out controls. The 4 residual catches are a statistical artifact of `n_ctrl < PCA_DIM` in 30 of 38 clusters and small-sample covariance noise in the rest.

Recommendation: do **not** ship Phase 7 as a stage-4 scorer. The mechanical SHIP would land an artifact in production that flags held-out controls as breaks at 100 % rate.

---

## Comparison: Phase 6.4 vs Phase 6.4b vs Phase 7

| Phase | Method | fjs residual catch | Catalog recall (115) | Honest? |
|---|---|---:|---:|:---:|
| 6.4 | per-cluster cosine, ≥ 5 controls, k-controls excluded | 1 (`runtime_fetch_2`) | 9  (7.83 %) | ✓ |
| 6.4b | + corpus-wide cosine fallback for excluded | 0                 | 6  (5.22 %) | ✓ |
| 7 | per-cluster Mahalanobis on PCA-64 + whitened-space fallback | 4 (mechanically) | 101 (87.83 %) | **✗** |

Phase 7 looks dramatically stronger by mechanical numbers but is the cleanest negative when the LOO sanity check is applied. Phase 6.4 remains the strongest *honest* result on the embedding axis: 1 residual catch + 7.83 % catalog recall, with no rank-deficiency pathology because cosine distance does not invert any covariance matrix.

| Per-corpus FP | 6.4 | 6.4b | 7 (mechanical) |
|---|---:|---:|---:|
| fastapi  | 0.690 % | 0.678 % | 0.678 % |
| rich     | 1.333 % | 1.333 % | 1.333 % |
| faker    | 2.290 % | 2.020 % | 2.020 % |
| hono     | 0.383 % | 0.337 % | 0.673 % |
| ink      | 0.746 % | 0.692 % | 0.692 % |
| faker-js | 0.826 % | 1.007 % | 1.007 % |

FP rates are similar across all three phases — the FP gate is not where the artifact bites. The artifact bites the BREAK side of the calculation only, because breaks are the only points scored in the regime where rank-deficiency dominates *and* are also the points the gate counts.

---

## Implications for era 14

After Phase 6.2 → 6.3 → 6.4 → 6.4b → 7, the embedding-anomaly axis has produced one honest residual catch (`runtime_fetch_2` in 6.4) at the cost of small but real FP drift, and a fully artifact-driven 4/5 in Phase 7 that does not survive a LOO check. Across five swings, the residual problem looks structurally hard:

- The 3 `runtime_fetch_*` residuals all sit in top-5 % of fjs control distances under cosine. They ARE anomalous, but not separable cleanly enough at 0.9 % FP.
- `error_flip_2` is genuinely typical against the corpus-wide centroid (Phase 6.4b). Its Phase 6.2 distance was small-sample noise.
- `error_flip_3` is at the 64th percentile under cosine. It is genuinely typical-looking.
- Mahalanobis-with-rank-deficiency conjures separation that is not real signal.

**Close era 14.** The honest residual catch ceiling on this feature axis is 1/5 (Phase 6.4 `runtime_fetch_2`). Either ship Phase 6.4 as opt-in stage-4 for the modest catalog gain (+9 catches), or close the era with the negative result documented and accept that the residual problem is not solvable with single-feature embedding distances at era 11's FP budget.

Phase 7's main contribution is **negative knowledge**: it rules out Mahalanobis on per-cluster covariances as a fix, and identifies the rank-deficiency mechanism that any future "tighter cluster-aware metric" attempt would have to confront. Specifically, any covariance-based scoring with Σ fit on k < d controls will reproduce this artifact unless either (a) k ≥ d (requires more data than we have per cluster) or (b) Σ is shrunk strongly toward the corpus-wide whitened identity, which collapses the metric back to the corpus-fallback whitened L² that Phase 6.4b already explored and which loses the one Phase 6.4 catch.

There is no remaining methodological lever on this axis. The era closes on Phase 6.4's PARTIAL.
