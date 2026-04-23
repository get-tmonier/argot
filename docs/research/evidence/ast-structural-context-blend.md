# AST structural context blend: JEPA alone held the lead

## Setup

Phase 10 tested whether AST-derived structural features could add
complementary signal to JEPA on the freshly expanded 51-fixture set
(9 categories). Five configs were run against
`microsoft/unixcoder-base` with `EnsembleInfoNCE(n=3, beta=0.1,
tau=0.1)` over 1089 core-only chunks: `baseline_9` (plain AST),
`ctx` (parent-scope features), `cooc` (within-file co-occurrence),
`full` (ctx + cooc), and `jepa_alone`. Each AST variant was blended
with JEPA at fixed weights 0.25 / 0.50 / 0.75.

## Results

`jepa (z)` AUC 0.6419 was the headline. No AST blend surpassed it; best blend 0.6339.

| scorer | AUC | Δ(z) |
|:---|---:|---:|
| `jepa (z)` (winner) | 0.6419 | +0.4491 |
| `jepa+ast_structural_ctx_zscore@0.25` | 0.6339 | +0.3933 |
| `jepa+ast_structural_zscore@0.25` | 0.6290 | +0.4068 |
| `jepa+ast_structural_cooc_zscore@0.25` | 0.6290 | +0.3950 |
| `ast_structural_ll` (standalone best) | 0.5661 | +36.89 |
| `ast_structural_cooc_ll` (standalone worst) | 0.4855 | −30.65 |

Phase 9 had reached 0.7697 under `jepa+ast_oov@0.50`, but on the
harder 51-fixture grid the same blend could not reproduce it — the
two previously-n/a categories (`background_tasks`, `framework_swap`)
were now evaluable and the new DI and routing breaks pulled the
average down.

## Interpretation

AST features either matched or degraded JEPA in the blend. Parent
context helped marginally on `dependency_injection` (ctx_zscore
standalone 0.8333 vs baseline 0.6667) but hurt `framework_swap`;
co-occurrence perfectly separated DI (`cooc_oov` AUC 1.0000) but
wrecked serialization. No single AST slice was reliably additive.
The Phase 10 blend refusal was the first clean signal that the
useful signal was living elsewhere — likely in the context selection
itself, which Phase 11 tested next.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase10_structural_context_2026-04-21.md`.
Re-written here for clarity, not copied.*
