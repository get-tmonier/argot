# Era 10 — Calibration Hardening Evidence

## Bench Configuration

All runs: 5 seeds [0,1,2,3,4], full corpus set, no sample-controls.
Era-9 baseline: n_cal=100, threshold_percentile=None (max).

## Four-Config Landscape

| Config | n_cal | estimator | Description |
|:---|---:|:---|:---|
| Primary | 300 | p95 | Both interventions combined |
| Fallback A | 300 | max (p100) | Larger sample, keep max estimator |
| Fallback B | 100 | p95 | Original sample size, switch estimator only |
| IQR (k=2.5) | 100 | p75 + 2.5×IQR | Margin-based, data-derived width |

## Threshold CV — All Corpora

| Corpus | Era-9 CV | Primary CV | Fallback A CV | Fallback B CV | IQR CV |
|:---|---:|---:|---:|---:|---:|
| fastapi | 0.4% | 2.5% | 0.0% | 12.7% | 2.7% |
| rich | 9.5% | 0.0% | 0.0% | 6.6% | 5.7% |
| faker | 3.7% | 0.0% | 0.0% | 4.9% | 5.5% |
| hono | 3.0% | 1.7% | 3.4% | 8.4% | **10.7%** |
| ink | 6.9% | **0.0%** | **0.0%** | **0.0%** | **12.3%** |
| faker-js | — | 0.0% | 0.0% | 1.6% | 1.9% |

Gate 1 (ink CV < 4%): percentile configs (Primary/A/B) ✓; IQR ✗ (12.3%).
Gate 2 (all CV < 5%): Primary ✓, Fallback A ✓, Fallback B ✗ (fastapi 12.7%, hono 8.4%), IQR ✗ (hono 10.7%, ink 12.3%, rich 5.7%, faker 5.5%).

## Threshold Level — All Corpora

Era-9 thresholds (max of 100 samples):

| Corpus | Era-9 Thr | Primary Thr | Fallback A Thr | Fallback B Thr | IQR Thr | IQR Δ |
|:---|---:|---:|---:|---:|---:|---:|
| fastapi | 5.278 | 3.486 | 5.307 | 3.543 | 5.159 | −0.119 |
| rich | 4.164 | 3.146 | 4.647 | 3.152 | 4.493 | +0.329 |
| faker | 5.211 | 3.887 | 5.496 | 3.856 | 5.371 | +0.160 |
| hono | 4.277 | 3.976 | 5.638 | 3.806 | 5.713 | **+1.436** |
| ink | 4.826 | 3.543 | 4.993 | 3.543 | 5.903 | **+1.077** |
| faker-js | 4.773 | 4.128 | 4.861 | 4.169 | 6.305 | **+1.532** |

Key observations:
- p95 consistently produces thresholds ~1.3–1.8 points below era-9 max, regardless of n_cal.
- max with n=300 (Fallback A) closely matches era-9 thresholds for most corpora, but hono's max jumped +1.36 points (4.277→5.638).
- IQR (k=2.5) sits close to era-9 for fastapi/rich/faker but balloons significantly for hono (+1.44), ink (+1.08), and faker-js (+1.53). These are the corpora with the smallest calibration pools relative to corpus size, where p75 is least stable.

## FP Rate — All Corpora

| Corpus | Era-9 FP | Primary FP | Fallback A FP | Fallback B FP | IQR FP |
|:---|---:|---:|---:|---:|---:|
| fastapi | 0.8% | 4.4% | 0.3% | 6.7% | 0.5% |
| rich | 0.8% | 1.7% | 0.6% | 2.2% | 0.6% |
| faker | 1.2% | 3.7% | 0.7% | 3.9% | 0.7% |
| hono | 0.5% | 1.3% | 0.2% | 2.1% | 0.3% |
| ink | 0.4% | 2.5% | 0.4% | 2.5% | 0.2% |
| faker-js | 1.0% | 1.5% | 1.0% | 1.4% | 0.1% |

Gate 5 (no corpus FP > 1.5%): Primary ✗ (fastapi/rich/faker/ink), Fallback A ✓, Fallback B ✗ (all but faker-js), IQR ✓.

## Recall — All Corpora

| Corpus | Era-9 Recall | Primary Recall | Fallback A Recall | Fallback B Recall | IQR Recall |
|:---|---:|---:|---:|---:|---:|
| fastapi | 91.7% | 91.7% | 91.7% | 93.5% | 91.7% |
| rich | 95.0% | 95.0% | 95.0% | 95.0% | 95.0% |
| faker | 95.0% | 95.0% | 95.0% | 95.0% | 95.0% |
| hono | 78.3% | 83.3% | 71.7% | 83.3% | 71.7% |
| ink | 93.3% | 100.0% | 93.3% | 100.0% | 86.7% |
| faker-js | 53.3% | 60.0% | 53.3% | 60.0% | 48.3% |
| **Avg** | **84.43%** | **87.5%** | **83.3%** | **87.8%** | **81.4%** |

Gate 4 (avg recall ≥ 83.43%): Primary ✓, Fallback A ✗ (83.3%), Fallback B ✓, IQR ✗ (81.4%, −3.0pp).
Gate 6 (no corpus regresses > 2pp): Primary ✓, Fallback A ✗ (hono −6.6pp), Fallback B ✓, IQR ✗ (hono −6.6pp, ink −6.6pp, faker-js −5.0pp).

## Verdict Preservation vs Era-9 (115 fixtures)

| Config | Same verdict | Preservation % | Regressions | Improvements | Reason-only changes |
|:---|---:|---:|---:|---:|---:|
| Primary | 105/115 | 91.3% | 0 | 3 | 7 (call_receiver→bpe) |
| Fallback A | 108/115 | 93.9% | 1 (hono_routing_3) | 0 | 6 (bpe→call_receiver) |
| Fallback B | 102/115 | 88.7% | 0 | 4 | 9 (call_receiver→bpe) |
| IQR (k=2.5) | ~112/115 | ~97.4% | ~3 (hono ×1, ink ×1, faker-js ×1) | 0 | — |

Gate 3 (verdict preservation ≥ 95%): percentile configs all fail; IQR passes (~97.4%).

Notes:
- All regressions in Primary and Fallback B are zero (no catch→miss flips).
- Reason-only changes (call_receiver→bpe or vice versa, flagged stays True) occur because the threshold shift moves some fixtures across the call_receiver/bpe boundary. These are not detection regressions.
- Fallback A's single regression (hono_routing_3: bpe→none) is a genuine miss caused by hono's inflated threshold (5.638 vs era-9 4.277).
- The reason-only change rate tracks threshold distance from era-9: Primary (−1.3 to −1.8) → 7 flips; Fallback B (same threshold levels as Primary) → 9 flips; Fallback A (+1.36 for hono) → 6 flips.
- IQR verdict preservation is high (~97.4%) because its threshold is above era-9 for most corpora — fewer controls are FP but some breaks are newly missed (regressions).

## Complete Gate Matrix

| Gate | Threshold | Primary | Fallback A | Fallback B | IQR (k=2.5) |
|:---|:---|:---:|:---:|:---:|:---:|
| 1 — ink CV | <4% | ✓ 0.0% | ✓ 0.0% | ✓ 0.0% | ✗ 12.3% |
| 2 — all CV | <5% | ✓ | ✓ | ✗ fastapi 12.7% | ✗ hono 10.7% |
| 3 — verdicts | ≥95% | ✗ 91.3% | ✗ 93.9% | ✗ 88.7% | ✓ ~97.4% |
| 4 — avg recall | ≥83.43% | ✓ 87.5% | ✗ 83.3% | ✓ 87.8% | ✗ 81.4% |
| 5 — FP | ≤1.5% | ✗ fastapi 4.4% | ✓ | ✗ fastapi 6.7% | ✓ |
| 6 — no recall >2pp | per corpus | ✓ | ✗ hono −6.6pp | ✓ | ✗ hono/ink/faker-js |

**No config clears all gates. Era 10 fails under the pre-registered four-config design space.**

Note on IQR fallbacks: The pre-registered Fallback A (k=2.0) targets "threshold too high" failures on Gate 5/6. Gate 6 does fail in that direction for IQR. However, the primary failure is Gates 1 and 2 (CV), which k=2.0 cannot fix — the root cause is p75 quantile instability with small pools, not k-level. Running Fallback A would lower the threshold but preserve the variance problem. No fallback run was executed.

## Root Cause Analysis

The BPE calibration score distribution has a heavy upper tail. The 95th percentile sits ~1.3–1.8 points below the maximum, and this gap is densely populated by control (non-argot) hunks. Dropping the threshold into this region inflates FP rates severely.

The max estimator avoids this gap but has two problems:
1. **Variance (existing era-9 issue):** max is a single-point estimator, sensitive to one outlier per seed.
2. **Upward bias with more samples (new finding):** increasing n_cal from 100→300 increases the expected max, because more samples → higher probability of drawing a rare high-scoring hunk. For hono, this bias was +1.36 points, enough to miss a fixture.

Simple percentile estimators cannot simultaneously satisfy all gates because:
- p95 fixes variance but drops the threshold too far → FP gate fails.
- max(n=100) preserves threshold level but has high variance → ink CV gate fails (era-9 problem).
- max(n=300) reduces variance but introduces upward bias → hono recall gate fails.

The IQR estimator (p75 + k×IQR) was a structurally different approach: margin-based, data-derived, expected to scale with corpus spread. **It failed on the opposite side from the hypothesis.** Rather than reducing CV, IQR increased ink CV from 6.9% to 12.3% and hono from 3.0% to 10.7%. The mechanism:
- p75 is itself a single-quantile estimator subject to the same seed-to-seed sampling noise as max
- With n_cal=100 from a small corpus, p75 varies substantially across seeds
- The IQR multiplier (k×IQR) amplifies this variance rather than dampening it
- For small-pool corpora (ink: 16,678 real-PR hunks; hono: 54,717), IQR is strictly worse than max on CV

Additionally, IQR inflated the threshold above era-9 for the three smallest-pool corpora (hono +1.44, ink +1.08, faker-js +1.53), causing recall regressions beyond the 2pp gate.

The IQR design hypothesis assumed that corpus spread (IQR) would tighten the threshold distribution. In practice, for small-pool corpora, the p75 and IQR point estimates are themselves unstable, making the combination more volatile than max alone.

## Era-11 Candidate Approaches

The four-config landscape now covers: fixed percentile with more samples (p95+300), max with more samples (max+300), fixed percentile with original samples (p95+100), and margin-based estimator (IQR+100). All fail. The failure mode is consistent: **any single-draw quantile estimator (max, p75, p95) with n_cal=100 inherits seed-to-seed instability from a small calibration pool**.

To resolve these constraints, a better estimator must be:
1. **Robust to outliers** (like p95) — not dominated by a single extreme value.
2. **Unbiased relative to era-9 threshold level** — so control hunks don't flood the FP gate.
3. **Stable across seeds** — low CV even from small pools.

The single-quantile approach (including IQR) is exhausted. Candidates requiring a different paradigm:
- **Multi-sample aggregation**: calibrate on k independent subsamples of n/k each, average the thresholds — reduces per-seed variance by √k without requiring a larger pool.
- **Explicit bias correction**: fit the expected max(n) as a function of n and correct for it, allowing n_cal=300 with unbiased max.
- **Per-corpus learned threshold percentile**: instead of a fixed p95 or IQR formula, learn the percentile that achieves a target FP rate on a held-out control set.
- **Calibration-aware parity testing**: accept the ~7% CV as intrinsic noise and adjust the parity test methodology to tolerate it (era-10 alternative framing).

## Shipped Deliverables

Despite the negative experimental result, era 10 ships the following infrastructure improvements:

1. **Sampler thin-pool fallback** (`random_hunk_sampler.py`): `sample_hunks` now caps at pool size and emits a `UserWarning` instead of raising `ValueError` when n > available pool. Required by rich corpus (237–238 qualifying hunks < 300).

2. **CLI flags** (`argot_bench/cli.py`): `--n-cal` and `--threshold-percentile` flags added to bench CLI (main + run-one). Enables future era A/B configs without code changes.

3. **Calibration CLI flag** (`calibration/__init__.py`): `--threshold-percentile` added to `argot-calibrate` CLI.

4. **Consistency tests**: `test_defaults_consistent.py` and `test_bench_alpha_defaults.py` extended to lock `threshold_percentile` and `n_cal` defaults across all layers, preventing future silent drift.

Defaults remain at era-9 values: n_cal=100, threshold_percentile=None (max).
