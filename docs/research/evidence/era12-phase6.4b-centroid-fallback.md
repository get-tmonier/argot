# Era 12 Phase 6.4b — Unsupervised centroid scoring with corpus-wide fallback

**Date**: 2026-05-03
**Branch**: `feat/era-12-ml-stage`
**Script**: `engine/scripts/era12_phase64b_centroid_fallback.py`
**Inputs**: `engine/.era12-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` (1891 rows; 115 breaks, 1776 controls)
**Persisted artifacts**:
- Centroid dict (cluster + corpus-wide): `engine/.era12-features/centroids_phase6.4b.joblib`
- Raw results JSON: `/tmp/era12_phase64b_results.json`

---

## TL;DR

**VERDICT: CLOSE NEGATIVE.** Adding a corpus-wide fallback for unmappable / low-population clusters strictly *worsens* the residual catch on faker-js (0 / 5 vs 6.4's 1 / 5) while leaving the cross-corpus stage-4 catalog recall unchanged for the corpora that contributed catches. The no-regression FP gate still passes everywhere.

The intuition "give `error_flip_2` a centroid and you'll catch one more residual" was correct as far as it goes — `error_flip_2` is now scored (distance 0.360) — but it scores at the 63rd percentile, far below threshold. Meanwhile, scoring the 173 previously-excluded faker-js unmappable controls under the fallback adds many *high*-distance items to the calibration tail (the rank-1 and rank-2 fjs controls under 6.4b are both `src/locales/ur/finance/account_type.ts` at distance 0.5217 — locale-data files that look very unlike the corpus-wide centroid). This pushes the 0.9 %-FP threshold up from **0.4892 → 0.5104**, and `runtime_fetch_2` (distance 0.4931, which barely cleared in 6.4) now misses by 0.017 cosine.

Net: zero residual catches, slightly noisier thresholds elsewhere, no improvement to ship.

---

## Setup recap

Same data as Phase 6.4. Method differs only at the centroid-routing step:

- For each `(corpus, cluster_id)` with **≥ 5 controls and `cluster_id ≠ -1`**: build a cluster centroid from CONTROLS (same as 6.4).
- For each corpus: also build a **corpus-wide fallback centroid** = L2-normed mean of ALL that corpus's CONTROL hunk embeddings (including unmappable rows). Unsupervised — no catalog labels touch centroid construction.
- At scoring: route to cluster centroid if available; else use corpus-wide fallback.
- Per-corpus threshold = `(1 − FP_target/100)`-quantile of CONTROL distances under the new fallback rule (so calibration and scoring use the same routing rule).

Pre-registered gates (carried forward from 6.4):
- SHIP: ≥ 2/5 faker-js residuals catch at faker-js FP ≤ 0.9 %, AND per-corpus FP ≤ baseline + 0.5 pp on all 6 corpora.
- CLOSE NEGATIVE: ≤ 1/5 residuals catch even with the fallback.

---

## Task 1 — Centroid construction

| Corpus | Cluster centroids (≥5 ctrl) | Low-pop clusters (now via fallback) | Unmappable rows (now via fallback) | Has corpus centroid? |
|---|---:|---:|---:|:---:|
| fastapi  | 8 | 0 | 5   | ✓ |
| rich     | 7 | 0 | 0   | ✓ |
| faker    | 5 | 2 | 30  | ✓ |
| hono     | 7 | 0 | 36  | ✓ |
| ink      | 5 | 1 | 20  | ✓ |
| faker-js | 6 | 2 | 173 | ✓ |

**38 cluster centroids built (identical to 6.4); + 6 corpus-wide fallback centroids.**

---

## Task 2 — Per-hunk routing under fallback

| Corpus | Total | Via cluster | Via fallback | Excluded | Breaks via cluster / fallback / excl | Controls via cluster / fallback / excl |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 327 | 322 | 5   | 0 | 32/0/0  | 290/5/0   |
| rich     | 316 | 316 | 0   | 0 | 16/0/0  | 300/0/0   |
| faker    | 313 | 278 | 35  | 0 | 16/0/0  | 262/35/0  |
| hono     | 314 | 278 | 36  | 0 | 17/0/0  | 261/36/0  |
| ink      | 306 | 285 | 21  | 0 | 17/0/0  | 268/21/0  |
| faker-js | 315 | 136 | 179 | 0 | 15/2/0  | 121/177/0 |

**Coverage now 100 % on every corpus.** All 17 faker-js breaks are scored (vs 15 in 6.4). The 2 newly-scored fjs breaks both route through the corpus-wide fallback — this is the `error_flip_2` recovery the phase was designed to test, plus one other.

---

## Task 3 — Per-corpus thresholds (re-calibrated under new rule)

| Corpus | FP target | Threshold 6.4 | Threshold 6.4b | Δ threshold | Controls (n) | Controls flagged | Actual FP % |
|---|---:|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 0.5689 | 0.5656 | −0.003 | 295 | 2 | 0.678 % |
| rich     | 1.2 % | 0.5401 | 0.5401 | 0      | 300 | 4 | 1.333 % |
| faker    | 2.0 % | 0.4445 | 0.4441 | −0.000 | 297 | 6 | 2.020 % |
| hono     | 0.5 % | 0.5991 | 0.6325 | **+0.033** | 297 | 1 | 0.337 % |
| ink      | 0.5 % | 0.5358 | 0.5321 | −0.004 | 289 | 2 | 0.692 % |
| faker-js | 0.9 % | 0.4892 | **0.5104** | **+0.021** | 298 | 3 | 1.007 % |

Two corpora see meaningful threshold inflation: hono (+0.033 cosine) and faker-js (+0.021 cosine). Both are corpora with high unmappable-row counts (36 / 297 = 12 % for hono, 173 / 298 = 58 % for fjs). Adding those rows to the calibration set drags the upper tail farther from the centroid, so the 1 − FP/100 quantile shifts up.

Actual FP rates remain within ±0.2 pp of target on every corpus → no-regression gate **passes**.

---

## Task 4 — Residual fixture catch (decisive test)

Apply faker-js threshold (0.5104) to the 5 residuals.

| Fixture | 6.4 distance | 6.4 caught? | 6.4b distance | Route (6.4b) | 6.4b threshold | 6.4b caught? | Top-X% (6.4b) |
|---|---:|:---:|---:|---|---:|:---:|---:|
| `error_flip_2`   | excluded | — | 0.3601 | corpus_fallback | 0.5104 | ✗ | top 36.6 % |
| `error_flip_3`   | 0.3138 | ✗ | 0.3138 | cluster | 0.5104 | ✗ | top 67.8 % |
| `runtime_fetch_1`| 0.4670 | ✗ | 0.4670 | cluster | 0.5104 | ✗ | top 4.4 % |
| `runtime_fetch_2`| 0.4931 | **✓** | 0.4931 | cluster | 0.5104 | ✗ | top 1.7 % |
| `runtime_fetch_3`| 0.4248 | ✗ | 0.4248 | cluster | 0.5104 | ✗ | top 11.7 % |

**Catch count: 0 of 5.** SHIP gate fails. CLOSE NEGATIVE triggers.

Three observations:

1. `error_flip_2` IS now scored (the change worked mechanically), but its distance to the corpus-wide fjs centroid is 0.36 — it sits at the 63rd percentile of fjs controls. It is *not* an embedding-anomalous hunk against the broader faker-js corpus. The Phase 6.2 reading of 0.5331 on a 2-control centroid was not just statistically unreliable, it was directionally misleading: with proper data, this hunk looks typical.
2. `runtime_fetch_2` is the casualty of recalibration. Its score is unchanged (0.4931) — same cluster centroid it had before — but the fjs threshold rose to 0.5104 because 5 unmappable controls in `src/locales/...` files now sit higher than 0.5 against the corpus-wide centroid. We lost the one residual catch we had.
3. The other 3 cluster-routed residuals are unchanged: `runtime_fetch_1` still misses by 0.044 cosine, `runtime_fetch_3` by 0.086, `error_flip_3` is still at the 32nd percentile.

---

## Task 5 — Per-corpus recall + FP audit

| Corpus | FP target | 6.4b threshold | Breaks (total/scored/caught) | Stage-4 recall (of total) | Actual FP % | Regression vs baseline |
|---|---:|---:|---:|---:|---:|---:|
| fastapi  | 0.6 % | 0.5656 | 32 / 32 / 0 | 0.0 %   | 0.678 % | +0.08 pp |
| rich     | 1.2 % | 0.5401 | 16 / 16 / 0 | 0.0 %   | 1.333 % | +0.13 pp |
| faker    | 2.0 % | 0.4441 | 16 / 16 / 3 | 18.75 % | 2.020 % | +0.02 pp |
| hono     | 0.5 % | 0.6325 | 17 / 17 / 0 | 0.0 %   | 0.337 % | −0.16 pp |
| ink      | 0.5 % | 0.5321 | 17 / 17 / 3 | 17.65 % | 0.692 % | +0.19 pp |
| faker-js | 0.9 % | 0.5104 | 17 / 17 / 0 | 0.0 %   | 1.007 % | +0.11 pp |

**No-regression gate (per-corpus FP ≤ baseline + 0.5 pp): PASS on all 6/6.**

**Stage-4 catalog recall: 6 / 115 = 5.22 %** (vs 6.4's 9 / 115 = 7.83 %).

The fallback strictly *loses* recall on hono (5.88 % → 0 %) and faker-js (11.76 % → 0 %), the two corpora whose threshold rose. faker, ink stayed flat (no fallback effect on their breaks since all of them route through clusters).

---

## Task 6 — Side-by-side 6.4 vs 6.4b

| Corpus | 6.4 catches | 6.4b catches | Δ catches | 6.4 FP | 6.4b FP | Δ FP | 6.4 stage-4 recall | 6.4b stage-4 recall | Δ recall |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| fastapi  | 0 | 0 | 0  | 0.690 % | 0.678 % | −0.01 | 0.00 %  | 0.00 %  | 0      |
| rich     | 0 | 0 | 0  | 1.333 % | 1.333 % |  0.00 | 0.00 %  | 0.00 %  | 0      |
| faker    | 3 | 3 | 0  | 2.290 % | 2.020 % | −0.27 | 18.75 % | 18.75 % | 0      |
| hono     | 1 | 0 | **−1** | 0.383 % | 0.337 % | −0.05 | 5.88 %  | 0.00 %  | **−5.88 pp** |
| ink      | 3 | 3 | 0  | 0.746 % | 0.692 % | −0.05 | 17.65 % | 17.65 % | 0      |
| faker-js | 2 | 0 | **−2** | 0.826 % | 1.007 % | +0.18 | 11.76 % | 0.00 %  | **−11.76 pp** |

Strict regressions vs 6.4: **hono** (lost 1 catch) and **faker-js** (lost 2 catches: `runtime_fetch_2` residual AND 1 non-residual). FP rates moved within noise on every corpus.

The mechanism is the same in both regression cases: the corpus has many unmappable rows whose distances to the corpus-wide centroid are atypically large (hono: 36 unmappable; fjs: 173). Adding them to the calibration tail pushes the per-corpus threshold up, and the previously-flagged breaks no longer clear the new bar.

---

## Task 7 — Verdict

| Pre-registered condition | Result | Pass |
|---|---|:---:|
| ≥ 2 of 5 residual catches at faker-js FP ≤ 0.9 % | 0 / 5 | ✗ |
| Per-corpus FP ≤ baseline + 0.5 pp on every corpus | max +0.19 pp (ink) | ✓ |
| 6.4b strictly improves on 6.4 | regressed on hono and faker-js | ✗ |

**VERDICT: CLOSE NEGATIVE.** The fallback methodology loses ground on the corpus that matters most.

### What this result actually says

- The "structurally bounded at 4/5" framing of Phase 6.4 was *technically* accurate (the MIN_CLUSTER_CONTROLS filter did exclude `error_flip_2` from scoring) but practically irrelevant. With proper scoring, `error_flip_2` is at the 63rd percentile of its corpus — not anomalous. Phase 6.2's distance of 0.53 against a 2-control centroid was not measuring anything real; it was small-sample noise pointing in a vaguely correct direction.
- The fallback's unintended consequence — recalibrating the threshold against 173 newly-scored unmappable controls — was the dominant effect on faker-js. Locale data files in `src/locales/...` are the most distinctive thing in faker-js and they are *not* breaks. Including them in the calibration tail raises the bar for everything else.
- The signal that exists on the runtime_fetch residuals (top 1.7 %, 4.4 %, 11.7 % of fjs controls) is genuinely there but cannot be separated cleanly from the locale-data tail at a 0.9 %-FP threshold under any centroid scheme tested in era 12.

### Implication for era 12

After Phases 6.2 → 6.3 → 6.4 → 6.4b, the embedding-anomaly stage has converged on a clear ceiling:

| Phase | Method | fjs residual catch (out of 5) |
|---|---|---:|
| 6.2 (probe, 1-control centroid OK) | centroid distance, p90 cut | 4 (inflated by 2-sample centroid) |
| 6.2 honest (≥5-control filter) | same | 3 |
| 6.3 (supervised) | LOO classifiers | 0 (1 from engineered-only) |
| 6.4 (unsupervised, calibrated) | per-cluster centroids, 0.9 %-FP | 1 (`runtime_fetch_2`) |
| **6.4b (this, with corpus-wide fallback)** | **per-cluster + corpus-wide fallback, 0.9 %-FP** | **0** |

6.4b is the cleanest test of "does giving every hunk a centroid help?" and the answer is *no*: the calibration cost outweighs the coverage gain. The phase 6.4 spec (skip thin clusters) was correct; the missing residual `error_flip_2` is genuinely typical-looking, not under-modeled.

**Recommendation**: close era 12. Phase 6.4 stands as the strongest honest result (1/5 residuals, +9 catalog catches, all FP gates pass). Either ship 6.4 as opt-in stage 4 for the modest catalog gain, or close with the 4-phase residual investigation documented and accept that the residual problem is not solvable with single-feature embedding distances at the FP budget era 11 sets.

Do not pursue further centroid variants — the experiments converge.
