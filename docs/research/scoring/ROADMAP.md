# Scoring Research — Roadmap

> Read this at the start of every session. Update it at the end.

**Current phase**: Phase 5 complete — sequential encoders (2/3 branches)
**Active branch**: `research/combined-optimizations`
**Last touched**: 2026-04-19
**Spec**: [`DESIGN.md`](DESIGN.md)

---

## Phase 1 — benchmark infrastructure

- [x] 1a. Add `--repo-name` to `argot-engine extract`; stamp `_repo` on records
- [x] 1b. Add `argot-engine corpus concat` utility
- [x] 1c. Add `argot-engine corpus benchmark` batch runner
- [x] Unit tests for downsample/stratify + concat
- [x] `just research benchmark` target wired
- [x] Verify suite green


## Phase 2 — sizing study

- [x] 01-corpus.md: pin repo URLs + SHAs
- [x] Clone + extract: argot, ruff, click, vite, effect, pydantic done
- [x] Concat + benchmark: small (3k), medium (7k), large (20k) complete
- [x] 02-sizing-study.md: AUC-vs-size table + interpretation

**Finding**: model is random below 20k records with default TF-IDF.

## Phase 3 — technique experiments

- [x] 03-context-after.md — small consistent gain at large (+0.038 cross)
- [x] 04-embed-dim.md — marginal, skip
- [x] 05-epochs.md — sweet spot at medium (7k); overfits at large with fixed 200
- [x] 06-char-ngrams.md — **best single technique**; lifts all metrics at all sizes
- [x] 07-imports-scope.md — repo identity signal, do not merge
- [x] 08-path-embed.md — repo identity signal, do not merge

## Phase 4 — synthesis

- [x] 99-synthesis.md: overall findings, ranked technique list
- [x] 09-combined.md: combined optimizations — char_ngrams + adaptive epochs +
      context_after as defaults; gains not fully additive; shuffled AUC is the
      only unconfounded metric
- [ ] Update `docs/scoring.md` with user-facing numbers

---

## Decisions made

- 2026-04-18 — corpus composition: 2 repos per bucket (TS + Py), 5 buckets.
- 2026-04-18 — all 6 techniques attempted; stop rule = AUC lift < 0.01.
- 2026-04-18 — calibration work deferred to separate branch.
- 2026-04-18 — corpus reclassification: buckets by record count; zod +
  typescript-compiler dropped; ruff → small; click + vite → medium.
- 2026-04-19 — adaptive epochs formula: `min(200, max(20, 1_400_000 // size))`.
  Cap at 200 — uncapped formula gives 466 at 3k which overfits per 05-epochs.md.
- 2026-04-19 — benchmark design flaw identified: all bucket pairs are TS + Py,
  so cross-repo and injected AUC measure language detection, not style.
  Shuffled AUC is the only valid metric. Future benchmarks must use
  same-language pairs.
- 2026-04-19 — TF-IDF identified as the fundamental bottleneck. Bag-of-words
  destroys token order; a world model requires sequential input. The JEPA
  objective is correct; the input representation is the problem.

## Open questions (parked)

- `docs/scoring.md` update: minimum viable corpus, quality characteristics.
  Blocked on same-language benchmark to give honest numbers.

## Phase 5 — sequential encoders

Branch off `research/scoring-benchmark`. Combined-optimizations kept **off** to isolate
encoder signal. Primary metric: shuffled AUC. Kill criterion for Branch 3: Branch 1
shuffled AUC must beat char_ngrams by ≥+0.02 on ≥2 buckets.

- [x] Prep PR: encoder dispatch infrastructure (EncoderKind, _train_tfidf factored, stubs)
- [x] 10-word-ngrams.md: `TfidfVectorizer(analyzer="word", ngram_range=(1,3))` — kill
      criterion triggered; Branch 3 blocked
- [x] 11-token-embeddings.md: `nn.Embedding` + masked mean pool — shuffled AUC beats
      char_ngrams at small (+0.139) and medium (+0.046); cross-repo collapses at large
- [ ] 12-transformer-encoder.md — **BLOCKED** (Branch 1 kill criterion triggered)

**Phase 5 findings:**

1. Token order signal exists — both encoders beat TF-IDF baseline on shuffled AUC.
2. Dense > sparse for order detection — token embeddings beat char_ngrams on shuffled AUC
   at small and medium despite mean pooling being order-invariant. The gain is from richer
   representation, not positional encoding.
3. Cross-repo vs shuffled trade-off — dense embeddings overfit to in-repo vocabulary at
   large scale (cross-repo 0.457 vs baseline 0.544 at 20k). char_ngrams remains the best
   cross-repo encoder at every scale.
4. Kill criterion insight — word-level order doesn't beat char-level features, suggesting
   style signal lives sub-word. A transformer on raw token atoms is unlikely to leapfrog
   char_ngrams without subword tokenisation (BPE).

**Recommendation for next phase:**
- Token embeddings + context_after + adaptive_epochs: address the cross-repo regression
  via the same combined-study approach as Phase 4.
- Subword tokenisation: BPE vocab prevents the embedding table fragmenting into
  repo-specific surface forms, expected to fix the large-scale cross-repo collapse.
- Same-language corpus pairs: current benchmark conflates language detection with style
  (all bucket pairs are TS + Py). Valid shuffled AUC requires same-language pairs.

## Session log

- **2026-04-18**: design approved; Phase 1 complete; Phase 2 sizing study
  complete (medium + large). Corpus reclassification triggered.
- **2026-04-19**: Phase 3 all 6 techniques complete. char_ngrams is the
  standout — merge unconditionally. imports_scope and path_embed are repo
  identity signals — do not merge. Phase 4 synthesis complete (99-synthesis.md).
- **2026-04-19**: Combined optimizations branch created. All three winning
  techniques promoted to defaults (no flags). Adaptive epochs capped at 200
  after discovering uncapped formula gives 466 epochs at 3k (overfitting risk).
  Combined benchmark complete: shuffled AUC 0.637/0.616/0.713 at small/medium/
  large — consistent improvement over baseline but gains not additive vs
  char_ngrams alone on cross-repo. Key insight: TF-IDF is the fundamental
  bottleneck; sequential encoder is the next research direction.
- **2026-04-19**: Phase 5 complete (2/3 branches). Word n-grams kill criterion
  triggered — branch 3 blocked. Token embeddings beat char_ngrams on shuffled
  AUC at small (+0.139) and medium (+0.046) but cross-repo collapses at large
  scale. Conclusion: dense representation wins on order detection; char_ngrams
  still wins on style generalisation. Next: BPE tokenisation + combined study.
