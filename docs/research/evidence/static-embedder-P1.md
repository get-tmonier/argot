# P1 — can a static embedder carry argot's semantic sense?

**Date:** 2026-07-29
**Plan:** `.scratch/plan-static-embedder.md`
**Question:** the heavy GGUF embedder runs at 23 fn/s on CPU, which is the root
cause of the 25-minute CI seed (`semantic-seed-cost.md`) and of every
workaround around it. `model2vec-rs` promises pure-Rust static embeddings two
orders of magnitude faster. Does the geometry survive?

**Verdict so far: speed yes, overwhelmingly. Geometry no — not as a drop-in.**

Corpus: MSEide/MSEgui, 26,982 Pascal functions extracted from the exact corpus
the fit used (`.argot/repo-corpus.txt`), joined to the fitted heavy index on
`text_hash` — 25,969 functions matched (99.5% of the heavy index). 1,500
queries, k=10, `redundant` gate at 0.85 (1,116 queries sit above it in the heavy
space). Host: 11-core Apple silicon.

## X1.1 / X1.2 — throughput and determinism

| model | dim | encode | throughput | load | rerun identical |
| --- | ---: | ---: | ---: | ---: | --- |
| heavy (jina-v2-base-code Q4, llama.cpp, CPU) | 768 | — | **23 fn/s** | — | yes |
| `potion-retrieval-32M` | 512 | 1.32 s | **20,405 fn/s** | 0.57 s | **yes** |
| `jina-code-05b-static-256` | 256 | 1.04 s | **25,892 fn/s** | 2.09 s | **yes** |

**887× to 1,126× the heavy model.** MSEgui's semantic index goes from ~19 min of
embedding to **~1 second**. Both static models are bit-identical on rerun.

This part is not in doubt and no quality result can take it away.

## X1.3 / X1.4 — geometry

| metric | `potion-retrieval-32M` | `jina-code-05b-static-256` |
| --- | ---: | ---: |
| overlap@10 vs heavy | 0.3925 | 0.2549 |
| top-1 agreement | 0.4187 | 0.3320 |
| **threshold flips** | 254 (16.9%) | 366 (24.4%) |
| — false negatives (heavy fires, static does not) | 254 | 366 |
| — false positives (static fires, heavy does not) | **0** | **0** |
| recall@20 (heavy top-1 in static top-20) | 0.8113 | 0.6107 |
| recall@50 | 0.8633 | 0.6580 |
| **recall@100** | **0.8973** | 0.6920 |
| static top-1 similarity, mean | 0.8606 | **0.9970** |

### What it says

- **Static-only is not a drop-in.** The best candidate loses **23%** of the
  `redundant` findings the heavy model produces (254 of 1,116).
- **The degradation is one-directional.** Every single loss is a false negative;
  neither static model ever invented a finding the heavy model would not make.
  For a guardrail that is the right direction to fail in — it under-reports, it
  does not cry wolf — but 23% is too much recall to give away.
- **Two-tier (C) does not clear the bar either.** Static-retrieve /
  heavy-rerank at K=100 recovers 89.7% of the heavy model's top-1, against the
  0.95 the plan set as the threshold for calling C viable.
- **A SOTA *code* teacher distilled *worse* than an English general-purpose
  model.** `jina-code-embeddings-0.5b` is built on Qwen2.5-Coder — a **decoder**.
  Its distilled space has a mean top-1 similarity of **0.9970**: the space has
  collapsed, everything is nearly collinear with everything. This is the known
  anisotropy of decoder embedding spaces, and averaging their token vectors
  makes it worse. **Teacher architecture matters more than code
  specialisation** — encoder teachers distil, decoder teachers do not.

### Confound eliminated

`Model2Vec::encode` defaults to `max_length=512` (`model.rs:206`) while the
heavy path truncates at 8,192. Re-run unbounded: potion moved from overlap
0.3895 → 0.3925 and 261 → 254 flips. Truncation was not a factor.

## X1.6 / X1.7 — the result that actually matters

Static retrieval topping out at recall@100 ≈ 0.90 raised the question the plan
had parked: does retrieval need an embedding at all? The index already stores
`subtokens` and `callees` per function, and near-duplicate code shares
identifiers heavily.

**X1.6 — recall of the heavy top-1 by a non-embedding prefilter** (inverted
index over subtokens, IDF-weighted, tokens present in >20% of functions skipped):

| prefilter | r@20 | r@50 | r@100 | r@200 | r@500 | build | query |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **subtokens (IDF)** | 0.8080 | 0.8820 | **0.9140** | 0.9353 | **0.9587** | **0.04 s** | 1.18 ms |
| callees (IDF) | 0.3013 | 0.3407 | 0.3593 | 0.3787 | 0.4100 | 0.03 s | 0.08 ms |
| subtokens + callees | 0.8047 | 0.8800 | 0.9120 | 0.9353 | 0.9587 | 0.04 s | 1.19 ms |

A lexical prefilter beats every static embedding at every K, and builds in
**0.04 s** against ~19 min of embedding. Callees alone are far too sparse to
retrieve with; adding them to subtokens changes nothing.

**X1.7 — finding-level quality**, the honest metric: prefilter to K candidates,
then re-rank them by *exact heavy cosine* (what the design would do). Top-1
recall understates quality, because `redundant` fires on a threshold — if the
true top-1 is missed but another candidate also clears 0.85, the finding still
fires with a valid neighbour.

| K | top-1 recall | overlap@10 | flips | FN | FP | **finding recall** | heavy embeds/fn |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 50 | 0.8693 | 0.6018 | 38 | 38 | 0 | 0.9664 | 51 |
| **100** | 0.8967 | 0.6731 | 24 | 24 | **0** | **0.9788** | 101 |
| 200 | 0.9193 | 0.7343 | 15 | 15 | 0 | 0.9867 | 201 |
| 500 | 0.9433 | 0.8075 | 7 | 7 | 0 | 0.9938 | 501 |
| 1000 | 0.9567 | 0.8518 | 3 | 3 | 0 | 0.9973 | 1001 |

(1,132 of 1,500 queries fire at 0.85 in the heavy space.)

**At K=100, 97.9% of `redundant` findings survive with zero false positives.**
At K=500, 99.4%. The best static embedding preserved **77.2%**.

### The artifact this design commits

With no vectors, the fit artifact is symbols + paths + lines + subtokens +
callees + hashes:

| artifact | raw | gzip | zstd-19 |
| --- | ---: | ---: | ---: |
| prefilter (no vectors) | 7.93 MB | 1.33 MB | **1.05 MB** |
| today's `semantic-index.json` | 58.55 MB | — | 37.34 MB |

**36× smaller compressed**, and it is *text* — identifiers — so it
delta-compresses across refits far better than quantised vectors ever could.
The whole `semantic-index-versionable.md` campaign (PCA, int8, frozen basis)
becomes unnecessary.

### What this dissolves

| problem measured earlier | under prefilter + rerank |
| --- | --- |
| 25-min CI seed, killed by timeout | fit is **0.04 s** |
| 58 MB index, 37 MB compressed | 1.05 MB compressed |
| CPU/GPU produce different indices | pure text + integer arithmetic, deterministic |
| ~100 MB model download **at fit** | no model needed at fit at all |
| index must be rebuilt per environment | trivially committable |

The heavy model is still needed **at check**, for the re-rank — 101 embeddings
per changed function at K=100 (~4.4 s on CPU, first encounter only; the
machine-wide embed cache is already content-addressed, so repeats are free).
On MSEgui's last 200 Pascal commits, a commit touches a median of **0** and a
p90 of **2** functions, so a p90 commit costs ~9 s once and less thereafter.

### Superseded by the latency constraint

**The prefilter-then-rerank design above is dead.** The requirement is that
`check` stays on the order of 100 ms, and that budget allows one or two heavy
embeddings in the entire run — not 101.

Measured on the fitted repo:

| commit | total | semantic pass | heavy embed |
| --- | ---: | ---: | ---: |
| config-only (no functions touched) | **0.41 s** | 0.05 s | — |
| adds 170 function definitions (92 candidates) | 3.23 s | 1.21 s | **0.02 s** |

Today's check already embeds **only the changed functions** — one heavy call
each — against corpus vectors that were precomputed at fit. That is exactly why
it is fast, and moving the corpus embedding into the check destroys it. On the
large commit the semantic embedding is 0.02 s; the real costs are candidate
extraction (0.58 s) and scoring (0.35 s).

**Therefore corpus vectors must stay precomputed, and the embedding must stay at
fit.** The prefilter remains interesting only as a *fit-time* screen — tested
next, and it fails.

## X1.8 — can the fit embed fewer functions?

If a cheap lexical score were a reliable lower bound on cosine, functions whose
best lexical partner falls below the cutoff would need no vector, shrinking the
one-time fit with identical findings.

- 19,437 of 26,107 functions (**74.5%**) have a ≥0.85 near-duplicate in this
  corpus — MSEgui is highly repetitive.
- Lexical score of true ≥0.85 pairs: min **0.0000**, 1st percentile 0.1669.
  Some genuine near-duplicates share almost no identifiers.
- Consequently the screen eliminates **0.0% of the corpus** at every cutoff that
  keeps findings intact (0.2% skipped at cutoff 0.30, already losing 10 pairs).

**Negative, and clean: every function needs a vector.** There is no fit-time
shortcut.

## X1.9 / X1.10 — the first verdict was measured unfairly; the fair one agrees

The P1 table above judged static models with the **heavy model's** gate (0.85)
inside the **heavy model's** space. That is rigged: argot self-calibrates the
gate per repo, and the static spaces have visibly different similarity
distributions (top-1 means 0.86 / 0.94 / 0.997 vs the heavy model's 0.90).

Redone two ways, on 2,000 queries:

1. **Separation (threshold-free AUC)** — positives = pairs the heavy model
   scores ≥0.95; negatives = **hard** ones, pairs that are lexically close but
   score <0.6 (an easy random-pair negative gave every model ≈0.99 and was
   uninformative).
2. **Matched firing rate** — each model uses *its own* gate, set so it fires on
   exactly as many queries as the heavy model does. Precision/recall then
   measures whether the pair it surfaces is genuine.

| scorer | AUC (hard neg) | pos mean | hard-neg mean | **matched precision/recall** |
| --- | ---: | ---: | ---: | ---: |
| heavy (reference) | 1.0000 | 0.9779 | 0.4652 | **1.0000** |
| jina-v2-code-static-256 | 0.9923 | 0.9731 | 0.7145 | **0.7261** |
| jina-v2-code-static-512 | 0.9924 | 0.9729 | 0.7129 | 0.7254 |
| potion-multilingual-128M | 0.9920 | 0.9639 | 0.6514 | 0.7214 |
| potion-retrieval-32M | 0.9929 | 0.9350 | 0.4396 | 0.7100 |
| potion-base-32M | 0.9911 | 0.9566 | 0.6454 | 0.7053 |
| static-retrieval-mrl-en-v1 | 0.9920 | 0.9295 | 0.4048 | 0.6986 |
| jina-code-05b-static-256 | 0.9785 | 0.9929 | 0.9737 | 0.6122 |
| lexical (IDF overlap) | 0.9741 | 0.7657 | 0.3398 | 0.5526 |

Throughput (26,982 functions, Apple silicon, all bit-identical on rerun):
11,268–25,892 fn/s — **490× to 1,126× the heavy model's 23 fn/s**.

### What seven models say

- **Every static model lands between 0.70 and 0.73.** Three training regimes
  (plain distillation, tokenlearn, MRL retrieval finetuning), four dimensions
  (256/512/1024), encoder and decoder teachers, code-specific and
  English-general vocabularies. Nothing moves the needle.
- **That convergence is the finding.** This is not model selection, it is a
  ceiling of the representation class: a bag of token vectors recovers ~72% of a
  contextual model's duplicate detection, however it is built.
- `static-retrieval-mrl-en-v1`, which the literature flags as the best static
  model *for code retrieval*, finishes second-to-last here (0.6986). Its
  vocabulary is BERT-English (30,522 tokens) and Pascal identifiers shred in it.
- **Vocabulary contributes ~+0.03**, no more: worst tokenizer for this corpus
  (0.6986) to best (0.7261).
- **An earlier excitement was wrong and is recorded as such.** With *random*
  negatives, plain lexical IDF overlap scored AUC 0.9999 — apparently matching
  the transformer. With hard negatives it drops to 0.9741, and at a matched gate
  it manages only 0.5526. Random-pair negatives are not a test.
- The decoder-teacher distillation stays pathological: hard-negative mean 0.9737
  against a positive mean of 0.9929 — a separation gap of 0.02, versus 0.51 for
  the heavy model.
- **Fusion does not help.** Blending static with lexical by score (0.62–0.70) or
  by reciprocal rank (0.62–0.68) is *worse* than static alone. The two signals
  fail on the same pairs.

### X1.11 — repo-specific vocabulary: abandoned, not measured

The one variant no off-the-shelf model can offer: distil the teacher with the
**repository's own identifiers** as vocabulary, so `formoncreate` gets a vector
instead of being averaged out of English fragments. argot is a per-repo tool, so
this is the natural idea.

It was not obtained. `model2vec`'s vocabulary extension byte-encodes each token
before adding it, so collisions surface in a doubly-encoded form
(`ValueError: Token 'ĠÄłfgrid' already exists`) that no pre-filter can predict —
filtering by ASCII, by strict identifier regex, and against the teacher's actual
61,056-token vocabulary all failed in turn. A drop-the-offender retry loop then
ran 30 min at full CPU without converging, because every collision restarts the
distillation from scratch.

Abandoned deliberately rather than fixed: the measured span between the worst
and best tokenizer for this corpus is **+0.027** (0.6986 → 0.7261), so a perfect
repo vocabulary plausibly reaches ~0.76–0.78 — a real gain that still cannot
change the verdict. Worth revisiting only if the static direction is chosen on
other grounds.

## Where P1 lands

Three levers were tested against the two hard constraints (check ≈100 ms, don't
lose findings):

| lever | outcome |
| --- | --- |
| faster embedder (static) | **dead** — 15.6% of findings lost, and it is context, not vocabulary |
| move retrieval into the check | **dead** — breaks the 100 ms budget by ~40× |
| embed fewer functions at fit | **dead** — 0% of the corpus is skippable |

The full-corpus heavy embedding is **irreducible**: ~5 min on Apple silicon with
Metal, ~19 min on a CPU runner, for 26k functions. The only remaining move is to
pay it **once** and distribute the result — which is exactly what
`semantic-index-versionable.md` measured: PCA256 + int8 with a frozen basis,
**5.2 MB committed, 0.25 MB per refit, zero gate decisions changed**. CI then
never fits, and the smaller artifact also cuts the check's fixed parse cost
(0.04 s today on the 58 MB index).

That campaign is not moot after all — this constraint selects it.

### The honest caveat (on the now-superseded prefilter design)

`overlap@10` is 0.67 at K=100 and 0.81 at K=500. `redundant` only needs the best
match and is nearly lossless; **`misplaced` votes over the top-10 neighbours**
and would degrade more. That is the open question for P2, and it is measurable
on the existing placement harness.

## Pending

- `jina-v2-code-static-{256,512}` — distilled from **the encoder teacher argot
  ships today**, so it isolates "static vs heavy" from "different teacher". This
  is the decisive candidate: the 0.5b result says the failure may be about
  decoder-vs-encoder rather than about static-vs-contextual.
  First attempt failed on `transformers` 5.3.0 (jina's `trust_remote_code`
  module imports `find_pruneable_heads_and_indices`, removed in 5.x); re-running
  under 4.57.6.

## Reading, if the encoder teacher does not close the gap

Then the ceiling is the absence of context — a bag-of-token-vectors cannot
represent what a function *does* — and the decision moves to **C′** (see the
plan): keep the heavy model, but build its index **lazily**. The machine-wide
embed cache is already content-addressed by `embed_text_hash`, so heavy vectors
computed during a check can be kept, and the heavy index materialises only over
the functions that ever surface as candidates. Fit stays at ~1 s permanently;
the re-rank cost decays toward zero as the cache fills. That keeps today's
quality and still deletes the 25-minute seed.

## Reproduction

- `crates/argot-rules-semantic/examples/static_bench.rs` (deleted after this
  record) — extraction + encode + vector dump.
- Scratch: `compare_static.py` (join on `text_hash`, top-k, flips, recall@K),
  `distill.py`, `sweep.sh`.
- `model2vec` 0.3.0 (Rust, inference) / 0.8.2 (Python, distillation).
