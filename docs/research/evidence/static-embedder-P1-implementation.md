# P1 — the static embedder, implemented and measured in the binary

**Date:** 2026-07-29
**Plan:** `.scratch/plan-no-fit-in-ci.md`
**Gate cleared:** `static-embedder-P0-verdict.md`

Everything below is measured with the real binary on real repositories, not with
an offline proxy. These are the numbers a docs page could quote.

## What shipped in this phase

- `EmbeddingModel` trait in `argot-rules-semantic` — the contract the index, the
  machine cache and both rules are written against. Two impls: the pinned GGUF
  transformer and the static distilled table.
- `StaticEmbedder` (`model2vec`) — **pure Rust**, no C++ backend, selected via
  `ARGOT_STATIC_MODEL` until the weights are embedded in the binary.
- The embedder is chosen once at fit *and* at check; its identity is written
  into the artifact and validated against the model that will query it, so an
  index built by one model can never be read by the other.
- Artifact format **v4**: int8 vectors with a shared scale, renormalised on
  load so a dot product is still a cosine.
- 58 crate tests green, 9 of them new (static path, int8 round-trip).

## Fit

MSEide/MSEgui, 26,107 Pascal functions, same machine, same repo:

| | fit (whole pipeline) |
| --- | ---: |
| transformer (jina-v2-base-code, GGUF) | ~10 min |
| **static (model2vec)** | **1 min 28** |

The semantic index is no longer a measurable share of the fit; what remains is
the voice calibration (profiled separately at ~100 s locally).

## Check latency

The check is what a developer waits for, so it is the number that matters most.

| | transformer, f16 index (58.6 MB) | static, int8 index (16.8 MB) |
| --- | ---: | ---: |
| artifact read + parse | 0.04 s | **0.03 s** |
| whole check, commit touching no function | 0.40 s | **0.34 s** |
| whole check, commit adding 170 functions | 3.23 s | **3.11 s** |

**The check got faster, not slower.** int8 is a narrower number format decoded
with one multiply — not compression — so nothing is decompressed at check time.
Git compresses the committed blob and decompresses it at `checkout`, never
during a check.

### A regression this phase introduced and fixed

Moving the embedder load earlier (to make identity validation possible) made it
**unconditional**: 0.73 s on *every* check, including those with nothing to
score, taking the check from 0.40 s to 1.08 s. Fixed by splitting validation —
`validate_format()` needs no model and runs early; `validate_for(model)` runs
after the embedder loads, which now happens only when there are candidates.
Worth remembering: the original ordering was load-bearing for latency, not
incidental.

**Still outstanding:** the static model loads in **0.80 s** because the shipped
weights are f32 (129 MB). The int8 weights exist and should cut this to ~0.2 s
when the model is embedded in the binary.

## Index size, by repository

| repo | language | functions | index file | **git blob** |
| --- | --- | ---: | ---: | ---: |
| rocksdb | C++ | 28,121 | 22.74 MB | **7.57 MB** |
| msegui | Pascal | 26,107 | 16.84 MB | 6.50 MB |
| powershell | C# | 19,524 | 15.86 MB | 5.19 MB |
| dagster | Python | 19,191 | 15.72 MB | 5.31 MB |
| laravel | PHP | 12,925 | 7.96 MB | — |
| saleor | Python | 6,493 | 5.03 MB | — |

Against the transformer's index on the same repo: **61.40 MB → 16.84 MB**
(3.6× smaller), before git's own compression.

C++ costs more per function than other languages — longer identifiers and
denser callee sets. Guava looked like the largest corpus at 55,686 extracted
functions but fits to only **24,189**: argot excludes its (very large) test
suite.

**Worst case measured: ~7.6 MB of git blob for a 28,000-function repository.**
Committable. A repo twice that size would land near 15 MB, which is where the
PCA option below becomes defensible again.

## Why int8 and not PCA

int8 in the static 256-d space, 2,000 queries on MSEgui:

| gate | threshold flips |
| --- | ---: |
| 0.85 | **0 / 2000** |
| 0.78 | **0 / 2000** |
| 0.70 | **0 / 2000** |

Zero decisions change at any of the rule's three cosine gates.

PCA was planned (it was worth 20 MB → 7.6 MB on the transformer's 768-d space)
and is **not** used, because the static model already emits 256 dimensions —
its distillation applied a PCA. Measured on top of that:

| scheme | git blob | flips @0.85 | **top-1 neighbour identity** |
| --- | ---: | ---: | ---: |
| int8 256 (shipped) | 6.50 MB | 0 | 1.000 |
| PCA192 + int8 | ~4.0 MB | 0 | 0.960 |
| PCA128 + int8 | ~3.2 MB | 1 | 0.937 |

It would still save ~2.5 MB with no threshold flips — but **4% of nearest
neighbours change identity**, and `misplaced` votes over those neighbours while
F4 evidence names one to the user. It also reintroduces an eigendecomposition, a
basis that must be stored *and frozen* (an unfrozen basis rewrites the whole
file every refit: 5.23 MB per refit instead of 0.25 MB), and an unresolved
question about cross-platform reproducibility of that decomposition. Not worth
2.5 MB when 6.5 MB is already committable. Documented as an option if a much
larger repository ever makes size blocking.

## Rule quality after the swap

Calibrated placement config, MSEgui, static vs transformer — argot chose the
**same** configuration for both (k=10, z=1, 27 merged areas):

| | static | transformer |
| --- | ---: | ---: |
| `sim_recall` | 0.9708 | 0.9749 |
| `sim_overfire` | **0.0081** | 0.0040 |

Recall is preserved (−0.4 pt). Over-fire doubles in relative terms but lands at
**0.81%**, which is where the transformer's own *published* figure already sits
(0.78%/hunk in the semantic bench) — the 0.40% here is this repository
calibrating better than average, not the norm.

The wide proxy run (31 corpora, 581 fixtures, 11 languages) is in
`static-embedder-P0-verdict.md`.

## Still to do in this phase

- Embed the weights in the binary (`include_bytes!`) and delete
  `ARGOT_STATIC_MODEL`; ship int8 weights to cut the 0.80 s load.
- Remove `llama-cpp-2` and its machinery: `ARGOT_OFFLINE`,
  `ARGOT_SEMANTIC_MODEL`, `ARGOT_MODEL_URL`, `argot model {fetch,status,clean}`,
  the sha256 path, the fetch-on-first-use degrade, the GGUF release asset.
- Move `semantic` into the base build loop — the C++ compile was its only reason
  to be CI-only.
- Re-check the rule's cosine bars against the fixture suite: they were fitted to
  the transformer's similarity distribution.
- `just verify` in full; only the semantic crate's tests have run so far.
- Dependency overlap to settle: `model2vec` pulls `tokenizers` 0.21 while argot
  is on 0.23, so both are currently compiled in.
