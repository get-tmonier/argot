# Era 14 Phase 6.4 — Unsupervised cluster-departure scoring on UnixCoder embeddings

**Date**: 2026-05-03
**Branch**: `feat/era-14-ml-stage`
**Script**: `engine/scripts/era14_phase64_centroid.py`
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (1891 rows; 115 breaks, 1776 controls)
**Persisted artifacts**:
- Centroid dict: `engine/.era14-features/centroids_phase6.4.joblib`
- Raw results JSON: `/tmp/era14_phase64_results.json`

---

## TL;DR

**VERDICT: PARTIAL** — 1 of 5 faker-js residuals catches at FP ≤ 0.9% (`runtime_fetch_2` only). SHIP gate (≥2 of 5) does not clear. The no-regression gate clears on all 6 corpora.

The unsupervised pivot fixed the supervised-leak failure of Phase 6.3 (which caught 0/5 residuals from any embedding-based model under LOO), but it does not, on its own, restore the Phase 6.2 STRONG SIGNAL result. Two forces are responsible:

1. **Honest centroids are weaker than Phase 6.2 reported**. Phase 6.2 built centroids from any cluster with ≥1 control, including faker-js cluster 3 which has only **2** controls. Phase 6.4 enforces the pre-registered `MIN_CLUSTER_CONTROLS = 5` filter; this drops `error_flip_2` (cluster 3) from the scored set entirely. The Phase 6.2 number "4/5 residuals above p90" was therefore inflated by trusting a centroid built from 2 samples.
2. **Faker-js coverage is thin**. 173 of 298 faker-js controls (58%) live in cluster `-1` (unmappable) and are excluded from scoring. Adding the low-pop exclusion drops scorable controls to **121 of 298**. The fjs threshold quantile is therefore being read off a 121-row tail.

Net result: the embedding-anomaly stage adds modest stage-4 recall (faker 18.75 %, ink 17.65 %, faker-js 11.76 %, hono 5.88 %, fastapi/rich 0 %) at acceptable FP cost — but the residual catch on faker-js is only 1/5 and the SHIP gate fails.

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

- For each `(corpus, cluster_id)` with `is_break = False`, ≥ 5 controls, `cluster_id ≠ -1`: centroid = L2-normed mean of `hunk_embedding` across that cluster's controls.
- For each row with a centroid for its `(corpus, cluster_id)`: `embedding_distance = 1 − cosine(hunk_n, centroid_n)`.
- Per-corpus threshold = `(1 − FP_target/100)`-quantile of CONTROL distances. Hunks above threshold are flagged.
- No catalog labels touched centroid construction; no classifier trained.

---

## Task 1 — Centroid construction

| Corpus | Valid centroids | Skipped (low-pop) | Unmappable rows (cluster=-1) | Control rows |
|---|---:|---:|---:|---:|
| fastapi  | 8 | 0 | 5   | 295 |
| rich     | 7 | 0 | 0   | 300 |
| faker    | 5 | 2 | 30  | 297 |
| hono     | 7 | 0 | 36  | 297 |
| ink      | 5 | 1 | 20  | 289 |
| faker-js | 6 | 2 | 173 | 298 |

**38 (corpus, cluster_id) centroids built; 5 skipped for < 5 controls.**

The faker-js unmappable share (173/298 = 58 %) dwarfs every other corpus and is what limits this stage's reach on the corpus that matters most.

---

## Task 2 — Per-hunk scoring coverage

| Corpus | Total | Scored | Excluded | Breaks scored | Breaks excluded | Controls scored | Controls excluded |
|---|---:|---:|---:|---:|---:|---:|---:|
| fastapi  | 327 | 322 | 5   | 32/32 | 0 | 290/295 | 5   |
| rich     | 316 | 316 | 0   | 16/16 | 0 | 300/300 | 0   |
| faker    | 313 | 278 | 35  | 16/16 | 0 | 262/297 | 35  |
| hono     | 314 | 278 | 36  | 17/17 | 0 | 261/297 | 36  |
| ink      | 306 | 285 | 21  | 17/17 | 0 | 268/289 | 21  |
| faker-js | 315 | 136 | 179 | 15/17 | 2 | 121/298 | 177 |

**Two faker-js breaks fall outside the scoring set entirely**: `error_flip_2` (cluster 3, only 2 controls → skipped) and one other. This means the residual catch is bounded above at 4/5 by construction — `error_flip_2` cannot be caught at any threshold by this stage as currently spec'd.

---

## Task 3 — Per-corpus thresholds

Threshold = `(1 - FP_target/100)`-quantile of the corpus's control distances.

| Corpus | FP target | Threshold | Controls (n) | Controls flagged | Actual FP % |
|---|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 0.5689 | 290 | 2 | 0.690 % |
| rich     | 1.2 % | 0.5401 | 300 | 4 | 1.333 % |
| faker    | 2.0 % | 0.4445 | 262 | 6 | 2.290 % |
| hono     | 0.5 % | 0.5991 | 261 | 1 | 0.383 % |
| ink      | 0.5 % | 0.5358 | 268 | 2 | 0.746 % |
| faker-js | 0.9 % | 0.4892 | 121 | 1 | 0.826 % |

Actual FP rates land within ±0.3 pp of target — small drift from quantile interpolation. All within baseline + 0.5 pp.

---

## Task 4 — Residual fixture catch (THE KILLER TEST)

Apply faker-js threshold (0.4892) to the 5 residual fixtures.

| Fixture | Distance | Threshold | Crosses? | Top-X% (rank vs controls) | File |
|---|---:|---:|:---:|---:|---|
| `error_flip_2`   | — (excluded; cluster 3 has 2 controls) | 0.4892 | — | — | `src/internal/locale-proxy.ts` |
| `error_flip_3`   | 0.3138 | 0.4892 | ✗ | top 35.5 % | `src/modules/airline/index.ts` |
| `runtime_fetch_1`| 0.4670 | 0.4892 | ✗ (close) | top 2.5 % | `src/internal/locale-proxy.ts` |
| `runtime_fetch_2`| 0.4931 | 0.4892 | **✓** | top 0.83 % | `src/modules/image/index.ts` |
| `runtime_fetch_3`| 0.4248 | 0.4892 | ✗ | top 4.1 % | `src/modules/company/index.ts` |

**Catch count: 1 of 5.** SHIP gate (≥ 2) **fails**.

Three observations:

1. The three `runtime_fetch_*` residuals all land in the top-5% of faker-js control distances — they ARE anomalous, just not anomalous enough to clear a 0.9 %-FP threshold. `runtime_fetch_1` misses by 0.022 cosine — it's the rank-2 most-anomalous member of the flagged set.
2. `error_flip_3` is at the 64th percentile — genuinely typical, not anomalous. The earlier Phase 6.2 percentile 0.656 confirms; that part was honest.
3. `error_flip_2` cannot be caught by this stage at all because its cluster has < 5 controls. The Phase 6.2 distance of 0.5331 came from a centroid built on 2 samples and should be discounted as noise.

---

## Task 5 — Recall + FP audit per corpus (Stage-4 only)

| Corpus | FP target | Threshold | Breaks (total / scored / caught) | Stage-4 recall | Actual FP % | FP regression vs baseline |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 0.5689 | 32 / 32 / 0 | 0.0 % | 0.690 % | +0.09 pp |
| rich     | 1.2 % | 0.5401 | 16 / 16 / 0 | 0.0 % | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 0.4445 | 16 / 16 / 3 | 18.75 % | 2.290 % | +0.29 pp |
| hono     | 0.5 % | 0.5991 | 17 / 17 / 1 | 5.88 % | 0.383 % | −0.12 pp |
| ink      | 0.5 % | 0.5358 | 17 / 17 / 3 | 17.65 % | 0.746 % | +0.25 pp |
| faker-js | 0.9 % | 0.4892 | 17 / 15 / 2 | 11.76 % | 0.826 % | −0.07 pp |

**No-regression gate (per-corpus FP ≤ baseline + 0.5 pp): PASS on all 6/6.**

**Stage-4 catalog recall added: 9 / 115 = 7.83 %** (counting all corpora).

The largest stage-4 contribution is on `faker` (3 catches at +0.3 pp FP) and `ink` (3 catches at +0.25 pp FP). Both are corpora where era-11 already covers most of the catalog — the embedding-anomaly stage adds a few new finds at acceptable cost. On fastapi and rich the stage adds zero recall; the controls and breaks aren't separable on this single feature in those corpora at the calibrated threshold.

The 2 caught faker-js breaks are NOT in the residual set — they're fixtures era-11 already catches via stage 1-3. Of the 17 faker-js breaks, 12 are catalog non-residuals (error/runtime/threading kinds with `n_unattested_callees > 0`), 5 are residuals; this stage caught `runtime_fetch_2` (one residual) plus 1 non-residual. So the residual contribution is exactly 1.

---

## Task 6 — Faker-js diagnostic (top control distances)

Top-20 faker-js controls sorted by `embedding_distance` descending.

| Rank | File | Cluster | Distance | Above threshold? |
|---:|---|---:|---:|:---:|
| 1  | `src/modules/date/index.ts` | 2 | 0.5112 | **✓** |
| 2  | `src/modules/helpers/index.ts` | 0 | 0.4892 | ✗ (= threshold) |
| 3  | `src/faker.ts` | 0 | 0.4892 | ✗ (= threshold) |
| 4  | `cypress/e2e/guide.cy.ts` | 6 | 0.4548 | ✗ |
| 5  | `src/modules/finance/index.ts` | 2 | 0.4477 | ✗ |
| 6  | `src/modules/internet/index.ts` | 4 | 0.4197 | ✗ |
| 7  | `src/modules/finance/index.ts` | 2 | 0.4151 | ✗ |
| 8  | `cypress/e2e/api.cy.ts` | 7 | 0.4122 | ✗ |
| 9  | `src/modules/person/index.ts` | 6 | 0.4102 | ✗ |
| 10 | `src/internal/base64.ts` | 4 | 0.3978 | ✗ |
| 11 | `src/modules/string/index.ts` | 7 | 0.3974 | ✗ |
| 12 | `src/modules/color/index.ts` | 7 | 0.3959 | ✗ |
| 13 | `src/internal/keys.ts` | 4 | 0.3924 | ✗ |
| 14 | `src/modules/word/index.ts` | 2 | 0.3923 | ✗ |
| 15 | `src/modules/color/index.ts` | 7 | 0.3760 | ✗ |
| 16 | `src/modules/helpers/eval.ts` | 7 | 0.3738 | ✗ |
| 17 | `src/locale/en_BORK.ts` | 1 | 0.3696 | ✗ |
| 18 | `src/modules/finance/index.ts` | 2 | 0.3683 | ✗ |
| 19 | `src/modules/finance/index.ts` | 2 | 0.3622 | ✗ |
| 20 | `src/modules/color/index.ts` | 7 | 0.3613 | ✗ |

**The single flagged control is `src/modules/date/index.ts` — a regular provider module, not a locale-data file or e2e test.** That's a real false positive on production code, not a known throwaway. Two e2e cypress files appear in the top-20 (ranks 4 and 8) but sit comfortably below threshold; locale data (`en_BORK.ts`) shows up at rank 17.

The threshold lands awkwardly close to several rank-1 / rank-2 / rank-3 controls (0.5112, 0.4892, 0.4892 — two are tied with the threshold itself). Tiny perturbations in the calibration set would shift which side of the line they fall on. This is consistent with the small fjs scorable-control sample (n=121) — the tail is very noisy.

---

## Task 7 — Verdict

| Pre-registered condition | Result | Pass |
|---|---|:---:|
| ≥ 2 of 5 residual catches at faker-js FP ≤ 0.9 % | 1 / 5 (only `runtime_fetch_2`) | ✗ |
| Per-corpus FP ≤ baseline + 0.5 pp on every corpus | max regression +0.29 pp (faker) | ✓ |
| Stage-4 adds catalog recall, never strips existing era-11 catches | +9 / 115 catalog catches added; era-11 catches preserved by definition (stage 4 is OPT-IN OR'd on top of stages 1-3) | ✓ |

**VERDICT: PARTIAL.** Pre-registered SHIP gate fails (1 < 2). Pre-registered CLOSE NEGATIVE doesn't apply (we have 1 ≥ 1). The orchestrator decision tree calls this out as PARTIAL.

### What the result actually says

- The Phase 6.2 STRONG SIGNAL of "4/5 residuals above p90" was overstated by trusting a 2-sample centroid for `error_flip_2`. With proper minimum-population filtering, the honest count is 3/5 above p90, and only 1/5 above the 0.9 %-FP calibrated threshold.
- The other 3 residuals (`runtime_fetch_1`, `runtime_fetch_2`, `runtime_fetch_3`) all sit in the top 5 % of fjs control distances — embeddings ARE picking them up as anomalous. The signal is real but it is NOT separated cleanly enough from the control tail to support a 0.9 %-FP threshold. `runtime_fetch_1` misses by 0.022 cosine.
- `error_flip_2` cannot be caught by this stage at all because its cluster has only 2 controls. Catching it would require either (a) merging tiny clusters into a corpus-level centroid, (b) sourcing additional control samples for cluster 3, or (c) lowering `MIN_CLUSTER_CONTROLS` to 2 — all of which trade off centroid trustworthiness against recall.
- `error_flip_3` is genuinely typical-looking under this metric (64th percentile, distance 0.314 vs threshold 0.489). No reasonable threshold tweak catches it without exploding FP.

### Stage-4 as a complement to era-11

Stage 4 adds 7.83 % catalog recall (9 / 115) at sub-baseline-+-0.5pp FP cost. That is a non-zero contribution. On a combined era-11 + stage-4 system it would push average recall from 89.97 % toward roughly 92 % — a real but modest gain. On the residual problem (the only thing era 14 was meant to solve), it gains exactly 1.

### Comparison vs Phase 6.2 / 6.3

| Phase | Method | Faker-js residual catch (out of 5) |
|---|---|---:|
| 6.2 (probe, 1-control centroid OK) | centroid distance, p90 cut | 4 |
| 6.2 (probe, ≥5-control centroid) | centroid distance, p90 cut | 3 (excludes `error_flip_2`) |
| 6.3 (supervised) | various LOO classifiers | 0 (1 from engineered-only) |
| 6.4 (this) | unsupervised, 0.9 %-FP threshold | 1 (`runtime_fetch_2`) |

Phase 6.4 is the most honest number we have: real centroids, real per-corpus calibration, no label leak. The price for honesty is that 4 of 5 residuals don't quite clear the bar.

---

## What this implies for era 14

The pre-reg verdict is PARTIAL. Three orchestrator options:

1. **Ship anyway as opt-in stage 4.** Adds +9 catalog catches across corpora at sub-0.5-pp FP regression. One residual catch (`runtime_fetch_2`) is something era-11 cannot do. Modest but real.
2. **Tighten or modify centroid construction.** The biggest leverage point is the `MIN_CLUSTER_CONTROLS = 5` filter excluding `error_flip_2` — its cluster has only 2 controls. Any reformulation that gives this hunk a centroid (e.g. fall back to corpus-wide centroid for low-pop clusters, or pool clusters across files) potentially recovers 1 more catch but introduces a new tunable knob.
3. **Close era 14.** SHIP gate failed; the embeddings did not deliver the residual catch the era was designed to test. Two passes (Phase 5 conservative XGBoost and Phase 6.3 LOO classifiers) caught 1/5 between them; this pass adds 1 more (different residual). After three swings, the residual problem looks structurally hard rather than under-modeled.

A modest scope tweak to test before closing: reproduce this analysis with a fallback centroid (corpus-wide mean of controls) for hunks whose cluster has < 5 controls. If that single change moves residual catch to ≥ 2 without breaking the no-regression gate, ship as stage 4. If not, close era 14 with the result documented.
