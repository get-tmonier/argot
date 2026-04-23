# AST contrastive treelets: the in-domain FastAPI victory

## Setup

Phase 13 introduced `ContrastiveAstTreeletScorer` — depth-3 AST treelets,
`max` aggregation, log-ratio of hunk treelet frequency against a
repo-specific `model_A` over stdlib `model_B`. The hypothesis: where BPE
captures "what words are rare" in a repo, treelets capture "what *shapes*
are rare". `model_A` was built from 20 FastAPI control files; `model_B`
was the generic CPython 3.13 stdlib treelet distribution. The bakeoff
re-used the 51-fixture FastAPI set (31 breaks + 20 controls) so results
were directly comparable to the Phase 12 `tfidf_anomaly` winner
(AUC 0.6968). A full three-tier Stage-2 gauntlet — smoke, LOO over 20
control files, and Django wrong-contrast substitution — gated promotion.

## Results

In-domain `ast_contrastive_max` AUC 0.9742 — beats `tfidf_anomaly` by
+0.27, with paired-bootstrap CI [+0.1371, +0.4242]. Eight of nine
categories scored 1.0000 (`async_blocking`, `background_tasks`,
`dependency_injection`, `downstream_http`, `framework_swap`, `routing`,
`serialization`, `validation`); `exception_handling` landed at 0.8333.

| gate | metric | value |
|:---|:---|---:|
| Smoke | overall AUC | 0.9742 |
| LOO stability (20 controls) | min AUC | 0.9315 |
| LOO stability (20 controls) | mean AUC | 0.9619 |
| Wrong-contrast (Django `model_A`) | AUC | 0.5323 |
| Wrong-contrast | Δ vs correct contrast | −0.4419 |

The scorer passed all three Stage-2 gates. The Django substitution check
was the most persuasive: swapping `model_A` to a different framework
collapses AUC to near-chance, so the signal clearly came from FastAPI
idioms, not a generic "Python-looking" bias.

## Interpretation

On FastAPI in-domain the contrastive treelet axis looked dominant —
better than anything prior, zero training cost, and the gate battery
ruled out the obvious artefacts. Promotion to cross-domain validation
was the only defensible next move, which is exactly where the story
turned.
