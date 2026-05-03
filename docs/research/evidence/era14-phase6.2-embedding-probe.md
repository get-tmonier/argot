# Era 14 Phase 6.2 — UnixCoder embedding KILL-SWITCH probe

**Date**: 2026-05-03
**Branch**: `docs/era-10-root-readme` (analysis-only; no code changes)
**Inputs**: `engine/.era14-features/{fastapi,rich,faker,hono,ink,faker-js}.jsonl` — 1891 rows (115 breaks, 1776 controls). Each row has `features.*` (Phase 5 engineered features) plus top-level `hunk_embedding` (768-d UnixCoder) and `context_embedding` (768-d UnixCoder).
**Reference**: Phase 5 memo [`era14-fixA-full.md`](era14-fixA-full.md), status [`era14-status.md`](era14-status.md)
**Code**: `engine/scripts/era14_phase62_probe.py` (one-shot)
**Persisted**: `/tmp/era14_phase62_results.json`
**Models saved**: `engine/.era14-features/pca100_phase6.2.joblib` (PCA-100 fit on per-corpus z-scored concat(hunk, ctx))

---

## TL;DR

| Test | Pre-reg gate | Result | Pass |
|---|---|---|---|
| Pooled dims with AUC > 0.65 | ≥ 5 | **134** (55 hunk + 79 ctx) | ✓ |
| LR(PCA-100) pooled 5-fold CV AUC | > 0.85 | **0.993** | ✓ |
| Top-3 PCA components AUC | > 0.70 (alt) | **0.81 / 0.78 / 0.75** | ✓ |
| Residuals above p90 of fjs control centroid distance | ≥ 1 of 5 | **4 of 5** | ✓ |
| Residuals above p90 (STRONG threshold) | ≥ 2 of 5 | **4 of 5** | ✓✓ |

**Verdict (per pre-registered rules): STRONG SIGNAL — proceed to Phase 6.3 (MLP head training).**

UnixCoder embeddings carry distinguishing signal far beyond what the engineered features could see. The single best embedding dimension (ctx-717, AUC 0.765) is weaker than the best engineered feature (`n_unattested_callees`, 0.886), but PCA-100 + linear logistic regression already saturates at pooled 5-fold CV AUC **0.993** — i.e. the embeddings are richly linearly separable. More importantly, the kill-switch test on the 5 residual faker-js fixtures the engineered features cannot catch (because their callees ARE in the cluster's attested set, so `n_unattested_callees=0`) shows **4 of 5 residuals sit above the 90th percentile of faker-js controls' cluster-centroid cosine distance**, and their nearest-neighbours are dominated by other catalog breaks (5–15 of 30 NN are breaks vs ~6% base rate in faker-js).

This is the cleanest "embeddings genuinely add new information" result the era has produced. Caveat: the 0.993 pooled CV AUC almost certainly contains some pretext leak (catalog fixtures share authoring patterns); the cross-corpus LOO and residual catch under LOO will be the real tests in Phase 6.3.

---

## Setup

| Corpus | Rows | Breaks | Controls |
|---|---:|---:|---:|
| fastapi | 327 | 32 | 295 |
| rich | 316 | 16 | 300 |
| faker | 313 | 16 | 297 |
| hono | 314 | 17 | 297 |
| ink | 306 | 17 | 289 |
| faker-js | 315 | 17 | 298 |
| **total** | **1891** | **115** | **1776** |

Embeddings are 768-d UnixCoder vectors per row, two flavours: `hunk_embedding` (the modified region itself) and `context_embedding` (broader file context). All AUCs use auto-flipping (`max(auc, 1-auc)`).

---

## Task 1 — Per-dimension pooled AUC

For each of the 1536 individual embedding dimensions (768 hunk + 768 ctx), compute pooled AUC for `is_break` over all 1891 rows.

### Histogram of all 1536 dim AUCs

| Bin | Count |
|---|---:|
| [0.50, 0.55) | 621 |
| [0.55, 0.65) | 781 |
| [0.65, 0.75) | 131 |
| [0.75, 0.85) | 3 |
| [0.85, 1.01) | 0 |

**134 of 1536 dimensions clear AUC > 0.65 (55 hunk + 79 ctx)** → kill-switch gate passes by 27×.

The distribution is unimodal and right-skewed: most individual dimensions are weak (0.55–0.65), a meaningful fraction (~9%) carry standalone signal stronger than that, and three dimensions reach the 0.75–0.85 band. No single dim is a silver bullet (top is 0.765), but the breadth of moderate-AUC dims is what makes a linear or non-linear head viable.

### Top-20 hunk_embedding dimensions by AUC

| Dim | AUC | Dim | AUC |
|---:|---:|---:|---:|
| 166 | 0.7550 | 243 | 0.6918 |
| 667 | 0.7310 | 362 | 0.6915 |
| 439 | 0.7225 | 164 | 0.6906 |
| 238 | 0.7222 | 117 | 0.6875 |
| 682 | 0.7136 | 507 | 0.6875 |
| 738 | 0.7081 | 659 | 0.6838 |
| 717 | 0.7057 | 293 | 0.6836 |
| 560 | 0.7045 | 487 | 0.6817 |
| 323 | 0.7020 | 555 | 0.6803 |
| 177 | 0.6995 | 637 | 0.6788 |

### Top-20 context_embedding dimensions by AUC

| Dim | AUC | Dim | AUC |
|---:|---:|---:|---:|
| 717 | 0.7652 | 691 | 0.7184 |
| 386 | 0.7512 | 495 | 0.7159 |
| 738 | 0.7482 | 227 | 0.7123 |
| 765 | 0.7398 | 644 | 0.7114 |
| 178 | 0.7301 | 760 | 0.7113 |
| 522 | 0.7300 | 551 | 0.7105 |
| 177 | 0.7274 | 711 | 0.7103 |
| 243 | 0.7214 | 584 | 0.7090 |
| 613 | 0.7209 | 362 | 0.7077 |
| 165 | 0.7206 | 708 | 0.7063 |

Notable: dims **177, 243, 362, 717, 738** appear high in both hunk and context lists — those latent axes carry signal regardless of whether the model looks at the hunk or its surrounding file. Plausibly tracks something like "external-call-ish code" vs "pure-data code" along a single UnixCoder feature axis.

---

## Task 2 — PCA + Logistic Regression

Concatenated `[hunk_embedding ; context_embedding]` → 1536-d. Per-corpus z-score standardisation (each corpus independently zero-centred and unit-scaled), then PCA to 100 components.

- **PCA-100 explained variance**: 0.6989 (70% of total variance retained)

### Top-20 PCA components by individual AUC

| PC | AUC | PC | AUC |
|---:|---:|---:|---:|
| 3 | **0.8114** | 22 | 0.6381 |
| 9 | **0.7788** | 88 | 0.6354 |
| 5 | **0.7451** | 51 | 0.6215 |
| 19 | 0.6975 | 23 | 0.6179 |
| 52 | 0.6716 | 40 | 0.6149 |
| 70 | 0.6712 | 46 | 0.6122 |
| 4 | 0.6492 | 55 | 0.6103 |
| 61 | 0.6460 | 65 | 0.6102 |
| 21 | 0.6436 | 54 | 0.6060 |

**Top-3 PCA component AUCs: 0.811 / 0.779 / 0.745** → all clear the 0.70 alt-gate.

### LR on PCA-100 — 5-fold CV pooled AUC = **0.993**

Stratified 5-fold CV over all 1891 rows. `LogisticRegression(C=1.0, max_iter=2000)` fitted on standardised PCA-100 features. AUC computed on out-of-fold predictions.

This number is impressively high but should be read with care: 5-fold CV pools all corpora, so the model can learn that "rows from fastapi look like X, rows from rich look like Y" and use those corpus signatures alongside genuine break signal. The Phase 5 conservative XGBoost on engineered features hit 0.9035 pooled CV AUC and still produced 0/5 LOO catch — pooled CV is not predictive of cross-corpus generalisation. The honest test will be Phase 6.3 LOO.

That said, LR(PCA-100) at 0.993 is materially higher than the engineered model's 0.87–0.90 ceiling, suggesting the embedding subspace contains at least some signal the engineered features missed. Whether that signal survives a held-out corpus is what Phase 6.3 will tell us.

---

## Task 3 — RESIDUAL FIXTURE PROBE

The 5 residual faker-js fixtures (`error_flip_2`, `error_flip_3`, `runtime_fetch_1/2/3`) all have `n_unattested_callees = 0` — their callees are in their cluster's attested set, so the engineered features Phase 5 trained on cannot see them. The question: do UnixCoder embeddings see them?

### 3a — 30 nearest neighbours (cosine on `hunk_embedding`)

For each residual, find the 30 cosine-nearest neighbours in (a) the faker-js pool (controls + breaks, n=315) and (b) the all-corpora pool (n=1891). Excluding self.

Base rate of `is_break` is ~5.4% in faker-js and ~6.1% pooled. By chance we'd expect ~1.6 breaks among 30 NN.

| Residual | NN breaks (faker-js pool) | NN breaks (all pool) | Max sim to any break |
|---|---:|---:|---:|
| `runtime_fetch_1` | 7 / 30 | 7 / 30 | 0.800 |
| `runtime_fetch_2` | 15 / 30 | 9 / 30 | 0.638 |
| `runtime_fetch_3` | 11 / 30 | 10 / 30 | 0.724 |
| `error_flip_2` | 5 / 30 | 5 / 30 | 0.650 |
| `error_flip_3` | 6 / 30 | 6 / 30 | 0.653 |

All five residuals are 3–9× enriched for breaks among their NN vs the base rate. Inspecting the named neighbours: `runtime_fetch_1`'s top break neighbours include `runtime_fetch_4`, `runtime_fetch_3`, `runtime_fetch_2`, `error_flip_2`, `threading_1/3`, and the cross-corpus `requests_source_1` (faker). The residuals are clearly clustering with other catalog "external-effect" breaks rather than dispersing into controls — exactly the "anomalous under embeddings" outcome the task spec named as exploitable.

### 3b — Cosine distance to cluster centroid

Centroid = mean (L2-normalised) `hunk_embedding` of `is_break=False` rows in the same `(corpus, cluster_id)`. Distance = `1 − cosine_sim(row, centroid)`. Computed only for rows with `cluster_id != -1`.

- **Pooled AUC of distance-to-centroid as `is_break` predictor**: 0.721
- **faker-js-only AUC**: **0.908** ← single feature; no model
- faker-js controls with valid centroid: 125 (of 298)
  - p90 distance: 0.392
  - p95 distance: 0.415

| Residual | dist-to-centroid | percentile vs faker-js controls | Above p90? |
|---|---:|---:|:---:|
| `error_flip_2` | 0.5331 | 1.000 (max) | **✓** |
| `runtime_fetch_2` | 0.4931 | 0.992 | **✓** |
| `runtime_fetch_1` | 0.4670 | 0.976 | **✓** |
| `runtime_fetch_3` | 0.4248 | 0.960 | **✓** |
| `error_flip_3` | 0.3138 | 0.656 | ✗ |

**4 of 5 residuals sit above the 90th percentile of faker-js controls' centroid distance** — `error_flip_2` is literally the most anomalous row in the entire faker-js pool by this metric. The lone exception is `error_flip_3` at the 66th percentile (still above median, just not in the anomalous tail).

This is the decisive result. The pre-registered residual kill-switch ("≥ 1 above p90") clears with margin; the STRONG-SIGNAL threshold ("≥ 2 above p90") clears 4× over.

---

## Task 4 — Verdict

Pre-registered interpretation rules (from task spec):

- STRONG SIGNAL: pooled gates pass AND ≥ 2 residuals above p90 → all three conditions satisfied
- WEAK SIGNAL: pooled gates pass AND 1 residual above p90 → not the case (we have 4)
- NO SIGNAL: pooled gates fail OR all 5 residuals look completely normal → not the case

**Verdict: STRONG SIGNAL — proceed to Phase 6.3 (MLP head training).**

What this means concretely: UnixCoder embeddings appear to encode a semantic axis that distinguishes the residual `runtime_fetch_*` and `error_flip_*` fixtures from their files' attested-control neighbourhood, even though their callees are entirely in the attested set. Plausibly the head can learn to spot "this hunk is doing something semantically different from this file's typical behaviour" via the embedding geometry, where the engineered callee/identifier features could not.

Caveat to carry into Phase 6.3: the pooled 5-fold CV AUC of 0.993 is suspiciously clean. Phase 5 saw the same pattern (pooled 0.886, LOO residual catch 0/5). The Phase 6.3 LOO + Set-B-restricted residual catch will be the only test that matters.

---

## Task 5 — Diagnostic comparison vs engineered features

| Predictor | Pooled AUC |
|---|---:|
| Best single ENGINEERED feature: `n_unattested_callees` | **0.886** |
| Best single EMBEDDING dimension: ctx-717 | 0.765 |
| Best single PCA-100 component: PC-3 | 0.811 |
| LR on PCA-100 embeddings (5-fold CV) | **0.993** |
| LR on engineered conservative Set B (Phase 3.6b) | ~0.87 |

Key observations:

1. **Single-feature comparison favours engineered**: the strongest individual engineered feature beats the strongest individual embedding dimension by 0.12 AUC. Embeddings don't have a single "kill" axis — their power is in linear combinations.
2. **Composite comparison favours embeddings**: linear regression on PCA-100 of embeddings (0.993) substantially beats the engineered XGBoost's pooled performance (~0.90). Whether this generalises is the open question.
3. **Most importantly — the residuals**: the engineered features cannot see the 5 residuals at all (`n_unattested_callees=0` for all five). The embedding centroid-distance feature alone gives 4/5 residuals percentile > 0.96 in their corpus's control distribution. This is the qualitatively new signal.

Embeddings are not just rediscovering what engineered features already had — at minimum, they discriminate inside the regime where engineered features go silent.

---

## What to take into Phase 6.3

- **PCA-100 model** (`pca100_phase6.2.joblib`) is reusable: per-corpus z-score → PCA → 100-d feature space already validated as linearly separable. Phase 6.3 MLP can either consume PCA-100 directly or re-fit on raw 1536-d.
- **Cluster-centroid distance** is a free single-scalar feature with faker-js AUC 0.91 and 4/5 residual catch at p90 — at minimum, add it to the engineered feature set for the conservative XGBoost model regardless of whether the MLP head ships.
- **Bench the right thing**: the Phase 5 mistake to avoid is shipping a pooled-CV winner that collapses under LOO. Phase 6.3 must report (a) pooled CV AUC, (b) LOO test AUC per corpus, and (c) faker-js-LOO residual catch at FP ≤ 0.9% — gate on (c).
- **Failure mode to watch for**: if Phase 6.3 also produces pooled 0.99 + LOO 0/5, the embeddings are encoding catalog/authoring-style features (not break semantics), and era 14 still closes negative even with this "STRONG SIGNAL" probe result.
