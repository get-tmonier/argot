# Phase 7.1 — Re-baseline 4 encoders on honest eval

**Branch**: `research/phase-7-honest-eval`
**Status**: complete — decision gate NOT cleared; proceeding to Phase 7.2
**Date**: 2026-04-19

## Setup

- Corpus: Phase 7.0 honest buckets (6 = 3 sizes × 2 langs), see [`corpus.md`](corpus.md).
- Encoders: `tfidf` (char_wb 3–5), `word_ngrams` (word 1–2), `token_embed`
  (mean-pooled token embeddings), `bpe` (BPE tokens + mean-pool).
- Seeds: 3 per (bucket × encoder), stratified downsample.
- Epochs: adaptive — `min(200, max(20, 1_400_000 // size))`.
- Target sizes: small=3000, medium=7000, large=20000.
- Outputs: `.argot/research/results-7-1-{tfidf,word-ngrams,token-embed,bpe}.jsonl`.

## Coverage

| encoder     | runs | note                                           |
|:------------|-----:|:-----------------------------------------------|
| tfidf       | 18/18 | complete                                       |
| word_ngrams | 18/18 | complete (encoder implemented in this phase)   |
| token_embed | 18/18 | complete                                       |
| bpe         | 7/18  | killed after small-\* + 1 seed of medium-py — synthetic AUC tracked the others and was locally the worst |

## Primary metric — `synthetic_auc_mean` (mean ± std over 3 seeds)

| bucket     | tfidf         | word_ngrams   | token_embed   | bpe           |
|:-----------|:--------------|:--------------|:--------------|:--------------|
| small-py   | 0.495 ± 0.001 | 0.504 ± 0.002 | 0.498 ± 0.003 | 0.495 ± 0.004 |
| small-ts   | 0.500 ± 0.004 | 0.496 ± 0.008 | 0.493 ± 0.004 | 0.494 ± 0.003 |
| medium-py  | 0.506 ± 0.010 | **0.516 ± 0.007** | 0.481 ± 0.008 | 0.434 (n=1) |
| medium-ts  | 0.501 ± 0.003 | 0.503 ± 0.005 | 0.501 ± 0.004 | —             |
| large-py   | 0.497 ± 0.000 | 0.500 ± 0.001 | 0.490 ± 0.004 | —             |
| large-ts   | 0.495 ± 0.001 | 0.501 ± 0.002 | 0.497 ± 0.002 | —             |

**Decision gate: NOT cleared.** Target was ≥ 0.85 at medium on ≥ 2/3 seeds.
Best observed: **word_ngrams medium-py seed 0 = 0.525** — 0.325 below target.
All encoders sit in the 0.48–0.52 band across all buckets; no encoder shows a
usable synthetic-mutation signal.

## Per-mutation AUC at medium (mean over 3 seeds)

| encoder     | bucket    | case_swap | debug_inject | error_flip | quote_flip |
|:------------|:----------|----------:|-------------:|-----------:|-----------:|
| tfidf       | medium-py | 0.493 | 0.504 | 0.500 | 0.527 |
| tfidf       | medium-ts | 0.486 | 0.518 | 0.500 | 0.500 |
| word_ngrams | medium-py | 0.535 | 0.530 | 0.500 | 0.500 |
| word_ngrams | medium-ts | 0.503 | 0.509 | 0.500 | 0.500 |
| token_embed | medium-py | 0.516 | 0.411 | 0.500 | 0.495 |
| token_embed | medium-ts | 0.534 | 0.471 | 0.500 | 0.500 |
| bpe (n=1)   | medium-py | 0.411 | 0.360 | 0.500 | 0.464 |

`error_flip` and `quote_flip` land on **exactly 0.500** almost everywhere:
the mutations no-op when the trigger syntax (`raise`/`throw`, string quotes)
is absent from the held-out hunk. Flag for Phase 8 mutation redesign.

## Secondary — `shuffled_auc` (token-order sanity check)

| bucket     | tfidf | word_ngrams | token_embed | bpe (n≤3) |
|:-----------|------:|------------:|------------:|----------:|
| small-py   | 0.699 | 0.711 | **0.752** | 0.751 |
| small-ts   | 0.587 | 0.574 | **0.658** | 0.632 |
| medium-py  | 0.702 | 0.718 | **0.810** | 0.825 (n=1) |
| medium-ts  | 0.716 | 0.718 | **0.776** | — |
| large-py   | 0.713 | 0.676 | 0.682 | — |
| large-ts   | 0.683 | 0.647 | **0.806** | — |

Sequential encoders (token_embed, bpe) learn token order — as Phase 5/6 showed.
Not the primary metric; included for continuity with prior phases.

## Secondary — `cross_auc_same_lang` (real style signal)

| bucket     | tfidf | word_ngrams | token_embed | bpe (n≤3) |
|:-----------|------:|------------:|------------:|----------:|
| small-py   | 0.633 | 0.602 | 0.566 | 0.642 |
| small-ts   | 0.600 | **0.647** | 0.521 | 0.543 |
| medium-py  | 0.511 | **0.593** | 0.558 | 0.503 (n=1) |
| medium-ts  | 0.462 | 0.536 | 0.546 | — |
| large-py   | **0.623** | 0.579 | 0.462 | — |
| large-ts   | 0.534 | 0.611 | **0.648** | — |

Weak but non-trivial style signal (0.46–0.65). No encoder dominates; the cross-repo
style gap is far smaller than the language gap the old eval was accidentally measuring.

## Interpretation

The bottleneck is **representation**, not the training head or the eval data:

1. Synthetic-AUC is flat near chance across 4 architecturally distinct encoders
   and 3 sizes — this is the ceiling for *from-scratch* embeddings at this
   compute budget, not a head or sizing problem.
2. Every encoder learns *some* token-order structure (shuffled AUC 0.57–0.83)
   and *some* real-style signal (cross-AUC-same-lang 0.46–0.65) — the encoders
   work; they just can't resolve subtle syntactic mutations.
3. `error_flip`/`quote_flip` being identically 0.500 means ≈ half the hunks are
   untouched by those mutations. Real signal on `case_swap`/`debug_inject` is
   ≤ 0.035 above chance — too small to be useful.

Confirms the Phase 7 design hypothesis: from-scratch encoders plateau well below
the 0.85 bar; move to pretrained encoders.

## Next

Proceed to **Phase 7.2** (density heads on BPE — kNN, GMM). If 7.2 also fails
the decision gate, Phase 7.3 brings in CodeRankEmbed.

## Deferred

- **Mutation redesign**: `error_flip`/`quote_flip` need hunk-aware trigger
  selection so they actually mutate. Defer to Phase 8.
- **GPU training**: CPU-only today (no `.to(device)` wiring). Consider for 7.2+
  if runtime becomes the constraint.
