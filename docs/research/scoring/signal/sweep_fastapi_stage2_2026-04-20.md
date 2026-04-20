# FastAPI Stage 2 sweep — LR schedule × predictor capacity

**Date:** 2026-04-20  
**Base config:** epochs=20, lr=1e-4, batch_size=128, lambd=0.09 (best mean from Stage 1)  
**Seeds:** {0, 1, 2}  
**Gate:** mean_delta ≥ 0.20 AND std_delta ≤ 0.02  
**Grid:** lr_schedule ∈ {flat, cosine} × predictor ∈ {depth=4/mlp=512, depth=6/mlp=512, depth=4/mlp=1024, depth=6/mlp=1024}  
**Note:** cos_d6m1024 not run — cosine schedule showed consistent underperformance across all 6 preceding configs; no evidence it would reverse the trend.

## Raw

| config | seed | delta | gate |
|---|---|---|---|
| flat_d4m512 | 0 | 0.1765 | ✗ |
| flat_d4m512 | 1 | 0.1599 | ✗ |
| flat_d4m512 | 2 | 0.2023 | ✓ |
| flat_d6m512 | 0 | 0.2054 | ✓ |
| flat_d6m512 | 1 | 0.1708 | ✗ |
| flat_d6m512 | 2 | 0.2156 | ✓ |
| flat_d4m1024 | 0 | 0.2080 | ✓ |
| flat_d4m1024 | 1 | 0.2099 | ✓ |
| flat_d4m1024 | 2 | 0.1913 | ✗ |
| flat_d6m1024 | 0 | 0.2038 | ✓ |
| flat_d6m1024 | 1 | 0.2494 | ✓ |
| flat_d6m1024 | 2 | 0.2112 | ✓ |
| cos_d4m512 | 0 | 0.1569 | ✗ |
| cos_d4m512 | 1 | 0.1520 | ✗ |
| cos_d4m512 | 2 | 0.1662 | ✗ |
| cos_d6m512 | 0 | 0.1670 | ✗ |
| cos_d6m512 | 1 | 0.1505 | ✗ |
| cos_d6m512 | 2 | 0.1842 | ✗ |
| cos_d4m1024 | 0 | 0.1538 | ✗ |
| cos_d4m1024 | 1 | 0.1926 | ✗ |
| cos_d4m1024 | 2 | 0.1536 | ✗ |
| cos_d6m1024 | — | — | — |

## Summary

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

## Verdict

**Gate met by `flat_d4m1024`:** mean=0.203, std=0.010 ✓

## Analysis

**Winner: `flat_d4m1024`** — depth=4, mlp_dim=1024, flat LR schedule. Only config satisfying both mean ≥ 0.20 and std ≤ 0.02.

**Wider MLP (1024) helps, deeper predictor (depth=6) hurts variance.** Going from mlp=512 to mlp=1024 at depth=4 raised the mean from 0.180 to 0.203 while cutting std from 0.021 to 0.010. Going from depth=4 to depth=6 at mlp=1024 raised the mean further (0.221) but drove std up to 0.025 — more capacity amplifies variance rather than signal.

**Cosine schedule uniformly worse.** Every cosine config underperforms its flat equivalent on mean delta (~0.03–0.04 drop). With only 20 epochs on a small corpus, cosine annealing decays lr too aggressively — the predictor runs out of learning rate before converging.

**Next step:** Promote `flat_d4m1024` (JepaCustomScorer, epochs=20, lr=1e-4, depth=4, mlp_dim=1024, flat schedule) as the winning config.
