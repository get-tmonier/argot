# P0 — Embedder spike in `llama-cpp-2` — RESULT: parity 1.0, gate G6 PASS

Status: **green — llama.cpp static link + jina-code Q4 + Metal + masked mean-pool
all proven in-process.** Date: 2026-07-07. Bin: `scratchpad/llama-spike/`
(throwaway, `llama-cpp-2 = 0.1.151`, feature `metal`).

## What passed

- **Static link works.** `llama-cpp-2` builds llama.cpp from source via cmake and
  links it statically — the same in-process C-dep shape as `git2`/libgit2. cmake
  was the only missing prerequisite (`brew install cmake`; Apple clang already
  present).
- **Metal works.** Kernels compiled/loaded; embeddings ran on the GPU.
- **jina-code Q4 GGUF loads and embeds in-process** using
  `LlamaPoolingType::Mean` — llama.cpp does the **masked** mean-pool internally
  (no manual pooling, so the E1 pad-token bug can't recur). `with_embeddings(true)`
  + `embeddings_seq_ith(0)`, then L2-normalize.
- **Gate G6 — parity ≥0.99: PASS at 1.00000.** All 15/15 `all_texts.json` probes
  hit cosine **1.00000** vs a same-model reference (jina-code Q4 via ollama =
  llama.cpp). median 1.00000, min 1.00000.
- **Perf (Metal, this machine):** model load ~0.06 s (warm OS cache), check-time
  **~20 ms/fn warm** (34 ms cold), 15 ms/fn batched. Matches the exploration's
  ~29 ms budget. Compute buffers ~21.5 MiB Metal + 3.5 MiB CPU.

## The one gotcha (cost me a red run, then fixed)

`ollama_ref.json` from the exploration is the **base-en** reference (confirmed:
cosine 0.9999 vs candle's base-en `vectors.json`). Comparing my **jina-code** Q4
output against a **base-en** reference gave ~0 cosine (different embedding spaces).
Regenerating the reference with the *same* model (jina-code Q4 via ollama) →
cosine 1.0. **Lesson for the bench: always compare within one model.**

Benign log noise from llama.cpp on encoder models (auto-handled, results exact):
`decode: cannot decode batches with this context (calling encode() instead)` and
`embeddings required but some input tokens were not marked as outputs -> overriding`.

## Pinned model (decision: mirror as argot-owned asset)

- Model: `jina-embeddings-v2-base-code`, **Q4_K_M GGUF**, 109.5 MB.
- **sha256 `1cea691a59c9aeb48f5a95d631f51a8f67850eb6638398c88343de8a6815b496`**
  (the exact bytes validated at parity 1.0 — the ollama blob name *is* this sha).
- Source today: only the community `gandolfi/jina-embeddings-v2-base-code-Q4_K_M-GGUF`
  has Q4 on HF (`ggml-org` has only Q8-code). Decision: **re-host these exact
  bytes as an argot-controlled release asset**, pin URL + this sha256, fetch-on-
  first-use. Robust against the community repo disappearing.

## Decisions taken with the user at this checkpoint

- **Release shape:** single published binary with `semantic` built-in (release
  builds `--features semantic`). Still feature-gated at compile time so the base
  path builds lean and pays zero runtime cost unless semantic is actively used.
- **Scope:** F1 + F4 + F2 all in v1 (build order F1+F4 → F2).
- **Model hosting:** mirror the Q4 GGUF as an argot-owned asset + pinned sha256.

## Verdict

P0 de-risked. The embedder is a real but bounded subsystem: link + load + Mean-pool
+ normalize, all proven. Proceed to integrate behind `feature = "semantic"`.
