# Scoring Research — Roadmap

> Read this at the start of every session. Update it at the end.

**Current phase**: Phase 7 approved — honest eval rebuild + pretrained encoder pivot
**Active branch**: `research/combined-optimizations` (Phase 7 work will branch off)
**Last touched**: 2026-04-19
**Specs**: [`DESIGN.md`](DESIGN.md) (Phases 1–6), [`DESIGN-phase-7.md`](DESIGN-phase-7.md) (Phase 7)
**Plans**: [`PLAN-phase-7-0.md`](PLAN-phase-7-0.md) (Phase 7.0 — honest eval rebuild)

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
- Subword tokenisation: BPE vocab prevents the embedding table fragmenting into
  repo-specific surface forms, expected to fix the large-scale cross-repo collapse.
- Same-language corpus pairs: current benchmark conflates language detection with style
  (all bucket pairs are TS + Py). Valid shuffled AUC requires same-language pairs.

## Phase 6 — BPE tokenisation + combined encoder study

Branch off `research/token-embeddings`. Phase 5 discipline for Workstream A (keep
combined-optimizations off to isolate encoder). Workstream B explicitly enables them.

- [x] 13-bpe-tokenisation.md: `BpeVocab` (HuggingFace ByteLevelBPE) replacing raw token vocab
- [x] 14-token-embed-combined.md: token_embed + context_after + adaptive_epochs (negative result)

**Phase 6 findings:**

1. BPE dramatically fixes cross-repo at small and medium — OOV collapse at small (token_embed
   0.445 → BPE 0.710) nearly eliminated. BPE dominates all prior encoders on cross-repo at
   medium (0.684 vs char_ngrams 0.650, token_embed 0.665).
2. BPE cross-repo collapse at large is reduced but not fixed — 0.514 ± 0.054 vs token_embed
   0.457. High variance; one seed regressed to 0.443. Shuffled AUC preserved (0.704 ≥ 0.697).
3. context_after + adaptive_epochs are harmful for dense encoders — the Phase 4 regularisation
   techniques that help TF-IDF cause catastrophic overfitting in token_embed at medium (shuffled
   0.582 vs 0.701 baseline). Cross-repo at large collapses to 0.317 — worst result in research.
   Dense encoders require architectural regularisation, not epoch scaling.

**Recommendation for next phase:**
- BPE + context_after + adaptive_epochs synthesis: despite Workstream B failing in isolation,
  BPE changes the overfitting dynamics. With BPE's reduced OOV rate, adaptive_epochs may no
  longer over-specialise to repo-specific tokens. Run a combined study analogous to 09-combined.
- Same-language corpus pairs: required for honest cross-repo numbers (current pairs are TS + Py,
  measuring language detection not style).

## Phase 7 — honest eval + pretrained encoder pivot

Spec: [`DESIGN-phase-7.md`](DESIGN-phase-7.md). Primary metric: `synthetic_auc_mean`.
Target: ≥ 0.85 at medium bucket on ≥ 2 of 3 seeds. Phase stops at the first
experiment that clears the bar.

- [ ] 7.0 eval rebuild — same-language pairs + synthetic mutations
      (`15-honest-corpus.md`). Post-extract: re-verify that candidate pairs
      (httpx+requests, fastapi+flask, pydantic+django, ky+chalk, vite+rollup,
      effect+angular) actually cluster at expected bucket sizes; swap/rename
      buckets if they don't.
- [ ] 7.1 re-baseline existing encoders on new eval (`16-rebaseline.md`)
- [ ] 7.2 density heads on BPE — kNN, GMM (`17-density-heads.md`)
- [ ] 7.3 frozen CodeRankEmbed + current head (`18-pretrained-jepa.md`)
- [ ] 7.4 frozen CodeRankEmbed + best density head (`19-pretrained-density.md`)
- [ ] 7.5 structural context — conditional on 7.2–7.4 failing
      (`20-structural-context.md`)

**Decision gates** at every experiment (7.1 – 7.5). STOP, summarise, update
this roadmap, wait for approval before continuing.

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
- **2026-04-19**: Phase 6 complete. BPE (Workstream A) nearly eliminates
  cross-repo collapse at small (0.445→0.710) and improves medium, but large
  remains partially collapsed (0.514 vs target 0.600). Shuffled AUC preserved.
  token_embed + context_after + adaptive_epochs (Workstream B) fails: medium
  shuffled regression -0.119, cross-repo at large collapses to 0.317. Phase 4
  techniques are harmful for dense encoders.
- **2026-04-19**: Phase 7 planned. Six phases of from-scratch experiments have
  plateaued around shuffled AUC 0.70. Target for v1 is 0.85. Two unresolved
  issues: (a) eval is language-confounded (all bucket pairs TS+Py), (b) no
  pretrained encoder has been tried. Phase 7 rebuilds the eval with
  same-language pairs + synthetic mutations, then runs a focused architecture
  search (density heads on BPE → CodeRankEmbed → cross-product → structural
  context) with decision gates at 0.85 synthetic AUC. Research mode, no ship
  pressure.
