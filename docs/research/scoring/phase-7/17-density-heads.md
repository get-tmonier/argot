# Phase 7.2 — Density heads on BPE embeddings

**Branch**: `research/phase-7-honest-eval`
**Status**: complete — decision gate NOT cleared; proceeding to Phase 7.3
**Date**: 2026-04-19

## Setup

- Encoder: BPE (same as Phase 7.1 — BpeVocab 8000 + MeanPoolEncoder → 192-dim)
- Heads: knn-20, gmm-8, gmm-16, gmm-32 (3 seeds each)
- Buckets: 6 = 3 sizes × 2 langs, same frozen corpus as 7.1
- Seeds: 3 per (bucket × head), stratified downsample
- Epochs: adaptive — `min(200, max(20, 1_400_000 // size))`
- Scoring: density anomaly score (mean kNN cosine distance / `-log p(emb)`) on hunk embeddings only
- Outputs: `.argot/research/results-7-2-{knn20,gmm8,gmm16,gmm32}.jsonl`

## Implementation notes

`KnnHead` caps `k` to `min(k, n_train)` to handle small training splits without
error. `GmmHead` uses `reg_covar=1e-3` + `float64` cast to avoid singular
covariance on high-dimensional (192-dim) data at medium/large bucket sizes.

## Primary metric — `synthetic_auc_mean` (mean ± std over 3 seeds)

| bucket     | knn-20        | gmm-8           | gmm-16        | gmm-32        |
|:-----------|:--------------|:----------------|:--------------|:--------------|
| small-py   | 0.510 ± 0.005 | 0.499 ± 0.004   | 0.495 ± 0.005 | 0.498 ± 0.007 |
| small-ts   | 0.507 ± 0.003 | 0.493 ± 0.002   | 0.489 ± 0.005 | 0.491 ± 0.002 |
| medium-py  | 0.531 ± 0.002 | **0.581 ± 0.014** | 0.575 ± 0.009 | 0.532 ± 0.010 |
| medium-ts  | 0.536 ± 0.001 | 0.561 ± 0.008   | 0.573 ± 0.004 | 0.568 ± 0.001 |
| large-py   | 0.515 ± 0.003 | 0.488 ± 0.001   | 0.491 ± 0.001 | 0.495 ± 0.002 |
| large-ts   | 0.508 ± 0.004 | 0.504 ± 0.002   | 0.506 ± 0.002 | 0.510 ± 0.002 |

**Decision gate: NOT cleared.** Target was ≥ 0.85 at medium on ≥ 2/3 seeds.
Best observed: **gmm-8 medium-py = 0.581 ± 0.014** — 0.269 below target.

## Per-mutation AUC at medium (mean over 3 seeds)

| head    | bucket    | case_swap | debug_inject | error_flip | quote_flip |
|:--------|:----------|----------:|-------------:|-----------:|-----------:|
| knn-20  | medium-py | 0.540 | 0.571 | 0.500 | 0.514 |
| knn-20  | medium-ts | 0.602 | 0.542 | 0.500 | 0.500 |
| gmm-8   | medium-py | 0.678 | 0.593 | 0.500 | 0.552 |
| gmm-8   | medium-ts | 0.750 | 0.494 | 0.500 | 0.500 |
| gmm-16  | medium-py | 0.680 | 0.576 | 0.500 | 0.544 |
| gmm-16  | medium-ts | 0.780 | 0.511 | 0.500 | 0.500 |
| gmm-32  | medium-py | 0.597 | 0.521 | 0.500 | 0.510 |
| gmm-32  | medium-ts | 0.773 | 0.498 | 0.500 | 0.500 |

`error_flip` and `quote_flip` remain at exactly 0.500 across all heads and
buckets — same finding as 7.1. Those mutations no-op when the trigger tokens
(`raise`/`throw`, quote characters) are absent from the hunk. They contribute
zero signal and depress every `synthetic_auc_mean` by ~0.125 relative to
what the working mutations achieve. Flagged for Phase 8 mutation redesign.

`case_swap` shows the clearest signal: gmm-16 medium-ts hits 0.780, gmm-32
medium-ts hits 0.773. The GMM clusters the embedding space by naming convention
(vite uses camelCase, typescript-eslint uses PascalCase/camelCase differently),
and case mutations fall outside the home cluster. This is real signal — but it
only works on one mutation axis.

`debug_inject` is weaker and inconsistent: knn-20 picks it up at medium-py
(0.571) but GMM heads do not (0.494–0.593). kNN's local neighbourhood
structure may be more sensitive to injected tokens changing local density.

## Secondary — `shuffled_auc`

Exactly 0.500 across all heads and all buckets. Expected: the density head
scores a hunk by its distance from the training distribution in embedding
space, with no context input. Shuffled hunks come from the same home repo,
so they remain inside the training distribution → score ≈ same → AUC ≈ 0.500.
The density head has no sequential signal (unlike the JEPA predictor in 7.1
which scored 0.57–0.83 on this metric).

## Secondary — `cross_auc_same_lang` (real style signal)

| bucket    | knn-20 | gmm-8 | gmm-16 | gmm-32 |
|:----------|-------:|------:|-------:|-------:|
| small-py  | 0.754  | 0.632 | 0.596  | 0.641  |
| small-ts  | 0.740  | 0.526 | 0.486  | 0.462  |
| medium-py | 0.791  | 0.841 | 0.805  | 0.653  |
| medium-ts | 0.685  | 0.743 | 0.751  | 0.743  |
| large-py  | 0.567  | 0.696 | 0.700  | 0.712  |
| large-ts  | 0.820  | 0.867 | 0.868  | 0.871  |

Strong cross-repo separation (0.65–0.87 at medium/large), substantially higher
than 7.1 (0.46–0.65 for any from-scratch JEPA encoder). The BPE embeddings
+ density head can discriminate *which repo* a hunk came from quite well.
This confirms the embeddings carry style cluster structure — but that structure
is about repo identity, not mutation detectability.

## Interpretation

**The predictor was not the bottleneck.** Swapping JEPA → density head does not
improve synthetic AUC. The four from-scratch encoders in 7.1 and the density
heads in 7.2 all plateau in the 0.49–0.58 band on synthetic AUC, with no head
approaching the 0.85 gate.

What 7.2 adds over 7.1:

1. **Density heads capture cross-repo style better** (0.65–0.87 vs 0.46–0.65
   in 7.1). The GMM clusters the embedding space; kNN neighbourhood distance
   preserves local geometry. Both are better at "which repo is this from?" than
   the JEPA surprise score.

2. **`case_swap` is detectable by GMM at medium-ts** (0.773–0.780). The
   naming-convention axis is encoded in the BPE embeddings, and GMM clusters
   are sensitive to it. But this is the only mutation that works: the other
   three remain invisible.

3. **Density heads trade away sequential signal** (`shuffled_auc` drops from
   0.57–0.83 → 0.500). The JEPA predictor measured token-order coherence;
   density heads do not.

The root cause is unchanged: a BPE encoder trained on 2k–16k records from two
repos does not have the prior knowledge to represent subtle code style
dimensions. The `case_swap` result at medium-ts is an exception because
vite/typescript-eslint have an unusually sharp camelCase contrast that maps
onto the embedding geometry — but even that is 0.22 below gate.

**Best head for 7.4**: gmm-8 (highest overall `synthetic_auc_mean` at medium,
and strong `cross_auc_same_lang`). Carry forward to pair with CodeRankEmbed in
7.4 if 7.3 also fails the gate.

## Next

Proceed to **Phase 7.3** — frozen CodeRankEmbed encoder + existing JEPA head.
The hypothesis is that a pretrained model already knows what `camelCase` and
`snake_case` mean, what a `console.log` injection looks like, and can separate
those in embedding space without any fine-tuning.

## Deferred

- **Mutation redesign**: `error_flip`/`quote_flip` need hunk-aware trigger
  selection. Defer to Phase 8.
- **GMM warm-start from pretrained embeddings**: if 7.4 (CodeRankEmbed +
  density head) is the winner, revisit gmm-8 vs gmm-16 with the richer
  embedding space.
