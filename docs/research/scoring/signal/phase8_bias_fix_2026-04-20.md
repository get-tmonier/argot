# Phase 8 Bias Fix — 2026-04-20

## Setup

- Encoder: `microsoft/unixcoder-base`
- Ensemble: `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)`
- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Approach: exclude-tests-from-train
- Training corpus: 1089 core chunks (2779 test chunks excluded)
- Scoring corpus: 3868 chunks (all, including test)
- Command: `uv run python -m argot.research.static_chunk_audit_test`

## Go/No-go

**⚠️ borderline — top-20 core or AUC marginal**

## Top-20 Composition

| File class | Count |
|------------|------:|
| core | 3 |
| test | 12 |
| docs_scripts | 5 |

## Core-only-fixture AUC (non-framework_swap)

**AUC: 0.7266**  (threshold ≥ 0.65 to continue)

## Overall Delta

break mean − ctrl mean = **0.2898**  (threshold ≥ 0.20 before stratified fallback)

## Per-category Delta

| Category | Δ | Break μ | Ctrl μ | #Break | #Ctrl |
|----------|--:|--------:|-------:|-------:|------:|
| async_blocking | +0.0755 | 3.0168 | 2.9413 | 2 | 1 |
| background_tasks | n/a | 3.5895 | n/a | 1 | 0 |
| dependency_injection | +0.2279 | 3.5342 | 3.3063 | 1 | 2 |
| downstream_http | +0.8830 | 3.8652 | 2.9822 | 2 | 1 |
| exception_handling | +0.1384 | 3.1168 | 2.9784 | 3 | 1 |
| framework_swap | n/a | 3.4202 | n/a | 3 | 0 |
| routing | +0.3357 | 3.4684 | 3.1327 | 2 | 1 |
| serialization | +0.1703 | 3.5717 | 3.4014 | 2 | 1 |
| validation | +0.5061 | 3.3441 | 2.8380 | 3 | 1 |
