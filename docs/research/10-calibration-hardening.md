# Era 10 — Calibration Hardening

> **TL;DR.** Era 10 ships in two phases. Phase 1 (multi-seed median threshold, K=7)
> reduces threshold CV from 7–10% to ≤3% across all corpora while preserving era-9
> verdict coverage exactly — all 6 pre-registered gates pass, and the amended parity
> rule from Era 7 is retired. Phase 2 (root-conditional call-receiver weighting) adds
> +5pp recall on hono by catching the `{attested-root, unattested-callee}` pattern, with
> all 5 standard quality gates passing. Phase 3 (per-callee frequency weighting) is a
> bounded negative result: two formulations explored, both failed at structural limits
> documented below. Era 11 begins from the Phase 2 baseline.

## Problem

Since Era 6, the ink TypeScript corpus has carried a calibration coefficient of
variation (CV) significantly higher than other corpora (7–10% vs 1–4% for Python
corpora). Two fixtures (`ink_dom_access_1`, `ink_dom_access_2`) sit close enough
to the threshold that their verdicts flip between re-runs with different random
seeds.

An amended parity rule was introduced in Era 7 to absorb this noise: fixtures may
use any prior era's seed-0 result for parity comparisons instead of requiring
strict per-run parity. This rule has been in force through Eras 8 and 9. Era 10
targets the noise source directly rather than absorbing it.

### Root Cause

The `max(cal_scores)` estimator is a single-draw estimator: one unusually
high-scoring calibration hunk per seed shifts the threshold for that entire seed.
With n_cal=100 from a small corpus, the max value varies substantially across
seeds — this is the direct source of the CV problem.

## Phase 1: Multi-Seed Median Threshold

### Design Space Exploration

Before arriving at the shipping config, four single-statistic estimator
configurations were tested and rejected. This section summarizes the exploration.

| Config | n_cal | estimator | Failure |
|:---|---:|:---|:---|
| Primary | 300 | p95 | Gate 3 (verdicts 91.3%) and Gate 5 (FP) |
| Fallback A | 300 | max (p100) | Gate 3 (verdicts 93.9%) and Gate 6 (hono −6.6pp) |
| Fallback B | 100 | p95 | Gate 3 (verdicts 88.7%) and Gate 5 (FP) |
| IQR (k=2.5) | 100 | p75 + 2.5×IQR | Gate 1 (ink CV 12.3%) and Gate 2 (hono CV 10.7%) |

The failure pattern is consistent: any single-draw quantile estimator (max, p75,
p95) inherits seed-to-seed instability from small calibration pools. The p95
configs fix variance but drop the threshold too far (FP gate fails). The max+300
config reduces variance but introduces upward bias for hono (recall gate fails).
The IQR estimator was expected to tighten CV via margin scaling — it made it
strictly worse, amplifying p75 instability rather than dampening it.

Full evidence in `docs/research/evidence/calibration-hardening.md`.

### Key Insight: Breaking the Single-Draw Assumption

All four failed configs tried a different point statistic (max, p75, p95, IQR) on
**a single sample**. The hidden assumption was that the right statistic, applied
once to n_cal=100 hunks, could simultaneously be stable and well-calibrated.

The multi-seed median breaks this assumption: instead of one calibration draw,
run K independent calibrations and take the median of the K thresholds. This
uses the **same max statistic** as era-9 but aggregates across K independent
samples.

The mathematical motivation is straightforward: the median of K i.i.d. max
statistics has the same expected value as a single max (no bias change), but
variance is reduced by approximately 1/K (more precisely, Var[median] ≈
π/(2K) × Var[max] for symmetric distributions). For K=7, the predicted CV
reduction is ~0.38× (1/√7). This is why the threshold lands close to era-9 by
construction — median of max(100) ≈ max(100) in expectation for stable
distributions.

### Interventions

| Intervention | Old value (era-9) | New value |
|:---|:---|:---|
| threshold_n_seeds | — (single calibration) | 7 |
| n_cal | 100 | 100 (unchanged) |
| threshold_percentile | None (max) | None (max, unchanged) |

The only change is `threshold_n_seeds=7`: each outer seed now runs 7 independent
inner calibrations and takes the median of the 7 resulting thresholds. No scorer
behavior, alpha values, or extractor logic changed.

### Phase 1 Results

All 6 pre-registered gates pass.

| Corpus | Recall | FP | CV | Threshold |
|:---|---:|---:|---:|---:|
| fastapi | 91.7% | 0.55% | 0.0% | 5.2585 |
| rich | 95.0% | 0.82% | 0.0% | 3.8424 |
| faker | 95.0% | 1.04% | 3.0% | 5.2572 |
| hono | 78.3% | 0.43% | 0.2% | 4.2891 |
| ink | 93.3% | 0.44% | 0.0% | 4.9932 |
| faker-js | 53.3% | 0.95% | 0.0% | 4.8607 |
| **Avg** | **84.43%** | — | **max 3.0%** | — |

Verdict preservation vs era-9 is approximately 100%: all six corpus recalls match
era-9 exactly, meaning no fixture changed its detection outcome.

### Amended Parity Rule — Retired

Gate 1 clears at 0.0% for ink (down from 6.9% in era-9). The amended parity rule
from Era 7 — which allowed any prior era's seed-0 result for parity comparisons
due to ink instability — is now **RETIRED**. Strict per-run parity is restored.

### Phase 1 Shipped Deliverables

1. **Multi-seed median threshold** (`calibration/__init__.py`): `threshold_n_seeds=7`
   is the new default.
2. **Thin-pool fallback** (`random_hunk_sampler.py`): `sample_hunks` caps at pool
   size and emits `UserWarning` instead of raising when n > pool.
3. **CLI flags**: `--n-cal`, `--threshold-percentile`, and `--threshold-iqr-k`
   added to bench CLI and `argot-calibrate`.
4. **Consistency tests**: `threshold_percentile`, `n_cal`, and `threshold_iqr_k`
   defaults locked across all layers.

## Phase 2: Root-Conditional Call-Receiver Weighting

### Hypothesis

Era-9's call-receiver assigns a flat α=2.0 weight per unattested callee regardless
of whether the callee's root is attested. Phase 2 adds `root_bonus` to the weight
when the callee's root IS attested but the full callee is not.

- **Foreign method on known root** (`hono_middleware_2`: `req.send` where `req` is attested):
  strong "weird combination on familiar object" signal. `weighted_contribution` returns
  `alpha + root_bonus = 4.0`.
- **Unknown root entirely** (`new_helper`): possibly legitimate codebase evolution.
  Standard alpha=2.0 applies.

Default: `call_receiver_root_bonus=2.0`.

### Pre-flight Analysis

Pre-bench analysis predicted the config could not catch the primary faker-js
cluster (foreign_rng_1/3, fetch-based fixtures, error_flip fixtures) because those
callees are globally attested in the faker-js corpus — the scorer skips them
entirely. See Phase 3 for the definitive probe of these cases.

Predicted catch: `hono_middleware_2` (Express 4-arg error-handler signature), which
has attested roots (`req`, `res`, `next`) but calls receivers unattested in the
hono corpus.

### Phase 2 Results

Run against Phase-1 baseline (run 20260425T111854Z vs baseline 20260425T095307Z).

| Corpus | P1 Recall | P2 Recall | Δ | P1 FP | P2 FP | Δ |
|:---|---:|---:|---:|---:|---:|---:|
| fastapi | 91.7% | 91.7% | 0 | 0.6% | 0.6% | 0 |
| rich | 95.0% | 95.0% | 0 | 0.8% | 1.2% | +0.4pp |
| faker | 95.0% | 95.0% | 0 | 1.0% | 1.4% | +0.4pp |
| hono | 78.3% | **83.3%** | **+5pp** | 0.4% | 0.5% | +0.1pp |
| ink | 93.3% | 93.3% | 0 | 0.4% | 0.4% | 0 |
| faker-js | 53.3% | 53.3% | 0 | 1.0% | 0.9% | −0.1pp |
| **Avg** | **84.43%** | **85.27%** | **+0.84pp** | | | |

All 5 standard quality gates pass. hono gained one fixture (`hono_middleware_2`).

### Phase 2 Gate Matrix

| Gate | Threshold | Result |
|:---|:---|:---:|
| 1 — CV preserved | all ≤ P1+1pp | ✓ |
| 2 — Verdict parity ≥ 95% | ≥95% | ✓ 99.1% |
| 3 — Avg recall | ≥84.43% | ✓ 85.27% |
| 4 — Per-corpus FP | ≤1.5% | ✓ max 1.4% |
| 5 — Per-corpus recall | ≥P1−2pp | ✓ hono +5pp |

Phase 2 ships. Avg recall: **84.43% → 85.27%** (+0.84pp).

### Why Faker-JS Was Not Helped

The 8 uncaught faker-js fixtures use `Math.random`, `fetch`, and `Promise.resolve` —
all globally attested in the faker-js corpus (in jest test files and locale
utilities). The scorer's attested-set lookup returns True for these callees, so
`weighted_contribution` contributes 0 before root_bonus is even evaluated. Root_bonus
cannot help when the callee itself is attested.

This is the defining structural constraint era-10 could not cross: the faker-js
break is **contextual** (X called in a file where X doesn't belong) not **categorical**
(X foreign to the repo). Era-11 targets this axis directly.

### Phase 2 Shipped Deliverables

1. **`CallReceiverScorer.attested_roots`**: set of callee roots seen in corpus.
2. **`CallReceiverScorer.weighted_contribution`**: replaces flat `min(n_unattested, k)×α`
   with per-callee `alpha + root_bonus` for root-attested cases.
3. **`call_receiver_root_bonus=2.0`**: plumbed through calibration, bench, and CLI layers.
4. **Consistency tests**: root_bonus default locked across all layers.

## Phase 3: Per-Callee Frequency Weighting (Negative Results)

Phase 3 pursued per-callee frequency weighting as a direct probe of the faker-js
cluster. Two formulations reached their structural limits and neither ships.

### Phase 3 v1: Per-Callee Log-Rarity Weighting

**Hypothesis**: weight each unattested callee by −log P(c) where P(c) = callee frequency
in corpus. Rare callees contribute more; common callees contribute less.

**Config**: `max_weight=5.0`, `cap=8.0`, `alpha=2.0`. Run 20260425T120434Z.

**Outcome**: Gate 4 (FP ≤ 1.5%) failed on all 6 corpora with FP rates 24–30% on
Python corpora. Average recall improved to ~95.8% (faker-js 17/17), but FP is
catastrophic.

**Failure mechanism**: with attested vocabulary ~5000, probability p(c) ≈ 0.0002 for
every callee. −log(p) ≈ 8.5 saturates max_weight=5.0 for every callee regardless of
frequency. The formula degraded to "every callee contributes max_weight," identical to
era-9 alpha=5.0 with no cap.

**Documented bound**: per-callee log-rarity fails at vocabulary sizes ≥ ~500. No
parameter adjustment fixes this; the log-scale discrimination collapses when all
items have similarly low empirical probability.

### Phase 3 v2: Fraction-of-Unattested Weighting

**Hypothesis**: instead of per-callee weights, weight each hunk by the fraction of its
callees that are unattested: `contribution = max_weight × (n_unattested / n_total)`.

**Config**: `max_weight=5.0`, `min_callees=1`, `alpha=2.0`. Run 20260425T124221Z.

**Outcome**: Gates 3 and 5 failed. Average recall dropped to 81.90% (−2.53pp vs
baseline 84.43%). Hono fell from 78.3% to 65.0% (−13.3pp). Three Phase-1
call_receiver catches became misses.

**Failure mechanism**: fraction formula returns 0.0 when all callees in a hunk are
attested (`n_unattested == 0`). Phase 2's root_bonus fires on `{unattested callee,
attested root}` patterns — e.g., `hono/routing_2` has `app.all` (unattested) with
root `app` (attested). Fraction formula with this hunk: if the hunk has many
callees including some attested, the fraction is small, contributing less than
root_bonus would. The formula does not replicate root_bonus semantics and actively
regresses existing catches.

**Documented bound**: fraction-of-unattested is not a superset of root_bonus. Using it
as a replacement regresses `{unattested callee, attested root}` catches. Using it
additively on top of root_bonus was not tested — but the faker-js cluster (target
callees all attested) would still show fraction=0, yielding no gain.

No fallbacks run: both pre-registered fallbacks (min_callees=2, max_weight=4.0)
share the fraction=0 structural defect.

### Phase 3 Design Space — Closed

| Version | Approach | Gate failure | Root cause |
|:---|:---|:---|:---|
| v1 | Per-callee log-rarity | FP 24–30% | Vocab saturation: p(c)→0 for all, −log(p) hits cap |
| v2 | Fraction-of-unattested | Recall −2.53pp | fraction=0 for attested-callee hunks → weaker than root_bonus |

Both bounds are structural. Era-11 must use a different signal source: file-cluster-
conditional attestation, where "attested" means "seen in files of this kind" rather
than "seen anywhere in the repo."

## Issue Status

GitHub issue #27 — "Reduce ink calibration CV below 4%" — **closed as resolved** by
Phase 1. ink CV: 6.9% → 0.0%.

## Era-10 Baseline

| Metric | Era-9 | Era-10 (Phase 2 ship) |
|:---|---:|---:|
| Avg recall | 84.43% | 85.27% |
| Max FP | 1.0% | 1.4% |
| Max CV | 6.9% | 3.0% |
| Fixtures | 115 | 116 (hono_middleware_2) |
| Amended parity rule | active | **RETIRED** |
