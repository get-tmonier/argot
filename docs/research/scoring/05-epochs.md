# Technique 3 — epochs (20 → 200)

**Branch**: `research/epochs`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

The baseline benchmark trains for only 20 epochs. More gradient passes should
let the model converge more fully and learn richer style representations,
especially at sizes where signal exists (≥ 20k). No effect expected at
small/medium where the model is random at 20 epochs.

## What changed

No code change — `--epochs` was already a parameter on `argot-engine corpus
benchmark`. The experiment runs with `--epochs 200` vs the baseline of 20.

Note: the DESIGN.md described this as "50 → 200"; the actual benchmark default
is 20 (the 50 figure is the standalone `argot-engine train` default). The
experiment is 20 → 200.

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds, small/medium/large
only (xlarge dropped — see `03-context-after.md §Decision`).

### Shuffled AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.500 ± 0.000  | 0.638 ± 0.013  | +0.138  |
| medium | 0.501 ± 0.001  | 0.671 ± 0.005  | +0.170  |
| large  | 0.637 ± 0.006  | 0.697 ± 0.008  | +0.060  |

### Cross-repo AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.446 ± 0.028  | 0.430 ± 0.079  | −0.016  |
| medium | 0.395 ± 0.005  | 0.608 ± 0.008  | **+0.213** |
| large  | 0.544 ± 0.015  | 0.438 ± 0.012  | **−0.106** |

### Injected AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.450 ± 0.020  | 0.613 ± 0.069  | +0.163  |
| medium | 0.398 ± 0.011  | 0.707 ± 0.018  | +0.309  |
| large  | 0.631 ± 0.010  | 0.668 ± 0.010  | +0.037  |

## Interpretation

**200 epochs has a sweet spot at medium (7k) — and badly overfits at large (20k).**

The results reveal a corpus size × epoch count interaction that is the key
finding of this experiment:

**Small (3k): overfitting.** Shuffled and injected jump massively (+0.138,
+0.163) but cross-repo is flat/negative (−0.016, high variance). The model
memorises the home repo's token distribution but fails to generalise. Same
pattern as Phase 2 small — just with more confident memorisation.

**Medium (7k): genuine learning.** All three metrics improve substantially,
and cross-repo improves the *most* (+0.213). This is the anti-overfitting
signature: the model learned what "this style" looks like well enough to also
recognise "this is not that style" on unseen foreign code. At 7k, 200 epochs
crosses a threshold where gradient passes produce generalisation rather than
memorisation. Cross-repo at medium (0.608) now exceeds the Phase 2 large
baseline (0.544) — matching 20k-record quality from 7k records.

**Large (20k): severe overfitting.** Cross-repo collapses from 0.544 to 0.438
(−0.106). Shuffled and injected improve modestly, but the model has memorised
the home repo so thoroughly at this size × epoch combination that it actively
misfires on foreign code. 200 epochs is too many for 20k records.

## Finding: optimal epochs scales with dataset size

The right number of epochs is not fixed — it depends on how much training data
is available. At 7k, 200 epochs is transformative. At 20k, 20 epochs is
already enough and 200 causes regression.

This implies the baseline epoch count (20) is well-calibrated for large but
too low for medium. A size-adaptive stopping criterion (e.g. early stopping on
a held-out loss, or epochs = f(n_records)) would let the model converge
appropriately at every size.

## Verdict

**Merge for medium-range repos (7k–15k records); do not merge for large.**
A blanket 200-epoch default would help users in the medium range but hurt
large-repo users. The right follow-on is either:
- A size-adaptive default (`epochs = max(20, 1_400_000 // n_records)` is a
  rough heuristic that gives ~200 at 7k and ~20 at 70k), or
- Early stopping on held-out loss.

This technique is the largest single-metric gain observed across all Phase 3
experiments so far (+0.213 cross-repo at medium), but requires careful
application.
