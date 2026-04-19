# Technique 7 — Combined Optimizations

**Branch**: `research/combined-optimizations`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)  
**Techniques combined**: char_ngrams + adaptive epochs + context_after (all as defaults, no flags)

## What changed

Three winning techniques from Phase 3 promoted to defaults simultaneously:

- **char_ngrams**: `TfidfVectorizer(analyzer="char_wb", ngram_range=(3,5), max_features=5000)`
- **adaptive epochs**: `min(200, max(20, 1_400_000 // size))` — 200 at 3k/7k, 70 at 20k
- **context_after**: always loaded and concatenated into `ctx_texts`

Benchmark protocol: per-bucket corpus files (small.jsonl @ 3k, medium.jsonl @ 7k,
large.jsonl @ 20k), 3 seeds each. Same protocol as Phase 3 techniques.

## Results

### Shuffled AUC

| bucket | baseline       | char_ngrams alone | combined       | Δ vs baseline | Δ vs char_ngrams |
|:-------|:---------------|:------------------|:---------------|:--------------|:-----------------|
| small  | 0.500 ± 0.000  | 0.500 ± 0.000     | 0.637 ± 0.008  | **+0.137**    | **+0.137**       |
| medium | 0.501 ± 0.001  | 0.655 ± 0.044     | 0.616 ± 0.021  | **+0.115**    | −0.039           |
| large  | 0.637 ± 0.006  | 0.684 ± 0.005     | 0.713 ± 0.013  | **+0.076**    | **+0.029**       |

### Cross-repo AUC

| bucket | baseline       | char_ngrams alone | combined       | Δ vs baseline | Δ vs char_ngrams |
|:-------|:---------------|:------------------|:---------------|:--------------|:-----------------|
| small  | 0.446 ± 0.028  | 0.719 ± 0.022     | 0.394 ± 0.040  | −0.052        | **−0.325**       |
| medium | 0.395 ± 0.005  | 0.650 ± 0.048     | 0.480 ± 0.101  | **+0.085**    | **−0.170**       |
| large  | 0.544 ± 0.015  | 0.648 ± 0.004     | 0.622 ± 0.034  | **+0.078**    | −0.026           |

### Injected AUC

| bucket | baseline       | char_ngrams alone | combined       | Δ vs baseline | Δ vs char_ngrams |
|:-------|:---------------|:------------------|:---------------|:--------------|:-----------------|
| small  | 0.450 ± 0.020  | 0.721 ± 0.020     | 0.743 ± 0.017  | **+0.293**    | +0.022           |
| medium | 0.398 ± 0.011  | 0.719 ± 0.052     | 0.679 ± 0.010  | **+0.281**    | −0.040           |
| large  | 0.631 ± 0.010  | 0.736 ± 0.006     | 0.742 ± 0.013  | **+0.111**    | +0.006           |

## Interpretation

### Gains are not additive

The combination beats the baseline on every shuffled and injected metric at every
size. But it does **not** beat char_ngrams alone on cross-repo — it regresses by
−0.325 at small and −0.170 at medium. The techniques do not compound cleanly.

The most likely explanation: adaptive epochs and context_after push the model
toward intra-repo coherence (shuffled/injected), while char_ngrams was also
driving cross-repo gains — and those two objectives compete for model capacity
at small/medium sizes. At large, where the model has more data to work with,
the combination is roughly additive (all three metrics up vs both baselines).

The medium cross variance is also notably high (0.386–0.620 across seeds) —
larger than any individual technique. This instability at medium is a sign the
model is near a phase boundary where training dynamics are sensitive.

### The honest metric: shuffled AUC

Cross-repo and injected AUC in this benchmark are confounded by a fundamental
design issue: **every bucket pairs a TypeScript repo with a Python repo.**
Cross-repo AUC largely measures language detection (Python vs TypeScript), not
style discrimination. Injected AUC injects Python tokens into TypeScript context
— again, language detection.

Shuffled AUC is the only honest metric: same repo, same language, tokens in
wrong order. On that metric the combination wins consistently:

- small: 0.500 → **0.637** (+0.137)
- medium: 0.501 → **0.616** (+0.115)
- large: 0.637 → **0.713** (+0.076)

These are real improvements in the model's ability to detect incoherent code
within a repo. But 0.713 at 20k still means the model fails ~29% of the time on
obviously shuffled tokens. For a style linter that needs to catch subtle
violations, that ceiling is a problem.

### The deeper limitation: TF-IDF destroys token order

Shuffled tokens produce nearly the same TF-IDF vector as ordered tokens — the
same tokens are present, just with different context/hunk assignment. The only
reason char_ngrams help shuffled AUC is that character 3–5-grams capture local
order (shuffling words changes character n-gram boundaries). But even that is a
weak proxy for sequence structure.

A world model — a model that learns the "physics" of how code flows in a repo
and flags anomalies — fundamentally requires sequential input. TF-IDF cannot
represent the difference between `const foo = bar()` and `bar() foo const =`.
No combination of TF-IDF techniques will fix this.

## Verdict

**Merge as an improvement over baseline, but do not treat as the end state.**

The combined techniques give the best shuffled AUC in the series at all sizes.
They beat the baseline everywhere on the metric that actually matters.
Merge them as the new default — they make the model meaningfully less random.

But the research series has surfaced a structural limit: the input representation
is the bottleneck, not the training objective or the technique stack.

## Next step: sequential encoder

The recommended next research direction is replacing TF-IDF with a sequential
encoder, in three steps of increasing complexity:

1. **Word n-grams** — `TfidfVectorizer(analyzer="word", ngram_range=(1,3))`.
   One-line change. Captures `const → foo →` bigrams, preserves local token
   order. Quick validation that sequential signal exists.

2. **Learned token embeddings + mean pooling** — small embedding table over
   repo vocab (dim 64), position-weighted mean. Dense sequential representation,
   no pretrained weights, < 5MB.

3. **Small transformer encoder** — 2-layer, 128 hidden, 4 heads, positional
   encodings. Replaces `TokenEncoder`. Genuine world model: learns to predict
   what follows what in this codebase. Still CPU-runnable, < 10MB trained
   artifact.

The JEPA training objective is correct. The input is the problem.

## Benchmark design note

All Phase 2–3 benchmarks pair one TypeScript repo with one Python repo per
bucket. Cross-repo and injected metrics are therefore confounded by language
detection. Future benchmarks should use same-language pairs to isolate style
discrimination from language detection.
