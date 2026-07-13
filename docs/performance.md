# Performance — contributor guide

The user-facing story is `landing/src/content/docs/performance.md`. This page
is the contributor contract: the invariants any performance change must hold,
and the instrumentation to prove it.

## The invariant

**Byte-identical output at any speed.** Same repo + window + model → the same
findings and the same card, bit for bit, cold or warm cache, 1 core or 16.
Two consequences:

- Embedding vectors are canonicalized to **f16 at the source** (fresh compute,
  artifact load, cache load all pass through the same canonicalization), so a
  cache hit vs miss is structurally incapable of moving a cosine.
- Parallelism is **per-item map only**; every reduction/aggregation that
  touches floats or ordering (threshold aggregation, evidence assembly,
  candidate ranking) stays sequential and deterministically ordered.

Any perf PR must show (a) a before/after per-phase table and (b) a byte-diff
of `audit`/`check` output on a pinned clone + pinned window. Method and the
founding numbers: `docs/research/evidence/audit-runtime.md`. Cautionary
precedent recorded there: **sequence batching was implemented, measured
(~1.2×), and reverted** because the packed-ubatch pooling flipped a cosine
tie and changed a `redundant` finding's evidence — the byte-diff caught it.
Drop the lever, not the invariant.

## Instrumentation

`ARGOT_TIMING=1` prints per-phase wall-clock lines to stderr: train,
calibrate (per language, base vs placement), semantic index (embed vs reuse
vs serialize), arch graph, integrity mini-replay, check (statistical scoring
vs semantic query embed vs candidate scoring vs integrity pass),
attribution. When a phase gets slower, this is the regression tripwire —
run it before profiling anything fancier.

## The embedding cache

`~/.cache/argot/embeddings/<model-sha16>/` — immutable f16 segments keyed by
the hash of the exact embed text. Properties to preserve:

- **Pure accelerator**: deleting it (or any entry) only costs recompute; the
  per-repo `.argot/semantic-index.json` stays the artifact of record.
- **Model identity in the key**: a model bump changes `<model-sha16>` and
  starts a fresh namespace — no cross-model reuse, no invalidation logic.
- Reuse priority at fit: prior index (seed) → cache → fresh embed; every
  source warms the cache.

## Known cost centers (rocksdb-class repos)

From the founding phase split (see the evidence memo): semantic embed of the
full corpus dominates fresh fits; placement calibration, base calibration,
and check-time semantic candidate scoring dominate the seeded floor; the
integrity mini-replay (~150 commits) rides on every fit. All of these are
per-item independent — keep them parallel, keep their aggregation ordered.

## What was deliberately rejected

Seeding the audit worktree fit with the main repo's `integrity.json` (and
the semantic reinvention replay): the mini-replay anchors at the fitted
checkout's HEAD, so the main-repo artifact is learned on a window that
*contains the audited commits* — reusing it would let the code under audit
shape its own gates. Don't revisit without solving the anchoring.
