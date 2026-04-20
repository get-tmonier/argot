# Phase 8 LLM Re-rank Experiment — 2026-04-20

## Setup

- JEPA encoder: `microsoft/unixcoder-base`
- LLM: `Qwen/Qwen2.5-Coder-1.5B-Instruct`
- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Fixtures: 27 (19 breaks, 8 controls)
- Command: `uv run python -m argot.research.llm_rerank_experiment`

## AUC by Blend Weight

| Approach | AUC |
|----------|----:|
| JEPA only         (α=1.00) | 0.7368 |
| 75% JEPA + 25% LLM (α=0.75) | 0.7434 |
| 50% JEPA + 50% LLM (α=0.50) | 0.7632 |
| 25% JEPA + 75% LLM (α=0.25) | 0.7961 |
| LLM only          (α=0.00) | 0.7763 |

**Best: 25% JEPA + 75% LLM (α=0.25) — AUC 0.7961**

## Per-category AUC

| Category | JEPA | LLM | Gain |
|----------|-----:|----:|-----:|
| async_blocking | 0.5000 | 0.5000 | +0.0000 |
| dependency_injection | 0.5000 | 1.0000 | +0.5000 |
| downstream_http | 1.0000 | 0.5000 | -0.5000 |
| exception_handling | 0.6667 | 0.0000 | -0.6667 |
| routing | 1.0000 | 1.0000 | +0.0000 |
| serialization | 0.5000 | 1.0000 | +0.5000 |
| validation | 1.0000 | 1.0000 | +0.0000 |
