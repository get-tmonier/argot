---
title: Performance
description: Where the time goes, the machine-wide embedding cache, multi-core calibration — and the one rule behind all of it, results that never change with speed.
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

- **`fit`** (once, then refreshed locally when the snapshot is stale) — walk the history,
  train the statistical voice model, calibrate thresholds, build the
  semantic index (embed every function), fit the architecture graph, and
  learn the test-integrity gates from a replay of accepted history. The
  embedded static table makes the semantic pass inexpensive; on a large
  repository, history processing, calibration, and the integrity replay can
  matter more than indexing the functions.
- **`check`** (every diff) — statistical scoring per hunk, plus one
  embedding + index search per *new function the diff defines*, plus the
  architecture and integrity passes.
- **`audit`** (on demand) — a full fit *as of the window's base commit* in a
  temporary worktree, then a check of everything since. This is the
  worst-case path: it is exactly why the engine work below exists.

## The two mechanisms

### 1. A machine-wide, content-addressed embedding cache

Every embedding is stored under `~/.cache/argot/embeddings/`, keyed by the
embedding model's identity and the *content* of the function. A function
that hasn't changed is never embedded twice — not across commits, not across
clones or worktrees of the same repo, not across repos that share vendored
code. A fresh clone of a repo your machine has already seen rebuilds its
semantic index mostly from cache. Bumping the embedding model invalidates
the cache naturally (it's part of the key); stale entries can be deleted at
any time — the cache is a pure accelerator, never a source of truth.

The cache is only safe because embeddings are canonicalized to f16 at the
source: the encoder's f32 output jitters in its low bits run-to-run (Metal
reduction order), but rounds to the same f16 that the index and the cache
store — so a cache-served vector and a freshly computed one are bit-identical,
and a cache hit can never change a finding.

### 2. Multi-core calibration and scoring

Calibration probes, semantic candidate scoring at check time, and the
integrity replay are all per-item-independent computations — so they run on
every core. The order-sensitive part (threshold aggregation, evidence
assembly) stays sequential, which is what keeps the output independent of
the core count. (Sequence-level batching of the embedder was tried and
dropped — it perturbed the low bits enough to flip a cosine tie and change a
finding; the invariant wins over the lever.)

**Sharing the machine:** set `ARGOT_THREADS=<n>` to cap every worker pool in
the binary — the calibration/scoring phases and the embedder's compute
threads — so a `fit` in a pre-commit hook doesn't saturate the machine while
you build. It only changes wall-clock, never a finding. There is no memory
knob: peak RSS is dominated by the embedding model plus the per-thread
working set, so lowering `ARGOT_THREADS` also lowers peak memory.

## Measured numbers

No released, canonical timing dataset is currently approved for public
performance claims. Hardware, repository shape, history depth, changed range,
and whether the embedding cache is warm materially affect `fit`, `check`, and
`audit` time. Use `ARGOT_TIMING=1` on the repository and command you care about,
and retain the command, revision, hardware, cache state, and range with any
comparison. Benchmark methodology and result provenance belong in the
[research evidence](https://github.com/get-tmonier/argot/tree/main/docs/research/evidence).

## Diagnostics

Set `ARGOT_TIMING=1` to print a per-phase wall-clock split to stderr on any
command (train, calibrate, semantic index — embed vs reuse —, arch graph,
integrity replay, check phases, attribution). If argot ever feels slow,
that split is the first thing to attach to an issue.

## CI

The [GitHub Action](/docs/ci/) reads the committed fit snapshot from the base
commit and never fits on an ephemeral runner. The machine-wide embedding cache
remains a local accelerator for the maintainer who refreshes that snapshot.
