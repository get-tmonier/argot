---
title: Performance
description: Where the time goes, the embedding cache, batched inference, multi-core calibration — and the one rule behind all of it, results that never change with speed.
group: Reference
order: 12
---

argot is a single static binary that does everything in-process — fit, check,
audit, the semantic index, the architecture graph, the test-integrity gates.
Its performance design follows one rule and three mechanisms.

**The rule: speed never changes a result.** Same repo, same window, same
model → the same findings, byte for byte, whether the caches were cold or
warm and whatever the core count. Parallelism and caching are allowed to
change *when* you get the answer, never *what* it is. Embedding vectors are
canonicalized to f16 at the source, so a cached vector and a freshly computed
one are bit-identical by construction; parallel work is per-item only, and
aggregation stays sequential and ordered.

## Where the time goes

- **`fit`** (once, then refreshed in the background) — walk the history,
  train the statistical voice model, calibrate thresholds, build the
  semantic index (embed every function), fit the architecture graph, and
  learn the test-integrity gates from a replay of accepted history. On big
  monorepos the semantic index dominates: tens of thousands of functions
  through the embedding model.
- **`check`** (every diff) — statistical scoring per hunk, plus one
  embedding + index search per *new function the diff defines*, plus the
  architecture and integrity passes.
- **`audit`** (on demand) — a full fit *as of the window's base commit* in a
  temporary worktree, then a check of everything since. This is the
  worst-case path: it is exactly why the engine work below exists.

## The three mechanisms

### 1. A machine-wide, content-addressed embedding cache

Every embedding is stored under `~/.cache/argot/embeddings/`, keyed by the
embedding model's identity and the *content* of the function. A function
that hasn't changed is never embedded twice — not across commits, not across
clones or worktrees of the same repo, not across repos that share vendored
code. A fresh clone of a repo your machine has already seen rebuilds its
semantic index mostly from cache. Bumping the embedding model invalidates
the cache naturally (it's part of the key); stale entries can be deleted at
any time — the cache is a pure accelerator, never a source of truth.

### 2. Batched embedding

Functions are packed into multi-sequence batches (up to 64 sequences per
decode) instead of being fed to llama.cpp one at a time. The model is an
encoder (jina-code, ~100 MB, CPU-first, Metal-accelerated on Macs) — batching
is where most of the raw embedding throughput comes from.

### 3. Multi-core calibration and scoring

Calibration probes, semantic candidate scoring at check time, and the
integrity replay are all per-item-independent computations — so they run on
every core. The order-sensitive part (threshold aggregation, evidence
assembly) stays sequential, which is what keeps the output independent of
the core count.

## Measured numbers

On FastAPI (1,100-file repo, laptop CPU): first `fit` ~25 s, background
refresh ~4 s, `check` ~0.2 s per diff (~0.6 s when the diff defines new
functions). Large-monorepo numbers (rocksdb-class: ~30k functions) are being
re-baselined with the engine work above and published in the benchmarks —
the phase-split methodology and raw runs live in the
[research log](/docs/research/).

## Diagnostics

Set `ARGOT_TIMING=1` to print a per-phase wall-clock split to stderr on any
command (train, calibrate, semantic index — embed vs reuse —, arch graph,
integrity replay, check phases, attribution). If argot ever feels slow,
that split is the first thing to attach to an issue.

## CI

The [GitHub Action](/docs/ci/) caches `.argot/` per base SHA. On large
repos, also cache `~/.cache/argot` (the model download and the embedding
cache) between runs — fits on ephemeral runners then start warm.
