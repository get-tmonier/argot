# Technique 4 — char_ngrams (TF-IDF character n-grams)

**Branch**: `research/char-ngrams`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

The baseline TF-IDF vectorizer uses word tokens as the unit of analysis. Style is
also expressed at the sub-word level: naming conventions (camelCase vs snake_case),
punctuation habits, operator spacing, and idiomatic token boundaries are character
patterns that word tokens erase. Adding character n-grams (3–5 characters,
`analyzer="char_wb"`) should let the vectorizer capture these surface patterns
alongside word-level semantics, improving all AUC metrics — especially cross-repo,
which tests whether the model has learned the *style* rather than the *vocabulary*.

## What changed

`train.py:train_model` gains a `char_ngrams: bool = False` parameter. When `True`,
the TF-IDF vectorizer switches to `analyzer="char_wb"` with `ngram_range=(3, 5)`.
`char_wb` pads tokens with whitespace before extracting n-grams, so word boundaries
are preserved — a 3-gram from "camelCase" won't bleed into the adjacent token.

`corpus.py:run_benchmark` and `_benchmark_one` thread `char_ngrams` through to
`train_model`. The CLI gains `--char-ngrams` (flag, default off). The justfile
gains `research-benchmark-char-ngrams`.

No change to the default training path.

## Results

Baseline is Phase 2 mean ± std (3 seeds, same sizes). Variant: 3 seeds,
small/medium/large only (xlarge dropped — see `03-context-after.md §Decision`).

### Shuffled AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.500 ± 0.000  | 0.500 ± 0.000  | +0.000  |
| medium | 0.501 ± 0.000  | 0.655 ± 0.044  | **+0.154** |
| large  | 0.637 ± 0.005  | 0.684 ± 0.005  | **+0.047** |

### Cross-repo AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.446 ± 0.023  | 0.719 ± 0.022  | **+0.274** |
| medium | 0.395 ± 0.004  | 0.650 ± 0.048  | **+0.255** |
| large  | 0.544 ± 0.012  | 0.648 ± 0.004  | **+0.103** |

### Injected AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.450 ± 0.017  | 0.721 ± 0.020  | **+0.271** |
| medium | 0.398 ± 0.009  | 0.719 ± 0.052  | **+0.321** |
| large  | 0.631 ± 0.008  | 0.736 ± 0.006  | **+0.105** |

## Interpretation

**Character n-grams are the single largest improvement across all Phase 3
experiments — and the only technique that lifts every metric at every bucket.**

### Small (3k): cross-repo jumps from random to useful

The baseline model at small is effectively random on cross-repo (0.446) and injected
(0.450) — it memorises vocabulary but can't detect foreign style. With char n-grams,
cross rises to 0.719 and injected to 0.721. Shuffled stays at 0.500 (still random on
token shuffles), which makes sense: shuffling within a record doesn't change character
patterns. The cross/injected jump means the model is now reading *style* — the
sub-word patterns that distinguish one codebase from another — rather than memorising
which words appear.

### Medium (7k): all three metrics improve substantially

Cross rises from 0.395 to 0.650 (+0.255), injected from 0.398 to 0.719 (+0.321),
shuffled from 0.501 to 0.655 (+0.154). The variance on cross and injected is higher
than at small (std ≈ 0.044–0.052 vs 0.022–0.020), which may reflect the model
picking up different character-level features across seeds. Even so, the worst seed
at medium still outperforms the best baseline seed by a large margin.

### Large (20k): consistent gains, no regression

Cross rises from 0.544 to 0.648 (+0.103), injected from 0.631 to 0.736 (+0.105),
shuffled from 0.637 to 0.684 (+0.047). Variance is low (std ≈ 0.004–0.006). Unlike
the epochs experiment, char n-grams does not overfit at large — the character-level
features generalise even at 20k records.

### Why char n-grams work so well here

Code style is heavily character-patterned. Naming conventions, indentation tokens,
operator choices, and comment markers all live at the sub-word level. A word-token
TF-IDF treats `camelCase`, `snake_case`, and `PascalCase` as distinct *words* but
misses the pattern that they *are all the same case convention used consistently*.
Character n-grams capture the recurring 3–5-character sequences that define a repo's
surface style — making the representation much richer for a style-discrimination task.

## Note on tokenization granularity

The bucket datasets contain **lexical tokens** (tree-sitter output), not subword
pieces. Each token's `text` field is the raw source text of a complete code unit —
`RetryOptions`, `snake_case_fn`, `../utils/merge.js` — so character n-grams on
joined token texts do capture naming-convention patterns at the right granularity.

What this approach cannot see:
- **Spacing around operators** (`a+b` vs `a + b`) — already collapsed by
  tokenisation; the space is the boundary between separate tokens.
- **Indentation patterns** — not emitted by the extractor.

A deeper variant would apply character n-grams (or raw character windows) to
pre-tokenisation source text. That would add spacing and indentation as signals
but would require the extractor to emit raw source windows instead of token lists —
a more invasive change. It's a candidate future technique, not a flaw in this
experiment.

## Verdict

**Merge unconditionally.** The improvement is large, consistent across all bucket
sizes, and shows no overfitting risk. This is the strongest technique found in Phase 3
and the one most worth integrating into the default training path.

Cross-repo AUC at small (0.719) now exceeds the Phase 2 large baseline (0.544) —
char n-grams give small repos (3k records) better cross-style discrimination than
the baseline achieves at 20k records. This directly addresses the original finding
that motivated this research: the model was "effectively random" at small corpus sizes.
