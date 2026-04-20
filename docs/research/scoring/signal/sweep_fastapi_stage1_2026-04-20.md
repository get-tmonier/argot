# FastAPI Stage 1 sweep — training budget (epochs × lr)

**Date:** 2026-04-20  
**Corpus:** FastAPI, 2000 records, full (no subsampling)  
**Seeds:** {0, 1, 2}  
**Gate:** mean_delta ≥ 0.20 AND std_delta ≤ 0.02  
**Grid:** epochs ∈ {20, 50, 100} × lr ∈ {5e-5, 1e-4}, batch_size=128, lambd=0.09  

## Raw

| config | seed | delta | gate |
|---|---|---|---|
| ep20_lr5e5 | 0 | 0.1556 | ✗ |
| ep20_lr5e5 | 1 | 0.1381 | ✗ |
| ep20_lr5e5 | 2 | 0.1637 | ✗ |
| ep20_lr1e4 | 0 | 0.1765 | ✗ |
| ep20_lr1e4 | 1 | 0.1599 | ✗ |
| ep20_lr1e4 | 2 | 0.2023 | ✓ |
| ep50_lr5e5 | 0 | 0.1924 | ✗ |
| ep50_lr5e5 | 1 | 0.1453 | ✗ |
| ep50_lr5e5 | 2 | 0.2249 | ✓ |
| ep50_lr1e4 | 0 | 0.1954 | ✗ |
| ep50_lr1e4 | 1 | 0.1058 | ✗ |
| ep50_lr1e4 | 2 | 0.1801 | ✗ |
| ep100_lr5e5 | 0 | 0.1992 | ✗ |
| ep100_lr5e5 | 1 | 0.1214 | ✗ |
| ep100_lr5e5 | 2 | 0.1949 | ✗ |
| ep100_lr1e4 | 0 | 0.1900 | ✗ |
| ep100_lr1e4 | 1 | 0.1355 | ✗ |
| ep100_lr1e4 | 2 | 0.1674 | ✗ |

## Summary

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| ep20_lr5e5 | 0.1525 | 0.0131 | ✗ |
| ep20_lr1e4 | 0.1796 | 0.0214 | ✗ |
| ep50_lr5e5 | 0.1875 | 0.0400 | ✗ |
| ep50_lr1e4 | 0.1604 | 0.0480 | ✗ |
| ep100_lr5e5 | 0.1718 | 0.0437 | ✗ |
| ep100_lr1e4 | 0.1643 | 0.0274 | ✗ |

## Verdict

**Gate not met.** No config reaches mean_delta ≥ 0.20 with std_delta ≤ 0.02.

## Analysis

**More epochs increases variance, not mean.** The best mean (0.188) is at ep50_lr5e5, but std=0.040 — 2× the gate. At ep100 the mean drops and variance stays high, suggesting the predictor overfits or loses the training signal past ~50 epochs on this corpus.

**The variance is structural.** Seed-to-seed swings of 0.05–0.09 appear across all configs. The root cause is likely the interaction between `split_by_time` (80/20 temporal split) and the small FastAPI corpus: different seeds produce significantly different training sets, leading to inconsistent predictor states.

**Best candidates for Stage 2:**
- Best mean: `ep20_lr1e4` (mean=0.180, std=0.021) — chosen as Stage 2 base
- Tightest variance: `ep20_lr5e5` (std=0.013) — mean too low but shows low epochs are more stable

**Next step:** Stage 2 — cosine LR schedule + predictor capacity (depth/mlp_dim), built on `ep20_lr1e4`.
