# Technique 7 — token_embed (learned nn.Embedding + masked mean pool)

**Branch**: `research/token-embeddings`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

A learned `nn.Embedding` table trained jointly with the JEPA objective should capture
token co-occurrence patterns as dense vectors, allowing the model to generalise across
surface forms in a way sparse TF-IDF cannot. Mean-pooling over token positions preserves
set membership while abstracting away exact position — a stepping stone between bag-of-words
and a full attention mechanism.

Expected failure mode per plan: regression vs char_ngrams on small (too few token occurrences
to train embeddings), parity or win on medium/large.

**Phase 5 discipline**: `char_ngrams`, `context_after`, and `adaptive_epochs` are all off.
Only the encoder changes.

## What changed

**New `engine/argot/jepa/vocab.py`** — `Vocab.build(records, max_size=8000, min_count=2)`:
counts token texts across `context_before` + `hunk_tokens`, keeps the top-8000 by frequency
(dropping singletons). Special IDs: `<pad>=0`, `<unk>=1`, `<bos>=2`.

**New `engine/argot/jepa/seq_encoder.py`** — `MeanPoolEncoder`:
```
nn.Embedding(vocab_size, 128, padding_idx=0)
→ masked mean pool over seq_len=256
→ nn.Linear(128, 192)
```
Sequences truncated to 256 tokens; padding is zero (ignored by the mask).

**`train.py::_train_token_embed`**: builds vocab from training split, pads to seq_len=256,
wraps `MeanPoolEncoder` in `JEPAArgot` in place of `TokenEncoder`. `ModelBundle.vectorizer`
holds the serialised `Vocab` object (research branch only).

**`validate.py::_vectorize_token_embed`**: re-tokenises the space-joined text string,
encodes via vocab, pads to seq_len=256.

Param budget: 8000 × 128 + 128 × 192 ≈ 1.05 M → ~4 MB fp32. ✓

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds, same bucket files.

### Shuffled AUC (primary metric)

| bucket | baseline       | token_embed    | Δ vs baseline | char_ngrams    | Δ vs char |
|:-------|:---------------|:---------------|:--------------|:---------------|:----------|
| small  | 0.500 ± 0.000  | 0.639 ± 0.012  | **+0.139**    | 0.500 ± 0.000  | **+0.139** |
| medium | 0.501 ± 0.000  | 0.701 ± 0.014  | **+0.200**    | 0.655 ± 0.044  | **+0.046** |
| large  | 0.637 ± 0.005  | 0.697 ± 0.019  | **+0.060**    | 0.684 ± 0.005  | +0.013    |

### Cross-repo AUC

| bucket | baseline       | token_embed    | Δ vs baseline | char_ngrams    | Δ vs char |
|:-------|:---------------|:---------------|:--------------|:---------------|:----------|
| small  | 0.446 ± 0.023  | 0.445 ± 0.090  | -0.001        | 0.719 ± 0.022  | -0.274    |
| medium | 0.395 ± 0.004  | 0.665 ± 0.026  | **+0.270**    | 0.650 ± 0.048  | **+0.015** |
| large  | 0.544 ± 0.012  | 0.457 ± 0.076  | -0.087        | 0.648 ± 0.004  | -0.191    |

### Injected AUC

| bucket | baseline       | token_embed    | Δ vs baseline | char_ngrams    | Δ vs char |
|:-------|:---------------|:---------------|:--------------|:---------------|:----------|
| small  | 0.450 ± 0.017  | 0.776 ± 0.070  | **+0.326**    | 0.721 ± 0.020  | **+0.055** |
| medium | 0.398 ± 0.009  | 0.761 ± 0.024  | **+0.363**    | 0.719 ± 0.052  | **+0.042** |
| large  | 0.631 ± 0.008  | 0.687 ± 0.005  | **+0.056**    | 0.736 ± 0.006  | -0.049    |

## Per-bucket interpretation

**Small (3000 records)**: Shuffled AUC jumps to 0.639 (+0.139 vs baseline, +0.139 vs char_ngrams)
— dense embeddings already capture token-order signal even at 3k records, beating char_ngrams on
the primary metric. However, cross-repo AUC is stuck near baseline (0.445) with high variance
(±0.090), while char_ngrams reaches 0.719. Embeddings are memorising in-distribution token
patterns rather than learning style generalisations — a standard small-data failure mode for
dense models. Injected AUC is surprisingly strong (0.776), suggesting the model discriminates
well when foreign content is inserted but can't generalise across whole repos.

**Medium (7000 records)**: Best bucket overall. Shuffled AUC 0.701 — beats both the baseline
(+0.200) and char_ngrams (+0.046). Cross-repo 0.665 slightly edges char_ngrams (0.650). Injected
0.761 also beats char_ngrams. This is the regime where there are enough training records to learn
meaningful dense representations without overfitting to repo-specific vocabulary.

**Large (20000 records)**: Shuffled AUC 0.697 ≈ medium (no further gain, slight edge over
char_ngrams +0.013). But cross-repo collapses to 0.457 ± 0.076 — well below baseline — while
char_ngrams holds at 0.648. The high variance and regression suggest overfitting: with 20k
records the embedding table specialises to the training-split repositories' token distributions,
and the held-out foreign repo is outside that distribution. This is consistent with the
repo-fingerprinting concern flagged in the path-embedding study.

## Verdict

**Token embeddings improve shuffled AUC across all buckets** (+0.139 / +0.200 / +0.060 vs
baseline), with the strongest gains at small and medium scale. They beat char_ngrams on the
primary metric at small (+0.139) and medium (+0.046) and roughly match at large (+0.013).

**However, cross-repo AUC is unstable**: dominant at medium but collapses at large (0.457,
below even the unigram baseline). This makes token embeddings unsuitable as a production
encoder: shuffled-token detection would improve, but cross-repo style discrimination — which
underlies the core argot use-case — would regress at production scale.

**Recommendation**: the shuffled AUC gain is real and reproducible. For a future combined
study, token embeddings paired with `context_after` and `adaptive_epochs` could recapture the
cross-repo regression. Subword tokenisation (BPE) may also help by preventing the embedding
table from fragmenting into repo-specific surface forms.
