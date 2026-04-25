# Era 10 — Calibration Hardening

> **TL;DR.** Era 10 tested four calibration estimator configurations to reduce
> ink's threshold CV from ~7–10% below 4%: p95+300, max+300, p95+100, and
> IQR-margin (p75 + k×IQR). All four fail at least one pre-registered gate.
> The era ships as a **four-config negative result** with infrastructure
> improvements (CLI flags, thin-pool fallback, consistency tests) and a
> detailed root-cause analysis pointing toward multi-sample aggregation for
> Era 11. Defaults revert to era-9 values (n_cal=100, max estimator).

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

No config clears all six gates. Full data in `docs/research/evidence/calibration-hardening.md`.

**Gate summary (Primary = p95+300, A = max+300, B = p95+100, IQR = p75+2.5×IQR+100):**

| Gate | Threshold | Primary | A | B | IQR |
|:---|:---|:---:|:---:|:---:|:---:|
| 1 — ink CV | <4% | ✓ 0.0% | ✓ 0.0% | ✓ 0.0% | ✗ 12.3% |
| 2 — all CV | <5% | ✓ | ✓ | ✗ | ✗ |
| 3 — verdicts | ≥95% | ✗ 91.3% | ✗ 93.9% | ✗ 88.7% | ✓ ~97.4% |
| 4 — avg recall | ≥83.43% | ✓ 87.5% | ✗ 83.3% | ✓ 87.8% | ✗ 81.4% |
| 5 — FP | ≤1.5% | ✗ | ✓ | ✗ | ✓ |
| 6 — recall >2pp | per corpus | ✓ | ✗ hono | ✓ | ✗ hono/ink/faker-js |

The failure pattern is consistent: any single-draw quantile estimator (max, p75, p95) with
n_cal=100 inherits seed-to-seed instability from small calibration pools. Increasing n_cal
reduces CV (Gates 1–2) but introduces upward bias for corpora like hono (Gate 6). IQR
was expected to tighten CV via margin scaling; instead p75 instability plus the IQR
multiplier made it strictly worse for small-pool corpora (ink, hono, faker-js).

## Gate Clearance

No gate cleared. Era 10 ships as a negative result.

## Amended Parity Rule

Gate 1 did not clear for all configurations (IQR fails at 12.3% for ink). The amended
parity rule from Era 7 remains in force.

## Shipped Deliverables

Despite the negative result, era 10 ships:

1. **Thin-pool fallback** (`random_hunk_sampler.py`): `sample_hunks` caps at pool size and emits `UserWarning` instead of raising when n > pool.
2. **CLI flags**: `--n-cal` and `--threshold-percentile` added to bench CLI; `--threshold-iqr-k` added for IQR configs.
3. **Calibration CLI flags**: `--threshold-percentile` and `--threshold-iqr-k` added to `argot-calibrate`.
4. **Consistency tests**: `threshold_percentile`, `n_cal`, and `threshold_iqr_k` defaults locked across all layers via `test_defaults_consistent.py` and `test_bench_alpha_defaults.py`.

Defaults remain at era-9 values: n_cal=100, threshold_percentile=None (max).

## Era-11 Forward

The single-quantile approach is exhausted across four structurally different configs.
Candidates for Era 11 require a paradigm shift:

- **Multi-sample aggregation**: calibrate on k independent subsamples, average thresholds — reduces variance by √k without a larger pool.
- **Explicit bias correction**: fit expected max(n) as a function of n; correct for upward bias, allowing n_cal=300 with unbiased max.
- **Per-corpus learned threshold percentile**: learn the percentile achieving a target FP rate on a held-out control set.
- **Calibration-aware parity testing**: accept ~7% CV as intrinsic noise; adjust parity methodology to tolerate it (alternative framing).

## Issue Status

GitHub issue #27 — "Reduce ink calibration CV below 4%" — remains open. Era 10 provides a
complete characterization of why single-quantile estimators cannot simultaneously satisfy
all gates for this corpus set.
