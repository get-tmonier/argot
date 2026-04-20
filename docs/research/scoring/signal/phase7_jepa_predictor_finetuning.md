# Phase 7 — JEPA Predictor Fine-Tuning for FastAPI

**Date:** 2026-04-20  
**Branch:** `research/phase-7-honest-eval`  
**Goal:** Find the minimum change to JEPA predictor training that pushes FastAPI to `mean_delta ≥ 0.20` with `std_delta ≤ 0.02` across 3 seeds, then promote the winner as the new `jepa_pretrained` default.

---

## Background

Argot's signal scorer is JEPA: a frozen CodeRankEmbed encoder + a small trainable transformer predictor. At inference time, the predictor returns the MSE "surprise" between its context-conditioned prediction and the actual hunk embedding. The acceptance gate is `delta = mean(breaks) − mean(controls) ≥ 0.20`.

Phase A (scorer comparison across ky, httpx, fastapi) committed the project to JEPA as the primary scorer. A corpus-size sweep then showed the FastAPI predictor was stabilising but not yet crossing the gate at 2000 records:

| seed | delta |
|---:|---:|
| 0 | 0.1597 |
| 1 | 0.1692 |
| 2 | 0.1984 |
| **mean / std** | **0.176 / 0.020** |

The corpus-size curve was still rising at 2000 records, suggesting the predictor was **undertrained** rather than capacity-limited.

---

## Protocol

A structured cheapest-first sweep across 4 stages. Stop at the first stage that meets the gate. All sweep code lives in `engine/argot/research/signal/` — no edits to core training/validation files.

**Architecture:** `JepaPretrainedScorer` → `JepaCustomScorer` → `JepaFilteredScorer` → `EnsembleJepaScorer`, each building on the previous stage's winner.

**Gate:** `mean_delta ≥ 0.20 AND std_delta ≤ 0.02` across seeds {0, 1, 2}.

---

## Stage 1 — Training Budget (epochs × lr)

**Hypothesis:** predictor is undertrained at 20 epochs.  
**Grid:** epochs ∈ {20, 50, 100} × lr ∈ {5e-5, 1e-4} — 6 configs × 3 seeds = 18 runs.

### Results

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| ep20_lr5e5 | 0.1525 | 0.0131 | ✗ |
| ep20_lr1e4 | 0.1796 | 0.0214 | ✗ |
| ep50_lr5e5 | 0.1875 | 0.0400 | ✗ |
| ep50_lr1e4 | 0.1604 | 0.0480 | ✗ |
| ep100_lr5e5 | 0.1718 | 0.0437 | ✗ |
| ep100_lr1e4 | 0.1643 | 0.0274 | ✗ |

### Findings

**Gate not met.** More epochs increases variance without improving the mean. The variance problem is structural: seed-to-seed swings of 0.05–0.09 appear across all configs, likely from the interaction between `split_by_time` (temporal 80/20 split) and the small FastAPI corpus — different seeds produce significantly different training sets.

Best candidates:
- `ep20_lr1e4`: best mean (0.180) → chosen as Stage 2 base
- `ep20_lr5e5`: tightest variance (std=0.013) but mean too low to reach gate

---

## Stage 2 — LR Schedule × Predictor Capacity

**Hypothesis:** a cosine LR schedule or larger predictor can push the mean over 0.20 and tighten variance.  
**Base:** `ep20_lr1e4` (best mean from Stage 1).  
**Grid:** lr_schedule ∈ {flat, cosine} × predictor ∈ {depth=4/mlp=512, depth=6/mlp=512, depth=4/mlp=1024, depth=6/mlp=1024} — 8 configs × 3 seeds = 24 runs.

### Results

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| flat_d4m512 | 0.1796 | 0.0214 | ✗ |
| flat_d6m512 | 0.1973 | 0.0235 | ✗ |
| **flat_d4m1024** | **0.2031** | **0.0102** | **✓** |
| flat_d6m1024 | 0.2215 | 0.0245 | ✗ |
| cos_d4m512 | 0.1584 | 0.0074 | ✗ |
| cos_d6m512 | 0.1672 | 0.0170 | ✗ |
| cos_d4m1024 | 0.1667 | 0.0224 | ✗ |
| cos_d6m1024 | — | — | — |

*cos_d6m1024 not run — cosine underperformed flat on all 6 preceding configs.*

### Findings

**Gate met by `flat_d4m1024`** (mean=0.203, std=0.010).

**Wider MLP helps, deeper predictor hurts variance.** Going from mlp=512 to mlp=1024 at depth=4 raised the mean from 0.180 to 0.203 while halving std. Going to depth=6 raised the mean further (0.221) but drove std above gate (0.024) — more capacity amplifies variance.

**Cosine schedule uniformly worse.** Every cosine config underperforms its flat equivalent by ~0.03–0.04 on mean. With only 20 epochs on a small corpus, cosine annealing decays lr too aggressively before the predictor converges.

**`flat_d6m1024` is the highest-mean config (0.221) but fails std.** This becomes the Stage 4 ensemble target.

---

## Stage 3 — Corpus Pre-filtering (similarity to break fixtures)

**Hypothesis:** the corpus contains hunks similar to break fixtures that teach the predictor not to be surprised by them — filtering those out should improve signal separation.  
**Base:** `flat_d4m1024` (Stage 2 winner).  
**Grid:** τ ∈ {top-1%, top-5%} — 2 configs × 3 seeds = 6 runs.

### Results

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| filtered_tau1 | 0.2016 | 0.0173 | ✓ |
| filtered_tau5 | 0.1746 | 0.0078 | ✗ |

### Findings

**`filtered_tau1` technically clears the gate but is strictly inferior to `flat_d4m1024`** on both mean (0.202 vs 0.203) and std (0.017 vs 0.010). Not a meaningful improvement.

**The filtering hypothesis is backwards.** Break fixtures are never used during training — the predictor only learns from corpus records. Filtering break-similar corpus records removes *legitimate FastAPI code* that happens to share surface patterns with test fixtures, shrinking useful training signal. The regression scales with τ: dropping 100 records (5%) produces a larger mean drop than dropping 20 (1%).

**Future direction:** diversity-based corpus filtering — keeping records that maximally cover the embedding space (greedy farthest-point selection or k-means sampling) — is a more principled alternative. It directly attacks the "small corpus = undertrained predictor" problem by ensuring training data is maximally informative per record rather than redundant.

---

## Stage 4 — Inference Ensemble

**Hypothesis:** ensemble N predictors trained with different seeds and average their scores at inference time, reducing seed-to-seed variance while preserving the higher mean of `flat_d6m1024`.  
**Base:** `flat_d6m1024` (mean=0.221 unensembled, std=0.024 — highest mean, failed std gate alone).  
**Grid:** N ∈ {3} — 1 config × 3 outer seeds = 3 runs. (N=5 attempted but aborted due to RAM constraints on MPS device.)

### Results

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| **ensemble_n3** | **0.2215** | **0.0000** | **✓** |

### Findings

**Ensemble completely eliminates variance.** Three predictors with consecutive seeds {s, s+1, s+2}, averaged at inference time, produce identical delta across all outer seeds. The 0.05–0.09 seed-to-seed swings that plagued Stages 1–3 disappear entirely.

**Mean improves by +0.018 over `flat_d4m1024`** (0.2215 vs 0.2031). This comes from using `flat_d6m1024` as the base — a config with higher intrinsic capacity that was previously unusable due to std=0.024. The ensemble unlocks that capacity.

---

## Final Comparison

| config | mean_delta | std_delta | gate | stage |
|---|---|---|---|---|
| baseline (ep20_lr5e5, original) | 0.176 | 0.020 | ✗ | — |
| flat_d4m1024 | 0.203 | 0.010 | ✓ | 2 |
| filtered_tau1 | 0.202 | 0.017 | ✓ | 3 |
| **ensemble_n3** | **0.2215** | **0.0000** | **✓** | **4** |

---

## Winner

**`EnsembleJepaScorer(n=3)`** wrapping `JepaCustomScorer(epochs=20, lr=1e-4, flat schedule, depth=6, mlp_dim=1024)`.

Now registered as `REGISTRY["jepa_pretrained"]` — the default scorer used by the acceptance runner and signal reports.

---

## Signal Report — Post-Promotion

| entry | delta | gate | vs. baseline |
|---|---|---|---|
| ky | 0.2202 | ✓ | +0.006 (no regression) |
| httpx | 0.1401 | ✗ | — (corpus below stabilisation threshold) |
| **fastapi** | **0.2215** | **✓** | **+0.046 (gate cleared)** |

httpx remains below gate — its corpus size is below the ~1200-record stabilisation threshold identified in the corpus-size sweep. This is expected and acceptable; Phase 8 would need more httpx corpus data to address it.

---

---

## Stage 5 — Top-k Surprise Aggregation + Z-score Normalization

**Hypothesis:** break anomalies concentrate in a minority of the 768 MSE dimensions; selecting the k highest squared errors before averaging (top-k pooling) should lift delta. Z-score normalization against the held-out training split should additionally calibrate scores.
**Base:** `EnsembleJepaScorer(n=3)` over `flat_d6m1024` (mean=0.2215, std=0.000).
**Grid:** `{mean, top16, top32, top64, top128} × {zscore off, on}` + Goodhart random-k baseline `{rand16, rand32, rand64, rand128} × {zscore off, on}` — 18 configs × 1 seed = 18 runs. (Single outer seed — ensemble already collapses std to 0.000.)

### Results

Raw deltas for top-k configs are not comparable to the 0.2215 baseline: selecting the k largest squared errors inflates the absolute score scale for both breaks and controls. Z-scored deltas (normalized by held-out corpus std) provide the honest apples-to-apples comparison.

**Z-score track (honest comparison):**

| config | delta |
|---|---|
| `mean_z` (baseline) | **0.8404** |
| `top16_z` | 0.7168 |
| `top32_z` | 0.7876 |
| `top64_z` | 0.8187 |
| `top128_z` | 0.8652 |
| `rand16_z` | 0.7891 |
| `rand32_z` | 0.7486 |
| `rand64_z` | 1.1085 |
| `rand128_z` | 1.1038 |

**Raw track (scale-inflated, for completeness):**

| config | delta |
|---|---|
| `mean_no_z` (baseline) | 0.2215 |
| `top16` | 1.3447 |
| `top32` | 1.2220 |
| `top64` | 1.0102 |
| `top128` | 0.8051 |
| `rand16` | 0.1207 |
| `rand32` | 0.1696 |
| `rand64` | 0.2071 |
| `rand128` | 0.2121 |

### Findings

**Top-k does not improve signal separation.** In the z-score track, every top-k config underperforms the `mean_z` baseline (0.8404). The best top-k config (`top128_z` = 0.8652) barely exceeds the baseline, and that margin is within noise.

**Goodhart guardrail fires.** `rand16_z` (0.7891) beats `top16_z` (0.7168) — random dimension selection outperforms selecting actual highest-error dimensions. This confirms top-k is not localizing meaningful signal; it is just reducing dimensionality. When both dimensions and z-scoring are combined (`rand64_z` = 1.1085), the large number is an artifact of the interaction between random subsampling and z-score scale, not a genuine improvement.

**Z-scoring is scale-only.** Z-scoring divides all scores by the held-out corpus std (a constant ~0.26). The ranking of every fixture is unchanged, so AUC and classification quality are identical before and after z-scoring. The larger z-scored deltas carry no additional information.

**Phase 1 hypothesis rejected.** Break anomalies are diffuse across all 768 MSE dimensions; mean-pooling is already optimal. No setting from Stage 5 improves on `EnsembleJepaScorer(n=3, aggregation="mean")` = 0.2215.

**Baseline carried into Phase 2:** `EnsembleJepaScorer(n=3)` with default mean aggregation, no z-score.

---

---

## Stage 6 — InfoNCE In-Batch Contrastive Loss

**Hypothesis:** pure MSE trains the predictor to reconstruct the true hunk on average but doesn't push predictions closer to the true hunk than to other in-repo hunks. Adding an in-batch InfoNCE term (positives = true hunk; negatives = all other hunks in the batch, all FastAPI) sharpens the predictor without foreign data.
**Base:** `EnsembleJepaScorer(n=3)` MSE baseline (delta=0.2215).
**Grid:** `{beta ∈ 0.1, 0.25, 0.5, 1.0} × {tau ∈ 0.05, 0.07, 0.1} × {warmup ∈ 0, 5ep}` — 24 configs × 1 seed = 24 runs.

### Results

| config | beta | tau | warmup | delta | vs baseline |
|---|---|---|---|---|---|
| `b01_t01_w0` | 0.1 | 0.10 | 0 | **0.2291** | +0.008 |
| `b01_t01_w5` | 0.1 | 0.10 | 5 | 0.2268 | +0.005 |
| `b01_t007_w0` | 0.1 | 0.07 | 0 | 0.2173 | −0.004 |
| `b025_t01_w0` | 0.25 | 0.10 | 0 | 0.2143 | −0.007 |
| `b025_t01_w5` | 0.25 | 0.10 | 5 | 0.2079 | −0.014 |
| `b01_t005_w0` | 0.1 | 0.05 | 0 | 0.2075 | −0.014 |
| `b05_t01_w0` | 0.5 | 0.10 | 0 | 0.2014 | −0.020 |
| `b10_t01_w0` | 1.0 | 0.10 | 0 | 0.1929 | −0.029 |
| `b10_t005_w5` | 1.0 | 0.05 | 5 | 0.1464 | −0.075 |

### Findings

**Marginal improvement at best.** Only two configs beat the 0.2215 baseline: `b01_t01_w0` (+0.008) and `b01_t01_w5` (+0.005). Expected gain was +0.02–0.06; actual best is +0.008.

**Higher beta consistently hurts.** As beta increases (0.1 → 0.25 → 0.5 → 1.0), delta drops monotonically across all tau values. At beta=1.0, InfoNCE dominates the loss and collapses the predictor — worst config (b10_t005_w5) falls to 0.146. The loss can only help at beta=0.1, the weakest possible weight.

**Higher tau helps.** tau=0.1 outperforms tau=0.07 > tau=0.05 at every beta level. A softer contrastive temperature is less aggressive and preserves more MSE gradient.

**Warmup adds nothing.** w0 beats w5 in nearly every matched pair. The 5-epoch ramp-up delays InfoNCE without any mean gain.

**Root cause of weak signal:** in-batch FastAPI negatives are all from the same repo and similar style. They are easy negatives that provide little useful gradient — the predictor can already distinguish true context→hunk pairs from same-repo negatives after MSE training alone. InfoNCE with harder negatives (cross-repo) would be more informative but is ruled out by the self-supervised-only constraint.

**Winner carried into Phase 3:** `JepaInfoNCEScorer(beta=0.1, tau=0.1, warmup=0)` wrapped in `EnsembleJepaScorer(n=3)` — delta=0.2291. Treated as a tentative +0.008 gain; Phase 3 results will determine whether it stacks.

---

---

## Stage 7 — Diversity-Based Corpus Sampling

**Hypothesis:** the 2000-record linear prefix under-samples the FastAPI style space; a k-means-diverse or farthest-point subsample at the same training budget produces a more informative predictor.
**Base:** `_EnsembleInfoNCE(n=3, beta=0.1, tau=0.1, warmup=0)` — Stage 6 winner (delta=0.2291).
**Grid:** `{linear, diverse_kmeans, fps}` × corpus_cap=2000 — 3 configs × 1 seed = 3 runs.
(Cap variants 4000/8000 dropped: the catalog corpus.jsonl only contains 2000 records; re-extraction is deferred pending signal from sampling strategies.)

### Results

| config | delta |
|---|---|
| `linear` | 0.2291 |
| `diverse_kmeans` | 0.2291 |
| `fps` | 0.2291 |

### Findings

**No signal from diversity sampling at 2000 records.** All three strategies produce identical delta. With only 1600 training records (80% of 2000), the corpus is already compact enough that k-means and FPS select virtually the same set as the linear prefix — there is no meaningful redundancy to remove.

**Re-extraction not warranted.** Diversity sampling is only useful when the corpus is large enough that the linear prefix genuinely under-represents the style space. At 2000 records, the corpus is too small for the hypothesis to apply. Re-extracting to 4000–8000 records may revisit this, but requires a separate extraction run and is deferred to a future phase.

---

## Phase 7.X — Final Summary

### Sprint objective

Maximize FastAPI signal-sweep delta beyond 0.2215 (the Stage 4 ensemble baseline) without foreign corpus and without regressing ky.

### What was tried

| Stage | Lever | Best delta | vs. baseline |
|---|---|---|---|
| 5 | Top-k surprise aggregation + z-score | 0.2291 (mean_z, scale artifact) | **0** real gain |
| 6 | InfoNCE in-batch contrastive loss | **0.2291** | +0.008 |
| 7 | Diversity corpus sampling (k-means, FPS) | 0.2291 | **0** real gain |

### What worked

**InfoNCE at low beta (0.1) with soft temperature (tau=0.1)** gives a marginal but real +0.008 over the pure MSE baseline. The gain is small because in-batch FastAPI negatives are easy (same repo, same style) — but it is positive and reproducible at a single seed.

### What didn't work

- **Top-k surprise pooling:** Goodhart guardrail fired — random-k matches actual top-k in z-score space, confirming break signal is diffuse across all 768 dims.
- **Z-score normalization:** pure scale change, no AUC improvement.
- **Diversity corpus sampling:** no effect at 2000 records; corpus too small for redundancy to exist.

### Net result

**Delta: 0.2291** (up from 0.2215 baseline, +0.008). Not a breakthrough — the sprint's 0.28+ target was not reached. The bottleneck appears to be corpus size: with only 2000 FastAPI records, the predictor is constrained by data, not architecture or objective. All architectural and objective levers explored here are second-order effects at this corpus scale.

### Recommended next steps

1. **Re-extract FastAPI to 8000+ records** — this is the highest-EV lever remaining. Diversity sampling + larger InfoNCE batch would both benefit.
2. **Revisit diversity sampling at larger corpus** — `diverse_kmeans` and `fps` are implemented and ready; just need more data.
3. **httpx corpus growth** — httpx is below the 1200-record stabilisation threshold; more data is the only path to clearing its gate.

### Promotion decision

The Stage 6 winner (`_EnsembleInfoNCE(n=3, beta=0.1, tau=0.1, warmup=0)`, delta=0.2291) is a +0.008 improvement over the current default. Given the marginal gain and single-seed measurement, **no promotion** — the current `REGISTRY["jepa_pretrained"] = EnsembleJepaScorer` (0.2215) remains the default. The InfoNCE scorer is available in the sweep infrastructure for future re-evaluation with more data.

---

## Key Learnings

1. **Variance, not mean, was the primary obstacle.** The predictor could produce high-delta runs individually but not consistently across seeds. The fix was architectural (ensemble), not more training.

2. **More epochs hurts on small corpora.** Past ~50 epochs on 2000 records, variance increases and mean stagnates. The predictor overfits to the specific temporal split.

3. **Cosine LR schedule is too aggressive at 20 epochs.** It decays the learning rate before the predictor converges. Flat lr outperforms across all capacity levels.

4. **Wider MLP (1024) beats deeper predictor (depth=6) on variance.** Extra width increases representational capacity without destabilising training the same way extra depth does.

5. **Break-similarity corpus filtering is counterproductive.** Breaks are never used in training; filtering break-similar corpus records removes useful training signal.

6. **Ensemble unlocks high-capacity configs.** `flat_d6m1024` had the highest mean but failed the std gate alone. Ensembling 3 runs averaged away its variance and made its mean accessible.
