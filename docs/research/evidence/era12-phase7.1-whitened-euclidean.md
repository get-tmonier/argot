# Era 12 Phase 7.1 — PCA-whitened squared-Euclidean (per-cluster anchored), the clean ablation of Phase 7

**Date**: 2026-05-03
**Branch**: `feat/era-12-ml-stage`
**Script**: `engine/scripts/era12_phase71_whitened_euclidean.py`
**Inputs**: `engine/.era12-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (1891 rows; 115 breaks, 1776 controls)
**Persisted artifacts**:
- Whitened-Euclidean dict (PCA models + per-cluster μ_c + per-corpus μ + thresholds): `engine/.era12-features/phase71_whitened_euclidean.joblib`
- Raw results JSON: `/tmp/era12_phase71_results.json`

---

## TL;DR

**VERDICT: CLOSE NEGATIVE.**

Phase 7.1 isolates the *honest* component of Phase 7's design (PCA-whitening + per-cluster anchoring) by replacing the broken per-cluster Σ with a corpus-pooled Σ implicit in PCA-whitening. The metric is squared Euclidean in PCA-64 whitened space, anchored at per-cluster μ_c.

Three findings:

1. **LOO sanity check passes cleanly.** Of 38 cluster models, **0 flagged** as artifact (ratio > 5×); only **2 marginal** (ratio > 1.5×, both at 1.56×). Mean ratio 1.14×, median 1.08×, max 1.56×. Compare to Phase 7's 37 / 38 flagged with ratios 5×–3 174×. The ablation diagnoses the Phase 7 pathology correctly: Σ_corpus on n ≈ 297 ≫ d = 64 has no rank-deficient null space, so held-out controls do not get inflated d².
2. **Zero residuals catch.** All 5 faker-js residuals score d² in 35–45, against fjs threshold 162.79. They sit at the 67th–80th percentile of fjs controls — typical-looking. Phase 7's 4/5 catch was driven entirely by the rank-deficiency artifact; once the artifact is removed, the residuals don't separate from controls.
3. **Stage-4 recall is 0/115 across all six corpora.** No catalog gain. FP rates pass the no-regression gate (≤ baseline + 0.5 pp on every corpus).

Phase 7.1 is the *honest* version of Phase 7 — and it confirms there is no embedding-anomaly signal at this metric. It also strengthens the era 12 close: the only embedding metric that catches anything (Phase 6.4 cosine, 1/5) cannot be improved by PCA-whitening + per-cluster anchoring.

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

**Method**:

- Per corpus: fit `sklearn.decomposition.PCA(n_components=64, whiten=True)` on the **CONCATENATED** `[hunk_embedding, context_embedding]` (1536-d) of CONTROL rows ONLY (`is_break == False`). Project ALL rows in that corpus into PCA-64 whitened space. **(IDENTICAL to Phase 7.)**
- For each `(corpus, cluster_id)` with `cluster_id != -1` and ≥ MIN_CLUSTER_CONTROLS = 5 controls: μ_c = mean of those controls in PCA-64 space. **No per-cluster covariance.**
- For each corpus: μ_corpus = mean of all CONTROL z_pca for the corpus (used for fallback scoring; ≈ 0 by construction since PCA whitening centers on controls, but computed explicitly).
- Score per hunk:
  - **Cluster-routed** (cluster_id ≠ -1, k_c ≥ 5): `d² = ‖z - μ_c‖²` (squared Euclidean in PCA-64).
  - **Corpus-fallback** (cluster_id == -1 OR k_c < 5): `d² = ‖z - μ_corpus‖²`.
- Per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL d² values (across both routings).
- No catalog labels touched any model fitting. The script asserts `feat[ctrl_mask]` is the only input to `pca.fit`; `is_break` is used only as a mask, never as a feature.

**Mathematical equivalence**: Mahalanobis distance with Σ = corpus-pooled covariance (n ≈ 297 controls), restricted to its top-64 eigenvector subspace (PCA truncation), anchored at per-cluster μ_c. The whitening normalizes per-PC variance — the principled selling point of Phase 7 — without estimating a per-cluster covariance.

---

## Task 1 — PCA per corpus (controls only)

| Corpus | n_controls | Top-1 var | Top-10 var (cum.) | Top-64 var (cum.) |
|---|---:|---:|---:|---:|
| fastapi  | 295 | 0.091 | 0.423 | **0.844** |
| rich     | 300 | 0.072 | 0.389 | **0.820** |
| faker    | 297 | 0.103 | 0.442 | **0.846** |
| hono     | 297 | 0.101 | 0.422 | **0.838** |
| ink      | 289 | 0.085 | 0.435 | **0.850** |
| faker-js | 298 | 0.105 | 0.455 | **0.853** |

**Identical to Phase 7** by construction (same fit on same data). Cross-check `np.max(|p7.components_ - p71.components_|) == 0.0` on every corpus's `pca_models[c].components_`.

---

## Task 2 — Per-(corpus, cluster) μ_c construction

| Corpus | Valid cluster μ_c | Skipped (low-pop, < 5) | Unmappable rows (cluster=-1) | Control rows | ‖μ_corpus‖ |
|---|---:|---:|---:|---:|---:|
| fastapi  | 8 | 0 | 5   | 295 | 1.4e-7 |
| rich     | 7 | 0 | 0   | 300 | 1.7e-7 |
| faker    | 5 | 2 | 30  | 297 | 8.4e-8 |
| hono     | 7 | 0 | 36  | 297 | 1.6e-7 |
| ink      | 5 | 1 | 20  | 289 | 1.0e-7 |
| faker-js | 6 | 2 | 173 | 298 | 1.7e-7 |

**38 cluster μ_c built** (same set as Phase 7 / Phase 6.4). μ_corpus ≈ 0 in every corpus (whitened space is centered on controls).

The number of valid cluster models is identical to Phase 7. The only difference is what's attached to each (μ vs. μ + Σ_reg_inv).

---

## Task 3 — Per-hunk routing

| Corpus | Total | Via cluster | Via corpus-fallback | Excluded | Breaks via cluster / fallback | Controls via cluster / fallback |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 327 | 322 | 5   | 0 | 32/0  | 290/5   |
| rich     | 316 | 316 | 0   | 0 | 16/0  | 300/0   |
| faker    | 313 | 278 | 35  | 0 | 16/0  | 262/35  |
| hono     | 314 | 278 | 36  | 0 | 17/0  | 261/36  |
| ink      | 306 | 285 | 21  | 0 | 17/0  | 268/21  |
| faker-js | 315 | 136 | 179 | 0 | 15/2  | 121/177 |

**Coverage 100 % on every corpus** (no excluded rows). Routing is identical to Phase 7 — same rule, same data. 15 of 17 fjs breaks route through 4 cluster models (clusters 0, 2, 6, 7); 2 route through corpus-fallback.

---

## Task 4 — Per-corpus threshold calibration

| Corpus | FP target | Threshold (d²) | Controls (n) | Controls flagged | Actual FP % | Δ vs target |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 137.41 | 295 | 2 | 0.678 % | +0.08 pp |
| rich     | 1.2 % | 131.97 | 300 | 4 | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 121.81 | 297 | 6 | 2.020 % | +0.02 pp |
| hono     | 0.5 % | 135.82 | 297 | 2 | 0.673 % | +0.17 pp |
| ink      | 0.5 % | 123.92 | 289 | 2 | 0.692 % | +0.19 pp |
| faker-js | 0.9 % | 162.79 | 298 | 3 | 1.007 % | +0.11 pp |

**No-regression gate: PASS on all 6 corpora** (max +0.19 pp ink, exactly the same as Phase 7). FP calibration is well-behaved; the metric does *not* over-flag at the calibrated threshold.

Threshold values land near 64 × (small constant) — natural for chi-squared(64) distributed whitened-space distances. The d² values are on the same order as `feat_pca · feat_pca` (whitened-space squared L2), since μ_c is a small offset relative to z's own norm in 64-d.

---

## Task 5 — Residual fixture catch

Apply faker-js threshold (162.79) to the 5 residuals. Side-by-side with Phase 6.4 cosine and Phase 7 d².

| Fixture | Cluster | Route | 7.1 d² | Threshold | Catch? | Top-X% rank | 6.4 cosine | 7 d² |
|---|---:|---|---:|---:|:---:|---:|---:|---:|
| `error_flip_2`   | 3 | corpus_fallback | 35.25 | 162.79 | ✗ | top 79.5 % | — | 35.25 |
| `error_flip_3`   | 2 | cluster         | 36.40 | 162.79 | ✗ | top 77.9 % | 0.3138 | 1532.48 |
| `runtime_fetch_1`| 2 | cluster         | 42.15 | 162.79 | ✗ | top 69.1 % | 0.4670 | 2358.61 |
| `runtime_fetch_2`| 6 | cluster         | 44.79 | 162.79 | ✗ | top 66.8 % | 0.4931 | 2181.34 |
| `runtime_fetch_3`| 7 | cluster         | 39.69 | 162.79 | ✗ | top 72.5 % | 0.4248 | 2181.34 |

**Catch count: 0 of 5.** SHIP gate fails on residual count.

The residuals score d² in 35–45 against fjs control distribution that runs from very-small to 222 (max). They sit between the 20th and 33rd percentile of fjs controls — well below the 99.1th-percentile threshold (162.79). The Phase 7 d² values for these same fixtures are 1500–4000; the >36×–88× gap between Phase 7 d² and Phase 7.1 d² *for the same fixture in the same cluster* is the rank-deficient inflation Phase 6b diagnosed for Phase 7. With the inflation removed, no signal remains.

`error_flip_2` is identical between Phase 7 and Phase 7.1 (35.25 in both): it routes to corpus-fallback, where Phase 7's d² is `z·z` and Phase 7.1's d² is `‖z - μ_corpus‖² ≈ z·z` (since μ_corpus ≈ 0 by whitening). This is the only fixture where the two phases agree by construction.

---

## Task 6 — Per-corpus stage-4 recall + FP audit

| Corpus | FP target | Threshold | Breaks (total / scored / caught) | Stage-4 recall | Actual FP % | FP regression |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 137.41 | 32 / 32 / **0** | 0.0 % | 0.678 % | +0.08 pp |
| rich     | 1.2 % | 131.97 | 16 / 16 / 0 | 0.0 % | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 121.81 | 16 / 16 / 0 | 0.0 % | 2.020 % | +0.02 pp |
| hono     | 0.5 % | 135.82 | 17 / 17 / 0 | 0.0 % | 0.673 % | +0.17 pp |
| ink      | 0.5 % | 123.92 | 17 / 17 / 0 | 0.0 % | 0.692 % | +0.19 pp |
| faker-js | 0.9 % | 162.79 | 17 / 17 / 0 | 0.0 % | 1.007 % | +0.11 pp |

**No-regression gate: PASS on all 6/6 corpora.**
**Stage-4 catalog recall: 0 / 115 = 0.0 %.**

This is the honest comparison to Phase 7's 101/115 = 87.83 %. The gap (+101 catches) is exactly what the rank-deficiency inflation was buying — it inflated break d² above threshold while leaving controls below threshold *only because controls were in-sample for their cluster's Σ*. Once Σ becomes a corpus-pooled, well-conditioned object, breaks and controls no longer separate.

This is also the principled reason to expect 0 catches: the residuals' raw embeddings are not anomalously far from their cluster's centroid in PCA-whitened space. Phase 6.4 cosine catches `runtime_fetch_2` because it routes through a tiny cluster (n=13) where it happens to be the 7th-most distant control by cosine; PCA-whitening + per-cluster μ does not concentrate that signal.

---

## Task 7 — LOO sanity diagnostic (THE CRITICAL TABLE)

For each cluster-routed (corpus, cluster_id) μ_c: hold out one control at a time, refit μ_c from the remaining controls (PCA *not* refit; only μ_c changes), score the held-out control. Phase 7.1's expected behaviour: ratio close to 1× across the board, since corpus-pooled Σ is well-conditioned.

**Aggregate**:

| Diagnostic | Phase 7.1 | Phase 7 |
|---|---:|---:|
| Clusters evaluated | 38 | 38 |
| Clusters with ratio > 5× (artifact-flag threshold) | **0** | **37** |
| Clusters with ratio > 1.5× (mild-inflation flag) | **2** | (~all 38) |
| Max LOO/in-sample ratio | **1.563** | 3 174 |
| Mean ratio | **1.14** | (>>1) |
| Median ratio | **1.08** | (>>1) |

**0 of 38 clusters flagged as artifact.** Phase 7.1 LOO sanity gate passes cleanly. The ablation has correctly removed the rank-deficient pathology.

Per-cluster breakdown, sorted by ratio descending (top 16):

| Corpus | Cluster | n_ctrl | rank-def in Σ_corpus? | in-sample max d² | LOO max d² | LOO/in-sample | Flag? |
|---|---:|---:|:---:|---:|---:|---:|:---:|
| fastapi  | 7 |  5 | ✗ |  63.16 |  98.69 | 1.563 | (mild) |
| hono     | 3 |  5 | ✗ |  60.12 |  93.94 | 1.562 | (mild) |
| hono     | 0 |  6 | ✗ | 107.20 | 154.37 | 1.440 | — |
| faker    | 0 |  7 | ✗ | 120.36 | 163.82 | 1.361 | — |
| rich     | 2 |  7 | ✗ | 118.23 | 160.92 | 1.361 | — |
| hono     | 5 |  7 | ✗ | 106.68 | 145.21 | 1.361 | — |
| fastapi  | 4 |  8 | ✗ |  76.79 | 100.30 | 1.306 | — |
| fastapi  | 2 |  9 | ✗ | 105.99 | 134.14 | 1.266 | — |
| faker-js | 0 | 10 | ✗ |  66.43 |  82.01 | 1.235 | — |
| faker-js | 1 | 12 | ✗ |  67.56 |  80.41 | 1.190 | — |
| rich     | 1 | 12 | ✗ | 132.76 | 158.00 | 1.190 | — |
| fastapi  | 5 | 12 | ✗ |  83.13 |  98.94 | 1.190 | — |
| faker-js | 6 | 13 | ✗ | 167.72 | 196.83 | 1.174 | — |
| rich     | 4 | 15 | ✗ |  79.95 |  91.78 | 1.148 | — |
| hono     | 7 | 17 | ✗ | 134.67 | 152.03 | 1.129 | — |
| ...      | ... | ... | ✗ | ... | ... | ~1.08 | — |

(rank-def column = "is Σ rank-deficient?"; for Phase 7.1 the answer is always **no** because Σ here is implicit in the PCA fit on n ≈ 297, not on the cluster-local k.)

**The 1.5× peak is the small-sample μ_c shift in 5-control clusters.** When you remove one of 5 points from a mean, the LOO mean shifts by `1/4 × (point - mean_orig)`. The expected ratio between the LOO d² of a removed point and the in-sample d² of the most-extreme remaining point is on the order of `(k/(k-1))² + small noise`, which for k=5 gives ≈ 1.56. **This is honest small-sample behaviour**, not an artifact. Both 1.56× clusters are at k=5.

**Comparison with Phase 7**: at k=5, Phase 7's LOO max ratio is 2 480× and 2 332× for the same two clusters. The pathology removed is decisive — about 3 orders of magnitude.

---

## Task 8 — Faker-js top-20 controls diagnostic

Top-20 faker-js controls sorted by d² descending.

| Rank | File | Cluster | Route | d² | Above thr (162.79)? |
|---:|---|---:|---|---:|:---:|
|  1 | `src/internal/keys.ts`                       |  4 | cluster | 222.36 | **✓** |
|  2 | `cypress/e2e/guide.cy.ts`                    |  6 | cluster | 167.72 | **✓** |
|  3 | `src/internal/base64.ts`                     |  4 | cluster | 166.76 | **✓** |
|  4 | `src/modules/internet/index.ts`              |  4 | cluster | 160.86 | ✗ |
|  5 | `src/locales/pl/book/format.ts`              | -1 | corpus_fallback | 135.96 | ✗ |
|  6 | `src/locales/ar/vehicle/fuel.ts`             | -1 | corpus_fallback | 133.93 | ✗ |
|  7 | `src/modules/internet/index.ts`              |  4 | cluster | 132.93 | ✗ |
|  8 | `vitest.config.ts`                           |  5 | corpus_fallback | 129.32 | ✗ |
|  9 | `src/modules/finance/index.ts`               |  2 | cluster | 128.15 | ✗ |
| 10 | `src/modules/color/index.ts`                 |  7 | cluster | 127.45 | ✗ |
| ... | (locale data, modules, configs) | mixed | mixed | ~107–127 | ✗ |
| 20 | `src/modules/string/index.ts`                |  7 | cluster | 107.56 | ✗ |

3 controls flagged: `internal/keys.ts`, `cypress/e2e/guide.cy.ts`, `internal/base64.ts`. Mix of cluster-routed and fallback. The threshold (162.79) lands among module-internal utilities and locale data, NOT among the 5 residuals (which all score 35–45, below the 50th percentile). The residuals are not in the high-d² tail of either the control or break populations.

---

## Task 9 — Verdict (pre-registered SHIP gate)

| Pre-registered condition | Result | Pass? |
|---|---|:---:|
| ≥ 2 of 5 residual catches at faker-js FP ≤ 0.9 % | 0 / 5 | ✗ |
| Per-corpus FP ≤ baseline + 0.5 pp on every corpus | max +0.19 pp (ink) | ✓ |
| LOO sanity check (ratio ≤ 5× on > 50 % of clusters) | 0 / 38 flagged | ✓ |

**VERDICT: CLOSE NEGATIVE.** Residual catch fails the SHIP gate. LOO sanity passes cleanly — the metric is honest, and the honest answer is that there is no embedding-anomaly signal at this metric for the 5 residuals.

---

## Comparison: 6.4 / 6.4b / 7 / 7.1

| Phase | Method | fjs residual catch | Catalog recall (115) | LOO honest? |
|---|---|---:|---:|:---:|
| 6.4   | per-cluster cosine, ≥ 5 controls, k-controls excluded | **1** (`runtime_fetch_2`) | 9 (7.83 %) | ✓ (no Σ to invert) |
| 6.4b  | + corpus-wide cosine fallback for excluded | 0 | 6 (5.22 %) | ✓ |
| 7     | per-cluster Mahalanobis on PCA-64 + whitened-space fallback | 4 (mechanical), 0 (honest) | 101 (mechanical), ≈ 0 (honest) | **✗** (37/38 artifact-flagged) |
| **7.1** | **per-cluster μ on PCA-64 + ‖z - μ_c‖² + corpus-fallback** | **0** | **0** (0 %) | **✓ (0/38 flagged, max ratio 1.56×)** |

| Per-corpus FP | 6.4 | 6.4b | 7 | **7.1** |
|---|---:|---:|---:|---:|
| fastapi  | 0.690 % | 0.678 % | 0.678 % | **0.678 %** |
| rich     | 1.333 % | 1.333 % | 1.333 % | **1.333 %** |
| faker    | 2.290 % | 2.020 % | 2.020 % | **2.020 %** |
| hono     | 0.383 % | 0.337 % | 0.673 % | **0.673 %** |
| ink      | 0.746 % | 0.692 % | 0.692 % | **0.692 %** |
| faker-js | 0.826 % | 1.007 % | 1.007 % | **1.007 %** |

FP rates are essentially identical to Phase 7 — calibration is not where the difference bites. Phase 7's "advantage" was entirely in the (artifact-driven) break tail.

The clean comparison **7.1 vs 6.4** isolates the value of PCA-whitening + per-cluster anchoring on its own:
- 6.4 with cosine (no whitening, k-controls excluded): **1 catch / 9 catalog**.
- 7.1 with PCA-whitened L² (with whitening, full coverage, per-cluster μ): **0 catch / 0 catalog**.

PCA-whitening + per-cluster anchoring is *worse* than 6.4 cosine on the residual axis. The reason is plausible in retrospect: cosine on raw 768-d embeddings preserves direction-only similarity in the high-dim representation, where the residuals are anomalous in narrow directions. PCA-64 truncation discards 15–18 % of variance — likely including some of the rare-direction signal that cosine was using. Whitening then reweights the surviving variance uniformly, further smoothing out any narrow-direction anomaly.

---

## Implications for era 12

Phase 7.1 closes the principled refinement of Phase 6.4 → Phase 7 axis with a clean negative:

1. **The Phase 7 mechanical signal was 100 % artifact.** Removing the rank-deficient Σ removes the entire 4/5 catch and the entire 87.83 % catalog recall, leaving 0/5 and 0 %. There was no underlying signal that Σ_cluster was amplifying — Σ_cluster was *generating* the apparent signal by inflating any out-of-training-set point.
2. **PCA-whitening + per-cluster anchoring is a real loss vs. plain cosine.** The honest comparison is 0/5 (7.1) vs 1/5 (6.4). Likely cause: PCA-64 truncation discards rare-direction variance that cosine in 768-d preserves; whitening then redistributes the remaining variance uniformly, further reducing the chance that a narrow-direction anomaly stands out at the threshold.
3. **The era-12 ceiling on the embedding-anomaly axis is the Phase 6.4 PARTIAL.** No principled refinement of the embedding axis (whitening, anchoring, covariance estimation, fallback) has improved on 1/5 + 9 catalog catches without introducing rank-deficiency artifacts.
4. **No further methodological lever exists on this axis.** The "missing improvement" between 6.4 and 7.1 is not a calibration knob or a coverage gap — it's that the residuals are not anomalous in the metric's geometry once the metric is honestly constructed.

**Recommendation**: confirm era 12 close on the negative result. The honest residual catch ceiling on this feature axis remains 1/5 (Phase 6.4 `runtime_fetch_2`). Either ship Phase 6.4 as opt-in stage-4 for the modest catalog gain, or close the era and accept that the residual problem is not solvable with single-feature embedding distances at era 11's FP budget.

Phase 7.1's contribution to the era-12 evidence base:
- It is the **clean ablation** that confirms Phase 7's mechanical signal was rank-deficiency-driven.
- It is the **honest test of PCA-whitening + per-cluster anchoring on its own merits**, and the answer is "worse than plain cosine."
- It establishes that no principled refinement of Phase 6.4 → Phase 7 axis improves on 1/5 catch.
