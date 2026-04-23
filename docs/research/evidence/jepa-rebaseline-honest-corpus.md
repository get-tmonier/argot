# JEPA re-baseline on the honest corpus: no encoder clears chance

## Setup

Phase 7 rebuilt the eval around same-language bucket pairs and four
deterministic token-level mutations (`case_swap`, `debug_inject`,
`error_flip`, `quote_flip`), making `synthetic_auc_mean` the primary
metric. Phase 7.1 re-ran every from-scratch encoder from the JEPA era
against the new corpus: `tfidf` (char_wb 3–5), `word_ngrams` (word 1–2),
`token_embed` (mean-pooled embeddings), and `bpe` (BPE + mean-pool). Each
combination ran 3 seeds × 6 buckets (3 sizes × 2 langs), JEPA predictor
head unchanged, adaptive epochs, stratified downsample. Decision gate:
`synthetic_auc_mean ≥ 0.85` at medium on ≥ 2 of 3 seeds.

## Results

Best observed primary metric was word_ngrams + JEPA at
**`synthetic_auc_mean` 0.516 ± 0.007 (medium-py)** — 0.334 below gate.
All four encoders sat in the 0.48–0.52 band across every bucket. BPE was
killed after 7 runs (small buckets + 1 seed of medium-py) when it tracked
the others at the low end.

Per-mutation AUC at medium exposed that two of four mutations were
no-ops: `error_flip` and `quote_flip` landed at exactly **0.500** across
every encoder and bucket, because ~half the held-out hunks lacked the
trigger tokens (`raise`/`throw`, quote characters) the mutations needed.
The working mutations were `case_swap` and `debug_inject`, with best
per-mutation numbers of 0.535 (word_ngrams medium-py `case_swap`) and
0.530 (word_ngrams medium-py `debug_inject`) — still far below gate.

Secondary metrics confirmed the encoders were not totally inert:
`shuffled_auc` ran 0.57–0.83 (sequential encoders learned token order)
and `cross_auc_same_lang` ran 0.46–0.65 (weak real style signal, no
encoder dominating).

## Interpretation

Four architecturally distinct from-scratch encoders plateau at
chance on synthetic mutations at this compute budget — the bottleneck is
representation, not predictor or training data size. With two mutations
contributing zero by construction, the four-way mean was depressed by
design; the result still locked in the verdict on from-scratch encoders
and forced the pivot to density heads.
