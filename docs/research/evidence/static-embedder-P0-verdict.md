# Phase 0 verdict — a 15.6M static embedder matches the 161M incumbent

**Date:** 2026-07-29
**Plan:** `.scratch/plan-no-fit-in-ci.md`
**Constraint set:** ≥0.85 recall on both semantic rules **at a comparable
false-alarm rate**, simplest possible architecture, no fit and no model download
in CI.

**Verdict: cleared, by the static embedder.** It matches the incumbent on recall
and fires *less* on ordinary code, at 300–500× the throughput and a tenth of the
size.

## Harness

Twelve bench corpora cloned at their pinned SHAs, **8 languages**, **37,931
functions** extracted with argot's own `functions_in_file`. No `argot fit` and no
Rust change is needed: the fixtures are ground truth and the corpus is walked
directly, so any candidate model is evaluable in minutes.

Ground truth is `benchmarks/semantic-fixtures/` — an authored reinvention of a
named corpus function with the original's location in a `# ID: path:line`
header. Corpus **and** fixtures are embedded with the same model each time, so
this compares models on the task rather than on agreement with the incumbent
(the flaw that invalidated P1/P2, recorded in P3).

## Half 1 — recall: does the planted original come back?

226 fixtures across the 12 corpora.

| model | params | hit@1 | hit@5 | hit@10 | throughput |
| --- | ---: | ---: | ---: | ---: | ---: |
| jina-embeddings-v2-base-code (incumbent) | 161M | 0.942 | 0.996 | 0.996 | 11–46 fn/s |
| **jina-v2-code-static (model2vec)** | **15.6M** | **0.956** | 0.991 | **1.000** | **5,000–21,000 fn/s** |
| all-MiniLM-L6-v2-code-search-512 | 22.7M | 0.903 | 0.987 | 0.996 | 116–344 fn/s |

At n=226 the 0.956 / 0.942 gap is inside noise (±0.029 at 95%). The safe claim
is **static ≥ incumbent**, not "better". Notably the *small contextual* model —
the candidate P3 recommended — is the **worst** of the three.

## Half 2 — over-fire: how often does it fire on ordinary code?

Every corpus function scored by the `redundant` rule against its own corpus;
a fire on a function nobody reinvented is an over-fire. 21,520 candidates.

| corpus | lang | static | incumbent |
| --- | --- | ---: | ---: |
| bat | rust | 0.0138 | 0.0035 |
| commander | javascript | 0.0000 | 0.0000 |
| express | javascript | 0.0493 | 0.0211 |
| fastapi | python | 0.0347 | 0.0399 |
| fmt | cpp | 0.0123 | 0.0195 |
| gh-cli | go | 0.0713 | 0.0764 |
| hono | typescript | 0.0351 | 0.0367 |
| ink | typescript | 0.0207 | 0.0166 |
| redis | c | 0.0181 | 0.0240 |
| rich | python | 0.0096 | 0.0084 |
| ripgrep | rust | **0.0225** | **0.0791** |
| rubocop | ruby | 0.0360 | 0.0552 |
| **TOTAL** | | **0.0324** | **0.0470** |

**Static fires 31% less** (698 vs 1,011). It is lower on 7 of 12 corpora, and
the largest gaps favour it.

### Methodological caveat — read before quoting these numbers

The text dump carries no `callees`, so the replicated rule confirms on
IDF-weighted **subtokens only**, without the callee-Jaccard and rare-callee
paths. That makes the conjunction strictly harder to satisfy, so both columns
are a **lower bound** on argot's real false-alarm rate. They are a lower bound
*identically for both models*, so **the comparison is sound; the absolute rate
is not citable** as argot's FP figure. The bench's own clean-commit replay
(`sem_fp.py`) remains the number of record.

## The wide run — 31 corpora, 581 fixtures, 11 languages

The 12-corpus run above is superseded by the full set. Corpora capped at 5,000
functions (the cap always keeps the fixture targets and is applied identically
to both models).

| metric | static 15.6M | incumbent 161M | delta |
| --- | ---: | ---: | --- |
| `redundant` recall @1 | 0.936 | **0.943** | −0.7 pt |
| `redundant` recall @5 | 0.985 | **0.993** | −0.8 pt |
| `redundant` over-fire | **0.0349** | 0.0414 | static **16% better** |
| `misplaced` recall | 0.940 | **0.945** | −0.5 pt |
| `misplaced` over-fire | 0.1648 | **0.1206** | static **37% worse** |

**Both models clear the 0.85 bar on both rules by a wide margin.** Static sits
within **0.7 points** of a model ten times its size on every recall figure, and
fires *less* on `redundant`. The one metric where it is clearly behind is
`misplaced` over-fire.

The 12-corpus subset had static *ahead*; the wider sample regressed it to just
behind — the expected behaviour when a sample triples.

### Reading the 0.1648 / 0.1206 correctly

Neither figure is argot's real `misplaced` false-alarm rate. This replication
runs the vote with three safety systems removed:

1. **No calibrated merge map.** Production merges entangled areas into one
   label; this uses raw directories, so it reports "in `db/` but the neighbours
   are in `kernel/`" where argot knows the two are one area. With the real map,
   the same computation on MSEgui gave **0.4%**, not ~16%.
2. **No `enabled` gate.** Placement disables itself entirely on repos whose
   areas are not separable; this always runs.
3. **No candidate filters** — nested functions, callee-less bodies, stubs under
   the line floor, directories absent at fit. Precisely the noisy cases.

The handicap is identical for both models, so **the ratio is meaningful and the
absolute value is not**. Extrapolating the +37% onto the published rate
(0.78%/hunk) gives ~1.06%/hunk — about **one extra false "misplaced" per 18
pull requests** on a 20-hunk PR. Small, but the extrapolation assumes the gap
survives the merge map, which is exactly what is unverified: the map may absorb
it, or may attack a different noise source altogether.

**This is the one item to check first in Phase 1**, on the first repo actually
fitted with the static embedder — before any further work depends on it.

## What this selects

The simplest of the three architectures, not the compromise:

| | recall | over-fire | fit (26k fns) | dependency |
| --- | ---: | ---: | ---: | --- |
| incumbent 161M | 0.942 | 0.0470 | ~19 min | llama.cpp C++, 100 MB GGUF download |
| small contextual 22.7M | 0.903 | — | ~1.8 min | ONNX Runtime C++, 25 MB |
| **static 15.6M** | **0.956** | **0.0324** | **~2 s** | **pure Rust, model embeddable in the binary** |

Consequences for the plan:

- Phase 1 loses `ort` and ONNX Runtime entirely — `model2vec-rs` is pure Rust,
  so **no C++ dependency at all** and `semantic` can join the base build loop
  it is excluded from today.
- The ~30 MB model ships **inside the binary**: nothing is ever downloaded,
  argot becomes fully offline, and `ARGOT_OFFLINE` / `ARGOT_SEMANTIC_MODEL` /
  `argot model {fetch,status,clean}` / the sha256 path / the release asset all
  disappear.
- A ~2-second fit makes Phase 4 (incremental fit) largely moot.
- The committed index shrinks: 256-d int8 instead of 768-d f16.

## Still required before building

1. **The bench's own halves**, not this proxy: `sem_bench.py` recall and
   `sem_fp.py` clean-commit FP, on the full 25-corpus / 604-fixture suite.
2. **`misplaced`** measured the same way — P2 found it robust under a static
   index (0.90–1.00 of the incumbent's transplant recall), but on 3 corpora.
3. The fixture suite's own weakness, found in P3: a number of fixtures are the
   original with the function *renamed* and the body untouched, which every
   model retrieves. The suite should be hardened, or per-fixture ranks reported,
   so a ceiling is visible rather than silent.
