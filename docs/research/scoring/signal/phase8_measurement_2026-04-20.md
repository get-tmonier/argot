# Phase 8 Measurement — 2026-04-20

## Setup

- Encoder: `microsoft/unixcoder-base`
- Ensemble: `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)`
- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Static chunks extracted: 3868
- Fixtures: 27 total (19 breaks, 8 controls)
- Command: `uv run python -m argot.research.static_chunk_audit_test`

## AUC

**AUC (ROC): 0.7368**  ✅ AUC ≥ 0.70 — continue to Phase 2

## Threshold Table

| Target FPR | Actual TPR | Score Cutoff |
|:----------:|:----------:|:------------:|
| 0.05 | 0.316 | 3.5399 |
| 0.10 | 0.316 | 3.5399 |
| 0.15 | 0.526 | 3.4116 |
| 0.20 | 0.526 | 3.4116 |

## Score Overlap Histogram (decile buckets)

| Bucket | Break | Control |
|--------|------:|--------:|
| [2.7317, 2.8999) | 2 | 1 |
| [2.8999, 2.9792) | 1 | 2 |
| [2.9792, 3.0780) | 0 | 2 |
| [3.0780, 3.1741) | 2 | 1 |
| [3.1741, 3.3196) | 2 | 0 |
| [3.3196, 3.4075) | 2 | 1 |
| [3.4075, 3.5200) | 3 | 0 |
| [3.5200, 3.5396) | 1 | 1 |
| [3.5396, 3.8229) | 3 | 0 |
| [3.8229, 4.3189) | 3 | 0 |

## Score Percentiles

**Break scores:**
min=2.7317  p25=3.1640  median=3.4116  p75=3.5647  p95=3.8785  max=4.3189

**Control scores:**
min=2.8380  p25=2.9691  median=3.0281  p75=3.1999  p95=3.4906  max=3.5387
