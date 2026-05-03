# Era 14 — Fix-A FULL analysis (all 6 corpora, post host-injection)

**Date**: 2026-05-03
**Branch**: `docs/era-10-root-readme` (analysis-only; no code changes)
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` — 1891 rows total (115 breaks + 1776 controls), all six corpora freshly extracted with Fix-A (`host_file` injection on all catalog fixtures).
**Reference**: pilot memo [`era14-fixA-pilot.md`](era14-fixA-pilot.md), status [`era14-status.md`](era14-status.md)
**Hyperparameters**: pre-registered Phase-3 XGBoost (`n_estimators=100, max_depth=4, lr=0.1, random_state=0`); no tuning.
**Code**: `/tmp/era14_fixA_full.py` (one-shot)
**Persisted**: `/tmp/era14_fixA_full_results.json`
**Models saved**: `.era14-features/{pooled_full_fixA,pooled_conservative_fixA}.joblib`, `.era14-features/loo_models_fixA/{corpus}.joblib` (6)

---

## TL;DR

| Test | Pre-reg gate | Result | Pass |
|---|---|---|---|
| Max single-feature pooled AUC | ≤ 0.85 | **0.886** (`n_unattested_callees`) | ✗ (just over) |
| `hunk_file_callee_jaccard` pooled AUC | ≤ 0.65 | **0.703** (was 0.844 in 3.6b) | ✗ (close) |
| Pooled conservative Set B 5-fold CV AUC | ≥ 0.85 (ship) | **0.9035** | ✓ |
| LOO test AUC ≥ 0.75 across corpora | ≥ 4 of 6 (ship) | **6 of 6** | ✓ |
| Residual catch under faker-js LOO at FP ≤ 0.9% | ≥ 2 of 5 (ship) | **0 of 5** | ✗ |

**Verdict (per pre-registered rules in the task spec): CLOSE NEGATIVE.** The spec is explicit — *"CLOSE NEGATIVE: LOO collapses (≤ 2 of 6 corpora pass) OR residual catch is 0/5 under LOO."* The residual-catch arm of the OR-clause has triggered.

The aggregate-discrimination story looks excellent: 6/6 LOO corpora pass, conservative Set B AUC 0.90, the strongest non-honest leak proxy (`hunk_file_callee_jaccard`) dropped from 0.84 → 0.70 with host injection. But the era's motivating signal — catching the 5 era-11 residual faker-js fixtures when the model has never seen any faker-js training data — is gone. The pilot's 2/5 catch was an in-corpus artefact; the cross-corpus generalisation collapses to 0/5.

The honest read: Fix-A converted the bench from "ML learns shape leak inside one corpus" to "ML learns existing-stage outputs that don't add anything beyond the existing stages on the residuals." The held-out faker-js model's top features (`n_unattested_callees`, `bpe_score`, `adjusted_bpe`, `stage2_flagged`) are exactly the signals stages 2–3 already produce — the ML stage is rediscovering them, not adding new signal.

---

## Setup

| Corpus | Set A total | Set A breaks | Set B total | Set B breaks |
|---|---:|---:|---:|---:|
| fastapi | 327 | 32 | 97 | 3 |
| rich | 316 | 16 | 88 | 1 |
| faker | 313 | 16 | 82 | 1 |
| hono | 314 | 17 | 94 | 1 |
| ink | 306 | 17 | 233 | 1 |
| faker-js | 315 | 17 | 191 | 4 |
| **total** | **1891** | **115** | **785** | **11** |

Set B = Stage-4 operating regime (rows where stages 1–3 returned no flag — `scorer_reason ∈ {None, "none"}`). Faker-js residuals (4 in Set B) are the era-11-uncaught fixtures; one residual (`runtime_fetch_2`) was already flagged by stage 2 so Set B sees 4/5.

---

## Task 1 — Per-feature pooled AUC (top-15)

Pooled across all 1891 rows, raw AUC for `is_break` (auto-flipped). `cluster_id` (categorical) and `ast_node_type_counts` (dict) excluded per task spec.

| Feature | Pooled AUC | vs 3.6b pooled | Note |
|---|---:|---:|---|
| n_unattested_callees | **0.8861** | 0.886 (unchanged) | honest content signal |
| adjusted_bpe | 0.8410 | ~0.93 | dropped (corpus-mix shifted) |
| bpe_score | 0.8394 | 0.833 | unchanged |
| hunk_callee_bag_size | 0.7443 | 0.74 | unchanged |
| n_distinct_callees | 0.7443 | identical | dedup target |
| n_attested_root_only | 0.7355 | — | — |
| **hunk_file_callee_jaccard** | **0.7033** | **0.844** | **−0.14, biggest Fix-A win** |
| max_nesting_depth | 0.6876 | — | down from 0.93 in faker-js pilot |
| n_returns | 0.6717 | — | down from 0.90 in faker-js pilot |
| hunk_callees_in_file_fraction | 0.6443 | — | down materially |
| cluster_jaccard_to_centroid | 0.6216 | 0.51 (3.6b) | up — content signal |
| n_distinct_identifiers | 0.6193 | 0.69 | unchanged |
| parse_fragment_flag | 0.6078 | — | borderline |
| file_callee_bag_size | 0.5836 | — | — |
| stage2_flagged | 0.5831 | — | — |

**Observations:**

1. `hunk_file_callee_jaccard` dropped from **0.844 → 0.703** pooled — Fix-A halved the leak signal, exactly as the pilot predicted directionally. Still above the strict 0.65 gate but no longer a top-3 discriminator.
2. `max_nesting_depth` (the pilot's biggest concern, 0.93 inside faker-js) drops to **0.688 pooled**. The 0.93 was a faker-js peculiarity, not a cross-corpus shape-leak.
3. `n_returns` similarly drops from 0.90 (faker-js alone) to **0.67 pooled**. Confirms the structural-leak features are corpus-specific, not systematic.
4. The top discriminator pooled is now `n_unattested_callees` (**0.886**) — an honest signal that stages 2–3 already exploit. This is the same value Phase 3.6b reported as "honest" before any Fix-A work.
5. Strict pre-reg gate **max single-feature ≤ 0.85** failed at 0.886 — but the failing feature is the existing-stage output, not a leak proxy. The interpretation is "the ML stage will mostly just re-learn what Stage 3 already does," not "the bench is leaky."

**Bottom line on Task 1:** Fix-A worked on the targeted shape-leak proxies (`hunk_file_callee_jaccard`, `max_nesting_depth`, `n_returns`, `hunk_callees_in_file_fraction` all dropped 0.10–0.25 AUC). The remaining single-feature signal is honest content signal already captured by existing stages.

---

## Task 2 — Pooled XGBoost (Full × Conservative) × (Set A × Set B)

5-fold stratified CV. Same XGBoost params throughout.

| Variant | Set | n | breaks | Mean AUC | Std | Folds |
|---|---|---:|---:|---:|---:|---|
| full | A (pooled) | 1891 | 115 | **0.9769** | 0.0072 | [0.966, 0.988, 0.974, 0.978, 0.979] |
| full | B (Stage-4) | 785 | 11 | 0.8913 | 0.0538 | [0.905, 0.890, 0.984, 0.829, 0.848] |
| **conservative** | **A** | **1891** | **115** | **0.9724** | 0.0093 | [0.959, 0.988, 0.973, 0.970, 0.971] |
| **conservative** | **B** | **785** | **11** | **0.9035** | 0.0538 | [0.879, 0.952, 0.977, 0.829, 0.881] |

Both Set B AUCs **comfortably clear the 0.85 ship gate**. The conservative model loses only 0.005 AUC on Set A vs the full model, and *gains* 0.012 on Set B — the dropped features (`cluster_jaccard_to_centroid`, `hunk_length_*`, `n_total_ast_nodes`, `hunk_file_callee_jaccard`, `hunk_callees_in_file_fraction`, AST counts) are not load-bearing for Stage-4 generalisation.

### Top-5 features by gain importance

**Full, Set A** (pooled, all features):
1. `n_unattested_callees` (31.68)
2. `hunk_callees_in_file_fraction` (28.56)
3. `bpe_score` (5.14)
4. `hunk_file_callee_jaccard` (3.95)
5. `n_awaits` (3.77)

**Full, Set B** (Stage-4 regime):
1. `ast_property_identifier` (6.65)
2. `n_throws` (2.34)
3. `adjusted_bpe` (2.01)
4. `max_nesting_depth` (1.92)
5. `hunk_callees_in_file_fraction` (1.88)

**Conservative, Set A**:
1. `n_unattested_callees` (29.06)
2. `adjusted_bpe` (6.66)
3. `bpe_score` (4.32)
4. `n_returns` (2.60)
5. `import_score` (2.30)

**Conservative, Set B**:
1. `max_nesting_depth` (1.99)
2. `adjusted_bpe` (1.59)
3. `cluster_id` (1.57)
4. `parse_fragment_flag` (1.44)
5. `n_throws` (1.25)

The Set A conservative model is dominated by `n_unattested_callees` (~29 gain, 4× the next feature) — the same signal Stage 3 already uses. Set B (where the ML stage actually fires) has flatter importances; `max_nesting_depth` is top, but at gain 2.0 it's only ~1.5× the runner-up — no single dominant feature.

**Pooled models saved:**
- `.era14-features/pooled_full_fixA.joblib` (Set A — full, 26 features)
- `.era14-features/pooled_conservative_fixA.joblib` (Set A — 16 features)

---

## Task 3 — Leave-one-corpus-out (LOO) — THE DECISIVE GENERALISATION TEST

Conservative feature set; train on 5 corpora's pooled rows (Set A), test on the 6th. Same XGB params.

| Held-out corpus | Train rows | Test rows | Test breaks | Test AUC | vs Pooled CV (0.9724) | Pass ≥ 0.75 |
|---|---:|---:|---:|---:|---:|---:|
| fastapi | 1564 | 327 | 32 | **0.9516** | −0.021 | YES |
| rich | 1575 | 316 | 16 | **0.9746** | +0.002 | YES |
| faker | 1578 | 313 | 16 | **0.9806** | +0.008 | YES |
| hono | 1577 | 314 | 17 | **0.9655** | −0.007 | YES |
| ink | 1585 | 306 | 17 | **0.9127** | −0.060 | YES |
| **faker-js** | 1576 | 315 | 17 | **0.7657** | **−0.207** | YES |

**LOO summary: 6/6 corpora pass test AUC ≥ 0.75.** Pre-registered LOO ship gate (≥ 4 of 6) **PASSES** comfortably.

5 of the 6 corpora come in within 0.06 AUC of the pooled CV estimate — strong evidence of cross-corpus generalisation for the bulk of the data. Faker-js is the outlier (test AUC 0.766, ~0.21 below pooled), which makes sense: faker-js holds the era-11 *residual* fixtures, the hardest cases by construction. Even so, 0.766 clears the 0.75 bar.

LOO models saved per held-out corpus under `.era14-features/loo_models_fixA/{corpus}.joblib`.

---

## Task 4 — Residual fixture catch under faker-js LOO — THE OTHER KILLER TEST

The model trained on the **other 5 corpora** (1576 rows, no faker-js exposure) predicts on the held-out 315 faker-js rows. Threshold for FP ≤ 0.9% on the 298 faker-js controls is **0.826** (actual achieved FP 0.67%, with 2/298 controls strictly above).

| Residual fixture | LOO predicted prob | Rank vs 298 controls | FP rate at this prob | Caught at FP ≤ 0.9%? |
|---|---:|---:|---:|---:|
| faker_js_error_flip_2 | 0.5155 | 17/298 above | 5.70% | **NO** |
| faker_js_error_flip_3 | 0.0110 | 161/298 above | 54.03% | **NO** |
| faker_js_runtime_fetch_1 | 0.0014 | 217/298 above | 72.82% | **NO** |
| faker_js_runtime_fetch_2 | 0.0047 | 181/298 above | 60.74% | **NO** |
| faker_js_runtime_fetch_3 | 0.0099 | 167/298 above | 56.04% | **NO** |

**Catch count: 0 of 5.**

Pre-registered residual gate (≥ 2 of 5): **FAIL.**

For context: in the in-corpus pilot, `runtime_fetch_1` had OOF prob 0.78 (rank 0/298). Under LOO it falls to **0.0014** (rank 217/298). The ~0.78 prob the model assigned within faker-js was almost entirely driven by features the model could only learn *with faker-js training data present* — exactly the in-corpus shape signal Fix-A was hoped to remove.

`error_flip_2` is the closest call (rank 17/298, FP 5.7%) — at a much looser threshold (FP ≤ 6%) it would catch, but that's far above the era-11 baseline (0.9%). The other four residuals sit in the bottom 30% of faker-js predictions; they are indistinguishable from noise to the LOO model.

---

## Task 5 — VERDICT: CLOSE NEGATIVE

Pre-registered decision tree (verbatim from task spec):

> - **SHIP**: pooled conservative Set B AUC ≥ 0.85 AND LOO gate passes AND residual catch ≥ 2 of 5 under LOO.
> - **PARTIAL**: LOO passes but residual catch is 0–1, OR LOO passes 3 of 6 corpora.
> - **CLOSE NEGATIVE**: LOO collapses (≤ 2 of 6 corpora pass) OR **residual catch is 0/5 under LOO.**

| Pre-reg condition | Required | Observed | Status |
|---|---|---|---|
| Pooled conservative Set B AUC | ≥ 0.85 | 0.9035 | ✓ |
| LOO test AUC ≥ 0.75 corpora | ≥ 4 of 6 | 6 of 6 | ✓ |
| Residual catch under faker-js LOO | ≥ 2 of 5 | **0 of 5** | ✗ |

The residual catch arm of the CLOSE-NEGATIVE OR-clause has triggered: *"residual catch is 0/5 under LOO."* The verdict is **CLOSE NEGATIVE** by the pre-registered rules, even though the aggregate discrimination story is excellent.

### Why aggregate metrics look great while the era's motivating signal collapses

- LOO 5/6 corpora at AUC 0.91–0.98 means the model **does** transfer cross-corpus on the *general* break-vs-control distinction. That's a real result and worth recording.
- The residual fixtures are, by construction, the era-11 cases that look most like benign code — callees attested in their cluster, stages 2–3 silent. The aggregate model's main signals (`n_unattested_callees`, `adjusted_bpe`, `bpe_score`, `n_returns`) are precisely the signals that are *quiet* on the residuals.
- The 2/5 catch in the in-corpus pilot was driven by per-corpus shape features (`max_nesting_depth` 0.93, `n_returns` 0.90 *within faker-js alone*). When the model trains on the other 5 corpora, those per-corpus shape signatures don't exist in the training distribution, so the model has nothing to fire on for the residuals.

### Decision rationale

The pre-reg rule is hard: residual=0/5 → CLOSE NEGATIVE. Two arguments for sticking with that ruling rather than relabelling as PARTIAL:

1. **Era 14's motivating question** was exactly "can ML catch the 5 residual faker-js fixtures era 11 misses?" The honest answer here is **no, not under cross-corpus training** — and cross-corpus training is the only setup that doesn't leak. The aggregate AUC story doesn't change that.
2. **Productionising for "general" break-vs-control discrimination is a different era**, not era 14. The conservative pooled model (Set A AUC 0.97, Set B AUC 0.90, LOO 6/6) might be useful as a generic 4th-stage "this hunk doesn't look like the corpus" signal — but that's a re-scoping, not a ship of era 14 as specified.

**Recommendation: close era 14 negative and document the conservative-model-as-general-discriminator finding as a follow-up era candidate.** The Fix-A work is real and worth keeping in the branch — the data is materially less leaky than 3.6b, and the LOO infrastructure built here is reusable. But the era-14 ship criteria are not met.

---

## Task 6 — Top-5 features in the held-out faker-js LOO model (conservative)

(Reported because the task spec asked for it on SHIP/PARTIAL outcomes; included here even on CLOSE NEGATIVE for the diagnostic value.)

| Rank | Feature | Gain importance | Type |
|---:|---|---:|---|
| 1 | n_unattested_callees | 22.06 | existing-stage output (Stage 3 input) |
| 2 | bpe_score | 6.06 | existing-stage output (Stage 1) |
| 3 | adjusted_bpe | 5.01 | existing-stage output (Stage 1 derivative) |
| 4 | stage2_flagged | 1.58 | existing-stage output (Stage 2 binary) |
| 5 | n_attested_root_only | 1.50 | existing-stage derivative |

**The held-out faker-js model is rediscovering existing-stage outputs.** No AST-shape feature (`max_nesting_depth`, `n_returns`, `n_throws`, `n_awaits`, `parse_fragment_flag`) appears in the top 5 — those features were the strongest signals *inside faker-js alone* in the pilot, but cross-corpus training shows they don't generalise. Top-1 importance dominates by 4× over top-2: this model is essentially a non-linear re-weighting of `n_unattested_callees`.

This explains the residual collapse cleanly: the residual fixtures are exactly the cases where `n_unattested_callees == 0` (callees attested in their cluster — that's why era 11 missed them). A model that mostly fires on `n_unattested_callees` cannot catch them.

---

## What worked / what didn't

**Worked:**
- Host-file injection (Fix-A) materially reduced the targeted shape-leak: `hunk_file_callee_jaccard` 0.844 → 0.703 pooled, `max_nesting_depth` 0.93 (faker-js alone) → 0.69 pooled, `n_returns` 0.90 → 0.67 pooled. The intervention attacked the right variables.
- Cross-corpus generalisation is real for the general break-vs-control task: 6/6 LOO corpora at AUC ≥ 0.75; 5/6 within 0.06 AUC of pooled.
- Conservative feature set is robust: drops only 0.005 Set A AUC vs full, *adds* 0.012 to Set B.

**Didn't work:**
- Residual catch under LOO collapses to 0/5 (the era's motivating signal).
- The model that generalises cross-corpus is one that mostly re-learns `n_unattested_callees` — i.e., re-learns Stage 3. It adds no incremental value on the cases Stage 3 misses.
- The pilot's 2/5 catch was driven by features that don't survive LOO (`max_nesting_depth`, `n_returns` in particular).

---

## What I couldn't analyse (and why)

- **No threshold sweep beyond FP ≤ 0.9%.** The pre-reg gate is a fixed FP rate; reporting catch at looser thresholds (e.g., 5%) would muddy the era-14 ship story. (For the record: at FP ≤ 6%, `error_flip_2` would catch — 1/5.)
- **No per-feature LOO ablation.** Could quantify how much each existing-stage output contributes to the held-out faker-js AUC of 0.766, but this is descriptive and doesn't change the verdict. Worth doing if the orchestrator chooses to re-scope to "general 4th-stage discriminator" follow-up.
- **No comparison against a "Stage-3-only" baseline.** If `n_unattested_callees` alone (single-feature predictor) scored the held-out faker-js residuals at the same rank as the LOO model, the ML stage adds zero. Quick spot-check from Task 1 numbers: `n_unattested_callees` pooled AUC 0.886 vs LOO faker-js 0.766 — the LOO model is *worse* than the single-feature predictor on faker-js. Strong suggestion that the ML stage is not adding signal on the held-out corpus.
- **No re-extraction.** Took the JSONLs as given (1891 rows). Confirmed counts match the task spec (115 breaks + 1776 controls).

---

## Outputs

- This memo: `docs/research/evidence/era14-fixA-full.md`
- Analysis script: `/tmp/era14_fixA_full.py`
- Persisted results JSON: `/tmp/era14_fixA_full_results.json`
- Models: `.era14-features/pooled_full_fixA.joblib`, `.era14-features/pooled_conservative_fixA.joblib`, `.era14-features/loo_models_fixA/{fastapi,rich,faker,hono,ink,faker-js}.joblib`
- Inputs (unchanged): `engine/.era14-features/{corpus}.jsonl` × 6
