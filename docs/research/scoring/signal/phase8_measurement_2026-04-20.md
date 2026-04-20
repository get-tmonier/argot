# Phase 8 Measurement — 2026-04-20

## Setup

- Encoder: `microsoft/unixcoder-base`
- Ensemble: `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)`
- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Static chunks extracted: 3868
- Fixtures: 27 total (19 breaks, 8 controls)
- Command: `uv run python -m argot.research.static_chunk_audit_test`

## AUC

**AUC (ROC): 0.7105**  ✅ AUC ≥ 0.70 — continue to Phase 2

## Threshold Table

| Target FPR | Actual TPR | Score Cutoff |
|:----------:|:----------:|:------------:|
| 0.05 | 0.368 | 3.6806 |
| 0.10 | 0.368 | 3.6806 |
| 0.15 | 0.421 | 3.5167 |
| 0.20 | 0.421 | 3.5167 |

## Score Overlap Histogram (decile buckets)

| Bucket | Break | Control |
|--------|------:|--------:|
| [2.5435, 2.8598) | 2 | 1 |
| [2.8598, 2.9988) | 1 | 2 |
| [2.9988, 3.1362) | 1 | 1 |
| [3.1362, 3.1789) | 2 | 1 |
| [3.1789, 3.2256) | 1 | 1 |
| [3.2256, 3.4666) | 3 | 0 |
| [3.4666, 3.5337) | 2 | 1 |
| [3.5337, 3.7063) | 1 | 1 |
| [3.7063, 3.8623) | 3 | 0 |
| [3.8623, 4.3331) | 3 | 0 |

## Score Percentiles

**Break scores:**
min=2.5435  p25=3.1705  median=3.4657  p75=3.7467  p95=4.0255  max=4.3331

**Control scores:**
min=2.7012  p25=2.9944  median=3.0857  p75=3.2640  p95=3.5720  max=3.6019
