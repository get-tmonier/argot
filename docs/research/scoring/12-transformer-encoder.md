# Phase 5 Branch 3 — transformer encoder

**Status**: BLOCKED — kill criterion triggered  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Why this branch was not run

Phase 5 used a kill criterion to gate Branch 3 (transformer encoder): Branch 1 shuffled AUC
must beat char_ngrams by ≥+0.02 on ≥2 buckets. The rationale was that a transformer on raw
token atoms is only worth the engineering cost if word-level order signal is demonstrably
useful.

Branch 1 (`word_ngrams`, [`10-word-ngrams.md`](10-word-ngrams.md)) failed the criterion:
shuffled AUC did not beat char_ngrams at any bucket size. This indicated that word-level token
order does not carry detectable style signal with a TF-IDF-style sparse encoding.

**Conclusion at the time**: a transformer on raw token atoms is unlikely to leapfrog char_ngrams
without subword tokenisation. The kill criterion was the correct call — investing transformer
capacity in repo-specific token atoms would reproduce the same OOV collapse observed in
`token_embed` (Branch 2, [`11-token-embeddings.md`](11-token-embeddings.md)).

## Future path

A transformer encoder becomes viable again once BPE tokenisation is the default (Phase 6
Workstream A, [`13-bpe-tokenisation.md`](13-bpe-tokenisation.md)). With subword units shared
across repos, a transformer on BPE tokens can learn positional patterns without memorising
repo-specific surface forms. This is the natural Phase 7 experiment if BPE proves stable.
