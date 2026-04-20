# Phase 9 — AST Structural Scorer — 2026-04-21

## Setup

- Encoder: `microsoft/unixcoder-base`
- JEPA ensemble: `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)`
- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Static chunks: 3868 total, 1089 core used for training
- Fixtures: 27 total (19 breaks, 8 controls)
- AST variants: loglik, zscore, oov (fully corpus-derived, no hand-crafted rules)
- Blend weights tested: 0.25/0.50/0.75 fraction AST
- Command: `uv run python -m argot.research.ast_structural_audit_test`

## Headline Results

### Raw scores

| Scorer | AUC | Delta (break−ctrl) |
|--------|----:|-------------------:|
| jepa | 0.7368 | +0.2898 |
| ast_ll | 0.6118 | +24.1747 |
| ast_zscore | 0.5329 | +0.8741 |
| ast_oov | 0.6974 | +11.1579 |

### Z-normalized scores (deltas comparable across rows)

| Scorer | AUC | Delta (z-score units) |
|--------|----:|----------------------:|
| jepa (z) | 0.7368 | +0.7910 |
| jepa+ast_ll@0.25 | 0.6974 | +0.6285 |
| jepa+ast_ll@0.50 | 0.6776 | +0.4660 |
| jepa+ast_ll@0.75 | 0.6447 | +0.3035 |
| jepa+ast_zscore@0.25 | 0.7105 | +0.6276 |
| jepa+ast_zscore@0.50 | 0.6711 | +0.4641 |
| jepa+ast_zscore@0.75 | 0.5921 | +0.3007 |
| jepa+ast_oov@0.25 | 0.7368 | +0.7466 |
| jepa+ast_oov@0.50 | 0.7697 | +0.7023 |
| jepa+ast_oov@0.75 | 0.7632 | +0.6579 |

## Per-category AUC

| Category | jepa | ast_ll | ast_zscore | ast_oov | jepa (z) | jepa+ast_ll@0.25 | jepa+ast_ll@0.50 | jepa+ast_ll@0.75 | jepa+ast_zscore@0.25 | jepa+ast_zscore@0.50 | jepa+ast_zscore@0.75 | jepa+ast_oov@0.25 | jepa+ast_oov@0.50 | jepa+ast_oov@0.75 |
|----------|------:|------:|------:|------:|------:|------:|------:|------:|------:|------:|------:|------:|------:|------:|
| async_blocking | 0.5000 | 1.0000 | 1.0000 | 1.0000 | 0.5000 | 0.5000 | 1.0000 | 1.0000 | 0.5000 | 1.0000 | 1.0000 | 0.5000 | 1.0000 | 1.0000 |
| background_tasks | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| dependency_injection | 0.5000 | 0.5000 | 1.0000 | 1.0000 | 0.5000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| downstream_http | 1.0000 | 1.0000 | 0.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| exception_handling | 0.6667 | 0.3333 | 0.6667 | 1.0000 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | 0.6667 |
| framework_swap | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| routing | 1.0000 | 0.5000 | 0.5000 | 0.5000 | 1.0000 | 1.0000 | 0.5000 | 0.5000 | 1.0000 | 0.5000 | 0.5000 | 1.0000 | 1.0000 | 1.0000 |
| serialization | 0.5000 | 0.5000 | 0.0000 | 0.0000 | 0.5000 | 0.5000 | 1.0000 | 0.5000 | 0.5000 | 0.0000 | 0.0000 | 0.5000 | 0.5000 | 0.0000 |
| validation | 1.0000 | 1.0000 | 0.6667 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |

## Per-category Delta

| Category | jepa | ast_ll | ast_zscore | ast_oov | jepa (z) | jepa+ast_ll@0.25 | jepa+ast_ll@0.50 | jepa+ast_ll@0.75 | jepa+ast_zscore@0.25 | jepa+ast_zscore@0.50 | jepa+ast_zscore@0.75 | jepa+ast_oov@0.25 | jepa+ast_oov@0.50 | jepa+ast_oov@0.75 |
|----------|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|-------:|
| async_blocking | +0.0755 | +190.2624 | +10.1479 | +25.0000 | +0.2062 | +0.4321 | +0.6581 | +0.8840 | +0.5533 | +0.9003 | +1.2474 | +0.4983 | +0.7905 | +1.0826 |
| background_tasks | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| dependency_injection | +0.2279 | +73.5113 | +3.9797 | +18.0000 | +0.6220 | +0.5737 | +0.5254 | +0.4771 | +0.6228 | +0.6237 | +0.6245 | +0.7139 | +0.8059 | +0.8978 |
| downstream_http | +0.8830 | +42.3751 | -3.6434 | +14.5000 | +2.4102 | +1.8695 | +1.3287 | +0.7880 | +1.6646 | +0.9189 | +0.1732 | +2.0070 | +1.6038 | +1.2006 |
| exception_handling | +0.1384 | -0.0785 | -0.1806 | +5.3333 | +0.3779 | +0.2833 | +0.1887 | +0.0941 | +0.2763 | +0.1747 | +0.0732 | +0.3567 | +0.3356 | +0.3144 |
| framework_swap | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| routing | +0.3357 | -157.8140 | -1.9641 | +4.0000 | +0.9162 | +0.4570 | -0.0022 | -0.4614 | +0.6100 | +0.3038 | -0.0024 | +0.7422 | +0.5681 | +0.3940 |
| serialization | +0.1703 | -26.6264 | -5.9958 | -15.5000 | +0.4648 | +0.3098 | +0.1547 | -0.0003 | +0.1131 | -0.2386 | -0.5904 | +0.1355 | -0.1938 | -0.5230 |
| validation | +0.5061 | +194.5473 | +5.2922 | +30.3333 | +1.3815 | +1.3198 | +1.2582 | +1.1966 | +1.2440 | +1.1065 | +0.9690 | +1.4531 | +1.5247 | +1.5964 |

## Top-20 Corpus Composition (per AST variant)

| Scorer | core | test | docs_scripts |
|--------|-----:|-----:|-------------:|

## Discussion

Best AST variant standalone: `ast_oov` AUC=0.6974 (JEPA baseline: 0.7368). Best blend: `jepa+ast_oov@0.50` AUC=0.7697.
`exception_handling`: JEPA=0.6667  ast_oov=1.0000  blend=0.6667.
`async_blocking`: JEPA=0.5000  ast_oov=1.0000  blend=1.0000.
`dependency_injection`: JEPA=0.5000  ast_oov=1.0000  blend=1.0000.
