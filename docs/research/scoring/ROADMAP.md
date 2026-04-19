# Scoring Research — Roadmap

> Read at the start of every session. Update at the end.

**Current phase**: Phase 7.2 — density heads on BPE (kNN, GMM)
**Active branch**: `research/phase-7-honest-eval`
**Last touched**: 2026-04-19
**Design docs**: [`DESIGN-phases-1-6.md`](DESIGN-phases-1-6.md) · [`DESIGN-phase-7.md`](DESIGN-phase-7.md)

---

## Active: Phase 7 — honest eval + pretrained encoder pivot

Spec: [`DESIGN-phase-7.md`](DESIGN-phase-7.md)
Primary metric: `synthetic_auc_mean` (4 mutations × same-language sub-corpora)
Target: ≥ 0.85 at medium bucket on ≥ 2 of 3 seeds
Corpus: [`phase-7/corpus.md`](phase-7/corpus.md)

- [x] 7.0 — eval rebuild: 12-repo honest corpus, `mutations.py`, per-mutation AUCs in `_benchmark_one`. Sanity run: `cross_auc_same_lang=0.628`, `synthetic_auc_mean=0.507` (3/4 mutations invisible to TF-IDF — expected; Phase 7.1 is the real test).
- [x] 7.1 — re-baseline 4 encoders on new eval ([`phase-7/16-rebaseline.md`](phase-7/16-rebaseline.md)). Decision gate NOT cleared: best `synthetic_auc_mean` = 0.525 (word_ngrams medium-py seed 0) vs 0.85 target. All 4 encoders sit in 0.48–0.52 across all buckets. Confirms from-scratch ceiling; move to pretrained in 7.3 after exhausting 7.2.
- [ ] 7.2 — density heads on BPE: kNN, GMM (`phase-7/17-density-heads.md`)
- [ ] 7.3 — frozen CodeRankEmbed + current head (`phase-7/18-pretrained-jepa.md`)
- [ ] 7.4 — frozen CodeRankEmbed + best density head (`phase-7/19-pretrained-density.md`)
- [ ] 7.5 — structural context, conditional on 7.2–7.4 failing (`phase-7/20-structural-context.md`)

**Decision gate at every sub-phase.** Stop, summarise, update this roadmap, wait for approval before continuing.

---

## Phases 1–6 — TF-IDF era (complete)

Full results in [`phases-1-6/`](phases-1-6/) · design: [`DESIGN-phases-1-6.md`](DESIGN-phases-1-6.md)

**Phase 2 — sizing** (`phases-1-6/sizing-study.md`): model is random below 20k records with default TF-IDF. char_ngrams (Phase 3) later pushed minimum viable size down to 7k.

**Phase 3 — technique experiments** (`phases-1-6/synthesis.md`): 6 techniques tested. char_ngrams is the standout — lifts all metrics at every scale. imports_scope and path_embed are repo-identity signals (rejected). epochs and context_after give incremental gains.

**Phase 4 — combined** (`phases-1-6/09-combined.md`): char_ngrams + adaptive epochs + context_after as defaults. Shuffled AUC 0.637 / 0.616 / 0.713 at small/medium/large. Gains not fully additive. TF-IDF identified as the fundamental bottleneck.

**Phase 5 — sequential encoders**: word_ngrams kills transformer branch. token_embed beats char_ngrams on shuffled AUC at small/medium (+0.139/+0.046) but cross-repo collapses at large.

**Phase 6 — BPE** (`phases-1-6/13-bpe-tokenisation.md`): BPE nearly eliminates cross-repo collapse at small (0.445→0.710), improves medium (0.684). Large still partially collapsed (0.514). Phase 4 regularisation harms dense encoders — adaptive_epochs causes catastrophic overfitting at medium for token_embed.

**Ceiling reached at ~0.70 shuffled AUC.** Two root causes: (a) eval confounded by TS+Py language mixing, (b) no pretrained encoder tried. Phase 7 addresses both.

---

## Decisions

- 2026-04-18 — 2 repos per bucket; stop rule AUC lift < 0.01.
- 2026-04-18 — calibration work deferred to separate branch.
- 2026-04-19 — adaptive epochs formula: `min(200, max(20, 1_400_000 // size))`. Cap at 200.
- 2026-04-19 — cross/injected AUC invalid for Phases 1–6 (measure language detection, not style). Shuffled AUC is the only honest metric for mixed TS+Py pairs.
- 2026-04-19 — Phase 7 pivots to same-language pairs + synthetic mutations + pretrained encoders.
- 2026-04-19 — chalk and axios dropped from small-ts (JS-heavy); replaced with colinhacks/zod.
- 2026-04-19 — rollup dropped from medium-ts (72% JS → bucket was 59% JS); replaced with typescript-eslint (98.5% TS → bucket now 95.4% TS).
- 2026-04-19 — `cross_auc_same_lang` guard relaxed from `len(langs) == 1` to `dominant_lang_share >= 0.95`; every bucket has tree-sitter stragglers from other languages so the strict guard was always null.
- 2026-04-19 — `word_ngrams` encoder implemented (word analyzer, ngram (1,2)) to complete the 4-way re-baseline; was a `NotImplementedError` stub.

## Open questions

- `docs/scoring.md` user-facing numbers: blocked on Phase 7 honest numbers.

---

## Session log

- **2026-04-18**: design approved; branch created; DESIGN.md + ROADMAP.md
  committed. Next: write Phase 1 implementation plan via writing-plans skill.
- **2026-04-18**: Phase 1 complete. `argot-corpus concat` and
  `argot-corpus benchmark` land, `extract --repo-name` stamps `_repo`.
  `just research-concat` and `just research-benchmark` wired. Next: Phase 2
  corpus kickoff — pin repo URLs + SHAs in `01-corpus.md`.
- **2026-04-18**: Phase 2 in progress. Extracted 7 repos. Medium + large
  benchmarks complete and valid. Corpus audit triggered reclassification:
  buckets now assigned by record count; zod and typescript-compiler dropped;
  vscode added for xlarge TS; small TS slot TBD. benchmark fix: streaming
  JSONL load in `corpus.py` (was crashing on 9.5 GB xlarge file). Re-running
  small + xlarge benchmarks after new repo acquisition.
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
- **2026-04-19**: Phase 7.0 complete. Honest corpus pinned (12 repos — httpx, requests, ky, zod, fastapi, flask, vite, rollup, pydantic, django, effect, angular — SHAs recorded in `.argot/research/datasets-v2/SHAS.md`). `mutations.py` with 4 mutations (case_swap, debug_inject, error_flip, quote_flip) landed with unit-test coverage. `_benchmark_one` now emits `synthetic_auc_mean` + per-mutation AUCs + `cross_auc_same_lang`. Sanity run on small-py confirmed end-to-end (cross_auc_same_lang=0.628, synthetic_auc_mean=0.507 — 3/4 mutations invisible to TF-IDF as expected; Phase 7.1 tests char_ngrams/BPE). Note: chalk and axios were dropped from small-ts (JS-heavy); replaced with zod (14,575 pure TS records). Next: Phase 7.1 — re-baseline 4 existing encoders on the new eval.
- **2026-04-19**: Phase 7.1 complete. 4 encoders (tfidf, word_ngrams, token_embed, bpe) × 6 buckets × 3 seeds re-baselined on honest eval. **Decision gate NOT cleared**: best `synthetic_auc_mean` = 0.525 (word_ngrams medium-py seed 0) vs 0.85 target; all encoders sit in 0.48–0.52 band across all sizes. Token-order signal survives (shuffled 0.57–0.83, token_embed leads), and weak real-style signal exists (`cross_auc_same_lang` 0.46–0.65) — encoders work, they just can't resolve the synthetic mutations. Two mid-run infra fixes: rollup → typescript-eslint swap (medium-ts was 59% JavaScript), and `cross_auc_same_lang` guard relaxed to dominant-lang ≥ 95%. `word_ngrams` stub implemented to enable the 4-way comparison. `error_flip`/`quote_flip` land at exactly 0.500 — mutations no-op when trigger syntax absent from hunk (flagged for Phase 8 mutation redesign). BPE killed after 7/18 runs (tracking the pack, not worth continuing). Full write-up: `phase-7/16-rebaseline.md`. Next: Phase 7.2 — density heads (kNN, GMM) on BPE embeddings.
