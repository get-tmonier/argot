# JEPA sizing study: random below 20k, plateau above

## Setup

Phase 2 baseline run of the default JEPA pipeline (TF-IDF word tokens,
`epochs=20`, `batch_size=128`) across four corpus sizes. Each bucket pairs
one TypeScript repo with one Python repo of similar record count — small
(3k), medium (7k), large (20k), xlarge (60k). For each bucket we
reservoir-sample to the target, train on 80% of the home repo, and evaluate
on the 20% held-out tail plus three adversarial negative sets. Three seeds
per bucket.

The question: at what corpus size does argot's scoring model stop being
effectively random?

## Results

| size (bucket) | shuffled AUC   | cross-repo AUC | injected AUC   |
|--------------:|:---------------|:---------------|:---------------|
|         3,000 | 0.500 ± 0.000  | 0.446 ± 0.028  | 0.450 ± 0.020  |
|         7,000 | 0.501 ± 0.001  | 0.395 ± 0.005  | 0.398 ± 0.011  |
|        20,000 | 0.637 ± 0.006  | 0.544 ± 0.015  | 0.631 ± 0.010  |
|        60,000 | 0.707 ± 0.002  | 0.549 ± 0.017  | 0.695 ± 0.009  |

Below 20k: shuffled AUC is exactly 0.500 — the model learns nothing.
Cross-repo and injected AUC sit *below* 0.5, meaning predictions are
actively misleading on unseen styles (a token-memorisation failure mode).

Between 7k and 20k: the phase transition. Shuffled jumps to 0.637 and
injected to 0.631 — both well above random. Cross-repo only reaches 0.544.

At 60k: shuffled 0.707, injected 0.695; cross-repo still stuck near 0.55.

## Interpretation

Two facts set the terms of the JEPA era. First, ~20k records is the floor
for a non-random model — any usable argot build needs a corpus of at least
that size. Second, even at 60k the plateau is modest: shuffled ceiling ~0.71,
cross-repo ~0.55. The hypothesis the era would test is that this plateau is
a *tuning* problem — right objective, wrong inputs and encoder — solvable
with sweeps along context, capacity, training length, and representation.
