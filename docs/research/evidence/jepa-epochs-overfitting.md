# JEPA epochs sweep: medium helped, large overfit

## Setup

Phase 3 sweep on training length: baseline runs with `epochs=20`; variant
runs with `epochs=200`. No code change — `--epochs` was already a parameter
on `argot-engine corpus benchmark`. Corpus buckets and protocol are the
same as the Phase 2 sizing study (small 3k, medium 7k, large 20k; one TS
repo + one Py repo; three seeds each). The xlarge bucket was dropped for
compute reasons.

Hypothesis: more gradient passes should help the model converge at sizes
where signal exists (≥ 20k), with no effect at small/medium where the
20-epoch baseline is already random.

## Results

| bucket | shuffled Δ | cross-repo Δ | injected Δ |
|:-------|:-----------|:-------------|:-----------|
| small  | +0.138     | −0.016       | +0.163     |
| medium | +0.170     | **+0.213**   | +0.309     |
| large  | +0.060     | **−0.106**   | +0.037     |

Cross-repo absolute values: small 0.430 ± 0.079, medium 0.608 ± 0.008,
large 0.438 ± 0.012 (baseline large was 0.544).

The medium bucket at 200 epochs produces the largest single-metric
cross-repo gain seen anywhere in Phase 3 (+0.213) — cross at medium (0.608)
now exceeds the Phase 2 large baseline (0.544). The large bucket regresses
by −0.106 at the same epoch count: the model memorises the home repo so
thoroughly that it actively misfires on foreign code.

## Interpretation

The right number of epochs is not fixed — it depends on corpus size. The
finding prompted an adaptive rule, `epochs = 1.4M / n` (clamped to
[20, 200]), giving ~200 at 7k and ~20 at 70k. This was the era's first
concrete lesson that knobs couple to data volume: a uniform default that
helps medium-sized repos will hurt large ones. It also set up the
combined-defaults run in Phase 4, which promoted adaptive epochs alongside
char n-grams and context_after.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/phases-1-6/05-epochs.md`. Re-written here for
clarity, not copied.*
