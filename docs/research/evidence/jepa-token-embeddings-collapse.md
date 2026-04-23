# JEPA token embeddings: shuffled up, cross-repo collapsed at large

## Setup

Phase 5 replaced the sparse TF-IDF encoder with a learned
`nn.Embedding` table (8000 token types, dim 128, masked mean pool over
seq_len=256, then a linear projection to 192). Vocab is built per-run from
the training split (`min_count=2`, drops singletons). Total params ~1.05M,
~4 MB fp32. Phase 5 discipline: `char_ngrams`, `context_after`, and
`adaptive_epochs` are all **off** — only the encoder changes.

Per-bucket corpora (small 3k, medium 7k, large 20k), three seeds each.
Hypothesis: dense embeddings should capture token co-occurrence in a way
sparse TF-IDF cannot, with the expected failure mode a small-data
regression vs char_ngrams.

## Results

| bucket | shuffled       | cross-repo     | injected       |
|:-------|:---------------|:---------------|:---------------|
| small  | 0.639 ± 0.012  | 0.445 ± 0.090  | 0.776 ± 0.070  |
| medium | 0.701 ± 0.014  | 0.665 ± 0.026  | 0.761 ± 0.024  |
| large  | 0.697 ± 0.019  | **0.457 ± 0.076** | 0.687 ± 0.005 |

Shuffled Δ vs baseline: +0.139 / +0.200 / +0.060. Token embeddings beat
char_ngrams on shuffled at small (+0.139) and medium (+0.046), and roughly
match at large (+0.013).

Cross-repo at large collapses to **0.457 ± 0.076** — an **−0.087**
regression vs the TF-IDF baseline (0.544) and −0.191 vs char_ngrams
(0.648). Small is stuck near baseline (0.445) with high variance;
medium (0.665) edges char_ngrams.

## Interpretation

Token embeddings won the primary metric (shuffled AUC) but collapsed
cross-repo at large — the failure mode is clear. The embedding table is
keyed on raw token texts (`myVariableName`, `mergeOptions`) which are
repo-specific surface forms. On a foreign repo most tokens are OOV, map
to `<unk>`, and produce near-random scores. The model fingerprints which
repo it was trained on. This framed the Phase 6 follow-up: move to BPE
subwords, which are shared across repos, and see whether the OOV collapse
recovers at large scale.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/phases-1-6/11-token-embeddings.md`. Re-written here
for clarity, not copied.*
