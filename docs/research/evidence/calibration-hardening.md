# Era 10 — Calibration Hardening Evidence

## Bench Configuration

All runs: 5 seeds [0,1,2,3,4], full corpus set, no sample-controls.
Era-9 baseline: n_cal=100, threshold_percentile=None (max).

## Three-Config Landscape

| Config | n_cal | estimator | Description |
|:---|---:|:---|:---|
| Primary | 300 | p95 | Both interventions combined |
| Fallback A | 300 | max (p100) | Larger sample, keep max estimator |
| Fallback B | 100 | p95 | Original sample size, switch estimator only |

## Threshold CV — All Corpora

| Corpus | Era-9 CV | Primary CV | Fallback A CV | Fallback B CV |
|:---|---:|---:|---:|---:|
| fastapi | 0.4% | 2.5% | 0.0% | 12.7% |
| rich | 9.5% | 0.0% | 0.0% | 6.6% |
| faker | 3.7% | 0.0% | 0.0% | 4.9% |
| hono | 3.0% | 1.7% | 3.4% | 8.4% |
| ink | 6.9% | **0.0%** | **0.0%** | **0.0%** |
| faker-js | — | 0.0% | 0.0% | 1.6% |

Gate 1 (ink CV < 4%): all three configs pass.
Gate 2 (all CV < 5%): Primary ✓, Fallback A ✓, Fallback B ✗ (fastapi 12.7%, hono 8.4%).

## Threshold Level — All Corpora

Era-9 thresholds (max of 100 samples):

| Corpus | Era-9 Thr | Primary Thr | Fallback A Thr | Fallback B Thr |
|:---|---:|---:|---:|---:|
| fastapi | 5.278 | 3.486 | 5.307 | 3.543 |
| rich | 4.164 | 3.146 | 4.647 | 3.152 |
| faker | 5.211 | 3.887 | 5.496 | 3.856 |
| hono | 4.277 | 3.976 | 5.638 | 3.806 |
| ink | 4.826 | 3.543 | 4.993 | 3.543 |
| faker-js | 4.773 | 4.128 | 4.861 | 4.169 |

Key observations:
- p95 consistently produces thresholds ~1.3–1.8 points below era-9 max, regardless of n_cal.
- max with n=300 (Fallback A) closely matches era-9 thresholds for most corpora, but hono's max jumped +1.36 points (4.277→5.638).

## FP Rate — All Corpora

| Corpus | Era-9 FP | Primary FP | Fallback A FP | Fallback B FP |
|:---|---:|---:|---:|---:|
| fastapi | 0.8% | 4.4% | 0.3% | 6.7% |
| rich | 0.8% | 1.7% | 0.6% | 2.2% |
| faker | 1.2% | 3.7% | 0.7% | 3.9% |
| hono | 0.5% | 1.3% | 0.2% | 2.1% |
| ink | 0.4% | 2.5% | 0.4% | 2.5% |
| faker-js | 1.0% | 1.5% | 1.0% | 1.4% |

Gate 5 (no corpus FP > 1.5%): Primary ✗ (fastapi/rich/faker/ink), Fallback A ✓, Fallback B ✗ (all but faker-js).

## Recall — All Corpora

| Corpus | Era-9 Recall | Primary Recall | Fallback A Recall | Fallback B Recall |
|:---|---:|---:|---:|---:|
| fastapi | 91.7% | 91.7% | 91.7% | 93.5% |
| rich | 95.0% | 95.0% | 95.0% | 95.0% |
| faker | 95.0% | 95.0% | 95.0% | 95.0% |
| hono | 78.3% | 83.3% | 71.7% | 83.3% |
| ink | 93.3% | 100.0% | 93.3% | 100.0% |
| faker-js | 53.3% | 60.0% | 53.3% | 60.0% |
| **Avg** | **84.43%** | **87.5%** | **83.3%** | **87.8%** |

Gate 4 (avg recall ≥ 83.43%): Primary ✓, Fallback A ✗ (83.3%), Fallback B ✓.
Gate 6 (no corpus regresses > 2pp): Primary ✓, Fallback A ✗ (hono −6.6pp), Fallback B ✓.

## Verdict Preservation vs Era-9 (115 fixtures)

| Config | Same verdict | Preservation % | Regressions | Improvements | Reason-only changes |
|:---|---:|---:|---:|---:|---:|
| Primary | 105/115 | 91.3% | 0 | 3 | 7 (call_receiver→bpe) |
| Fallback A | 108/115 | 93.9% | 1 (hono_routing_3) | 0 | 6 (bpe→call_receiver) |
| Fallback B | 102/115 | 88.7% | 0 | 4 | 9 (call_receiver→bpe) |

Gate 3 (verdict preservation ≥ 95%): all three configs fail.

Notes:
- All regressions in Primary and Fallback B are zero (no catch→miss flips).
- Reason-only changes (call_receiver→bpe or vice versa, flagged stays True) occur because the threshold shift moves some fixtures across the call_receiver/bpe boundary. These are not detection regressions.
- Fallback A's single regression (hono_routing_3: bpe→none) is a genuine miss caused by hono's inflated threshold (5.638 vs era-9 4.277).
- The reason-only change rate tracks threshold distance from era-9: Primary (−1.3 to −1.8) → 7 flips; Fallback B (same threshold levels as Primary) → 9 flips; Fallback A (+1.36 for hono) → 6 flips.

## Complete Gate Matrix

| Gate | Threshold | Primary | Fallback A | Fallback B |
|:---|:---|:---:|:---:|:---:|
| 1 — ink CV | <4% | ✓ 0.0% | ✓ 0.0% | ✓ 0.0% |
| 2 — all CV | <5% | ✓ | ✓ | ✗ fastapi 12.7% |
| 3 — verdicts | ≥95% | ✗ 91.3% | ✗ 93.9% | ✗ 88.7% |
| 4 — avg recall | ≥83.43% | ✓ 87.5% | ✗ 83.3% | ✓ 87.8% |
| 5 — FP | ≤1.5% | ✗ fastapi 4.4% | ✓ | ✗ fastapi 6.7% |
| 6 — no recall >2pp | per corpus | ✓ | ✗ hono −6.6pp | ✓ |

**No config clears all gates. Era 10 fails under the pre-registered design space.**

## Root Cause Analysis

The BPE calibration score distribution has a heavy upper tail. The 95th percentile sits ~1.3–1.8 points below the maximum, and this gap is densely populated by control (non-argot) hunks. Dropping the threshold into this region inflates FP rates severely.

The max estimator avoids this gap but has two problems:
1. **Variance (existing era-9 issue):** max is a single-point estimator, sensitive to one outlier per seed.
2. **Upward bias with more samples (new finding):** increasing n_cal from 100→300 increases the expected max, because more samples → higher probability of drawing a rare high-scoring hunk. For hono, this bias was +1.36 points, enough to miss a fixture.

Simple percentile estimators cannot simultaneously satisfy all gates because:
- p95 fixes variance but drops the threshold too far → FP gate fails.
- max(n=100) preserves threshold level but has high variance → ink CV gate fails (era-9 problem).
- max(n=300) reduces variance but introduces upward bias → hono recall gate fails.

## Era-11 Candidate Approaches

To resolve these constraints, a better estimator must be:
1. **Robust to outliers** (like p95) — not dominated by a single extreme value.
2. **Unbiased relative to era-9 threshold level** — so control hunks don't flood the FP gate.
3. **Stable across seeds** — low CV.

Candidates:
- **Trimmed mean** (e.g., mean of top 50% of cal scores): smooth estimator, reduces outlier influence, but doesn't converge to the max.
- **IQR-based threshold** (e.g., Q3 + k*IQR): calibrated to corpus spread, explicit control over FP level.
- **Explicit bias correction**: fit the expected max(n) as a function of n and correct for it, allowing n_cal=300 with unbiased max.
- **Per-corpus learned threshold percentile**: instead of a fixed p95, learn the percentile that achieves a target FP rate on a held-out control set.

## Shipped Deliverables

Despite the negative experimental result, era 10 ships the following infrastructure improvements:

1. **Sampler thin-pool fallback** (`random_hunk_sampler.py`): `sample_hunks` now caps at pool size and emits a `UserWarning` instead of raising `ValueError` when n > available pool. Required by rich corpus (237–238 qualifying hunks < 300).

2. **CLI flags** (`argot_bench/cli.py`): `--n-cal` and `--threshold-percentile` flags added to bench CLI (main + run-one). Enables future era A/B configs without code changes.

3. **Calibration CLI flag** (`calibration/__init__.py`): `--threshold-percentile` added to `argot-calibrate` CLI.

4. **Consistency tests**: `test_defaults_consistent.py` and `test_bench_alpha_defaults.py` extended to lock `threshold_percentile` and `n_cal` defaults across all layers, preventing future silent drift.

Defaults remain at era-9 values: n_cal=100, threshold_percentile=None (max).
