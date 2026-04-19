# Technique 6 — word_ngrams (TF-IDF word n-grams, (1,3))

**Branch**: `research/word-ngrams`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)  
**Phase 5 kill criterion**: shuffled AUC must beat char_ngrams by ≥+0.02 on ≥2 buckets to proceed to Branch 3

## Hypothesis

Word n-grams (`ngram_range=(1,3)`) capture local token-order patterns that a unigram TF-IDF
vectorizer erases. The first-order transition "def foo" → "foo (" and the second-order
"def foo (" are small sequential units that might carry style signal. This is the cheapest
possible test of whether token order matters at all for shuffled-token AUC.

Combined-optimizations (char_ngrams, context_after, adaptive epochs) are kept **off** to
isolate the encoder signal. The only change relative to the TF-IDF baseline is the
vectorizer parameters.

## What changed

`train.py::_train_word_ngrams`:
```python
TfidfVectorizer(analyzer="word", ngram_range=(1, 3), max_features=5000, min_df=2)
```

Identical downstream plumbing (tokenisation, DataLoader, training loop, ModelBundle). The
`TokenEncoder` MLP takes whatever `actual_input_dim` the vocabulary yields — no architecture
changes.

Design choices:
- `ngram_range=(1,3)`: unigrams + bigrams + trigrams; captures short local transitions
- `min_df=2`: drops hapax legomena, keeps vocab size tractable at 3k records
- `max_features=5000`: matches `INPUT_DIM` constant; vocabulary fills to this cap at ≥7k records

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds, same bucket files.

### Shuffled AUC (primary metric)

| bucket | baseline       | word_ngrams    | Δ       | char_ngrams    | Δ vs char |
|:-------|:---------------|:---------------|:--------|:---------------|:----------|
| small  | 0.500 ± 0.000  | 0.500 ± 0.000  | +0.000  | 0.500 ± 0.000  | +0.000    |
| medium | 0.501 ± 0.000  | 0.615 ± 0.004  | **+0.114** | 0.655 ± 0.044  | -0.040    |
| large  | 0.637 ± 0.005  | 0.654 ± 0.011  | **+0.017** | 0.684 ± 0.005  | -0.030    |

### Cross-repo AUC

| bucket | baseline       | word_ngrams    | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.446 ± 0.023  | 0.490 ± 0.053  | +0.044  |
| medium | 0.395 ± 0.004  | 0.553 ± 0.032  | **+0.158** |
| large  | 0.544 ± 0.012  | 0.587 ± 0.012  | **+0.043** |

### Injected AUC

| bucket | baseline       | word_ngrams    | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.450 ± 0.017  | 0.496 ± 0.051  | +0.046  |
| medium | 0.398 ± 0.009  | 0.630 ± 0.020  | **+0.232** |
| large  | 0.631 ± 0.008  | 0.669 ± 0.004  | **+0.038** |

## Per-bucket interpretation

**Small (3000 records)**: Shuffled AUC is stuck at 0.500 on both baseline and word_ngrams —
the model has no shuffled-token signal at this scale. Cross and injected AUC are marginally
above baseline (±noise). Same pattern as char_ngrams. The dataset is too small for any
vectorizer to learn meaningful order signal in the JEPA objective.

**Medium (7000 records)**: Word n-grams produces a large shuffled AUC jump over baseline
(+0.114), roughly half the char_ngrams gain (+0.154). The bigram/trigram boundary patterns
evidently do carry some order signal. However, cross-repo and injected AUC are well below
char_ngrams, suggesting the char-level features also capture style fingerprints more
reliably than word-level sequences.

**Large (20000 records)**: Word n-grams improves shuffled AUC by +0.017 over baseline but
falls -0.030 below char_ngrams. Cross and injected AUC are also below char_ngrams levels.
The pattern is consistent: word n-grams capture order signal, but not as effectively as
character-level features at any scale.

## Verdict

**Kill criterion: TRIGGERED — do not proceed to Branch 3.**

Word n-grams improve over the unigram TF-IDF baseline on shuffled AUC at medium and large
scale (+0.114 and +0.017), confirming that local word-order patterns carry signal. However,
word n-grams fail to beat char_ngrams by ≥+0.02 on any bucket (Δ is -0.000, -0.040,
-0.030 at small/medium/large).

Interpretation: character n-grams are a strictly better representation than word n-grams
for this task. They capture the same local sequential patterns (naming conventions, operator
spacing) at finer granularity, and without the vocabulary explosion that limits word trigrams
at small scale.

Branch 3 (transformer encoder) is blocked per the plan. The hypothesis that "sequential
signal exists" is confirmed — both word and char n-grams beat the unigram baseline on
shuffled AUC — but the specific form of that signal (character-level, not word-level) means
a transformer trained on word tokens is unlikely to match char_ngrams without subword
tokenisation, which is deferred to a follow-up study.

Branch 2 (token embeddings) runs regardless of this outcome, as it tests a different axis
(dense learned representations vs. sparse bag-of-n-grams).
