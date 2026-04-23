# Density heads on BPE: one mutation axis, no sequential signal

## Setup

Phase 7.2 held the BPE encoder fixed (BpeVocab 8000 + MeanPoolEncoder →
192-dim, same as 7.1) and swapped the JEPA predictor for distance-based
heads: kNN-20 and Gaussian mixtures with 8, 16, and 32 components.
Scoring was density anomaly on hunk embeddings only — mean kNN cosine
distance or `-log p(emb)` — with no context input. Same frozen
corpus as 7.1: 3 seeds × 6 buckets × adaptive epochs. The hypothesis
separated two failure modes: was the JEPA predictor the bottleneck, or
was the BPE representation itself? Decision gate:
`synthetic_auc_mean ≥ 0.85` at medium on ≥ 2 of 3 seeds.

## Results

Best primary metric: **BPE + gmm-8 at 0.581 ± 0.014 (medium-py)** —
0.269 below gate. All four density heads plateaued in the 0.49–0.58
band; swapping the predictor did not move the primary metric out of the
same region 7.1 had found.

Per-mutation AUC at medium revealed a single sharp axis: `case_swap`
hit **0.780 at medium-ts (gmm-16)** and 0.773 (gmm-32), with the GMM
clustering the embedding space by naming convention and case
mutations falling outside the home cluster. The other three mutations
stayed flat (`error_flip`/`quote_flip` still exactly 0.500 as trigger
tokens remained absent; `debug_inject` inconsistent at 0.494–0.593).

Secondary metrics told the important story. `shuffled_auc` collapsed
to **exactly 0.500** across every head and bucket — density heads have
no sequential input, so shuffled tokens sit at the same distance as
real ones. `cross_auc_same_lang` ran 0.65–0.87 at medium/large, far
stronger than 7.1's 0.46–0.65, with **GMM-16 cross_auc_same_lang 0.868
at large-ts** the peak — the BPE density space discriminated repo
identity well.

## Interpretation

The predictor was not the bottleneck: swapping to density anomaly did
not unlock synthetic mutation detection either. What the swap did was
trade sequential signal for repo-identity signal, isolating that
BPE embeddings encode repo-style structure but not fine-grained
mutation-detectability. One real axis (`case_swap`) was recoverable;
nothing else was.
