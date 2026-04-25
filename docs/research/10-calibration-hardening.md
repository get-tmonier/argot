# Era 10 — Calibration Hardening

> **TL;DR.** Reduce ink's calibration coefficient of variation from ~7–10% to
> below 4% by tripling the calibration sample size (n_cal: 100 → 300) and
> switching from max-based to percentile-based threshold estimation (max →
> p95 with linear interpolation). Both changes are calibration-only; no scorer
> behavior or extractor changes. The amended parity rule introduced in Era 7
> retires if ink threshold CV drops below 4%.

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

### Root Causes

**1. Small calibration sample (n_cal=100).** Threshold variance scales as
approximately `1/√n_cal`. With only 100 calibration hunks, a single unusual hunk
per seed noticeably shifts the threshold. Tripling to n_cal=300 should reduce CV
by approximately 0.58×, lowering the expected ink CV to around 4–6%.

**2. max(cal_scores) is a single-outlier estimator.** One unusually high-scoring
calibration hunk lifts the threshold for that entire seed. The p95 percentile
with linear interpolation is robust to individual outliers while still remaining
conservative: the top 5% of calibration hunks can be arbitrarily high without
affecting the threshold estimate.

## Interventions

| Intervention | Old value | New value |
|:---|:---|:---|
| n_cal (calibration sample size) | 100 | 300 |
| threshold estimator | max(cal_scores) | p95 with linear interpolation |

Both changes are **calibration-only** — no scorer behavior, alpha values, or
extractor logic changes. `call_receiver_alpha=2.0`, complex-chain canonicalization,
and all extractors remain unchanged from Era 9.

### Implementation

- `SequentialImportBpeScorer.threshold_percentile` default: `None` (max) → `95.0`
  (p95 with linear interpolation)
- `RunConfig.n_cal` default: `100` → `300`
- `--threshold-percentile` and `--n-cal` flags added to the bench CLI for future
  A/B configurations
- Sampler hardened: `sample_hunks` now caps at pool size and emits `UserWarning`
  when requested n > available pool, instead of raising an error. This is
  necessary because ink's thin calibration pool can fall below n_cal=300 in some
  configurations.

### Consistency Tests

The era-9 default-consistency test suite (which locked `call_receiver_alpha=2.0`
across all layers) was extended to also lock `threshold_percentile=95.0` and
`n_cal=300` across engine and bench layers. All configuration pairs in the
consistency suite verify that engine and bench produce identical thresholds and
predictions.

## Results

_[To be filled after bench run completes]_

## Gate Clearance

_[To be filled after bench run completes]_

## Amended Parity Rule Retirement

If Gate 1 clears — ink threshold CV drops below 4% across all seeds — the
amended parity rule introduced in Era 7 retires. Future eras can use strict
per-fixture parity gates without seed-0 fallbacks. Fixtures will be held to
exact verdict consistency across re-runs, as in all other corpora.

## Issue Closed

This era targets GitHub issue #27 — "Reduce ink calibration CV below 4%."
