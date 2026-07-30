# P3 — is it the engine, the model, or the measurement?

**Date:** 2026-07-29
**Constraints set:** ≥0.85 recall on **both** semantic rules at a comparable
false-alarm rate, drastically simpler architecture, and the fit must be
affordable **in CI** (a team cannot depend on whoever last ran it locally, and
the semantic index should be the *freshest* artifact, not the stalest).

## 1. The engine is not the bottleneck

argot runs jina-embeddings-v2-base-code — a BERT-family **encoder** — through
llama.cpp, which decodes one sequence at a time. Published Rust benchmarks say
ONNX Runtime is ~14× Candle for this family, so the same weights were tried on
ONNX (the model ships `model.onnx`, `model_fp16.onnx`, `model_quantized.onnx`).

| engine | throughput |
| --- | ---: |
| llama.cpp, GGUF Q4, 8 threads | ~2,482 tok/s |
| ONNX Runtime, quantized, 8 threads | ~2,750 tok/s |

**1.1×.** The 14× figure was against Candle, not against llama.cpp, which is
already a well-optimised CPU kernel. Batching did not help either — padding
waste cancels it when function lengths vary (batch 64 was *slower* than batch 8).

The corpus is **2,775,313 tokens** (26,982 MSEgui functions, measured). At ~2,700
tok/s that is ~17 minutes on any engine. **The cost is the model's 161M
parameters, not the runtime.**

## 2. A small contextual model is 10× faster

`all-MiniLM-L6-v2-code-search-512` — 22.7M parameters, fine-tuned for code
search — on the same corpus, PyTorch CPU, 8 threads, length-sorted batching:

| model | params | throughput | MSEgui fit |
| --- | ---: | ---: | ---: |
| jina-embeddings-v2-base-code (today) | 161M | 23 fn/s | ~19 min |
| all-MiniLM-L6-v2-code-search-512 | 22.7M | **239 fn/s** | **1.8 min** |
| model2vec static (jina-v2-code distilled) | 15.6M | 10,861 fn/s | ~2 s |

**10.4×**, and that is PyTorch; ONNX Runtime is typically another 2–3× on top.
A ~2-minute fit is an ordinary CI step, which satisfies the "refit in CI"
requirement that rules out the commit-the-index design.

## 3. Both of my quality metrics turned out to be unsound

This is the important part of P3, and it invalidates the headline numbers of P2.

**Metric A — agreement with the incumbent.** Every `redundant` number so far
(0.31 static, 0.42 projected, 0.41 MiniLM) measures *whether a candidate
reproduces the findings of the current model*. That is not the same as being
right: a different embedding surfaces different-but-equally-valid duplicates,
and `redundant`'s conjunction of tight conditions amplifies any ranking
difference into a different finding. Asking for 0.85 agreement is close to
asking for the same model.

**Metric B — the authored fixtures.** `benchmarks/semantic-fixtures/` is real
ground truth (a reinvention of a named function plus its location). Measured on
hono + fastapi, corpus and fixtures embedded with the same model each time:

| model | params | hono hit@1 | fastapi hit@1 |
| --- | ---: | ---: | ---: |
| jina-embeddings-v2-base-code | 161M | 0.950 | 0.950 |
| all-MiniLM-L6-v2-code-search-512 | 22.7M | 0.950 | 0.950 |
| model2vec static | 15.6M | **0.950** | **0.950** |

Three model classes spanning 10× in size — including a bag-of-tokens static
model — score **identically** on this 40-fixture sample.

> **Correction (same day, after all 31 corpora were dumped).** The explanation
> given here — "many fixtures are the original with the function renamed" — was
> **generalised from the worst corpus in the suite**. Measured against each
> fixture's *exact* original across all 581 resolvable fixtures: median
> similarity **0.583**, and only **42/581 = 7.2%** are identical once
> identifiers are masked. Thirty-five of those 42 sit in four TypeScript
> corpora — hono 12/20, ink 9/22, excalidraw 8/20, outline 6/20 — and this
> sample was hono + fastapi. The suite as a whole is mostly genuine rewrites,
> so it is **more trustworthy than this section claimed**; what was really
> demonstrated is that *a 40-fixture sample drawn from hono* is at ceiling.
> See `static-embedder-P0-verdict.md` for the 581-fixture run, where the
> per-corpus spread (0.800–1.000) shows the suite does discriminate.

So: metric A is too strict to be meaningful, metric B is too easy. **No sound
measurement of "what a different embedder actually costs" exists yet**, and the
P2 conclusion ("static loses 60% of `redundant`") should be read as "static
disagrees with the incumbent 60% of the time", which is a weaker claim.

## Where this leaves the decision

The standout candidate is a **small contextual code encoder** (~23M):

- keeps contextual understanding, so it is not subject to the bag-of-tokens
  ceiling that P1/P2 established for static embeddings;
- **1.8 min** for a 26k-function repo, i.e. affordable in CI on every base
  advance — the property the team-ownership argument requires;
- matches the 161M model exactly on the ground-truth fixtures;
- ships as a ~90 MB fp32 / ~25 MB int8 ONNX file — small enough to embed or
  fetch once, and `ort` replaces the llama.cpp C++ build.

## What must happen before building

The only measurement that would settle this is the **full fixture suite** —
604 fixtures across 25 corpora, via the existing `sem_all.py` harness — run for
each candidate embedder, reporting `redundant` recall *and* clean-commit
over-fire. The offline proxies in P1–P3 have reached their limit:

- agreement-with-incumbent overstates the loss,
- a 40-fixture sample understates it (everything is at ceiling).

Until that runs, no number in P1–P3 should be quoted as "what we lose".
