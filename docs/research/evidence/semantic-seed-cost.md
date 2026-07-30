# The semantic seed on a large repo — why the first CI run cannot be optimised out

**Date:** 2026-07-29
**Trigger:** the first `argot` CI run on a fork of MSEide/MSEgui
([PR #1](https://github.com/damienmeur/mseide-msegui/pull/1)) was killed by
`timeout-minutes: 30` and wrote nothing to disk. Raised in response to
[#312](https://github.com/get-tmonier/argot/issues/312) — "can argot handle a
large Pascal project?".
**Verdict:** the cost is irreducible by tuning (best realistic gain ~1.4×). The
problem is **where the cost is paid and what survives it**, not how fast the
encoder is.

## The run

MSEide/MSEgui: 924k lines, 919 supported files, 9,836 commits, Object Pascal.
`ubuntu-latest`, cold caches, `pull_request` event.

| phase | wall clock |
| --- | --- |
| install binary + fetch model + plan | 4 s |
| **voice fit** (train + calibrate + arch + integrity) | **4 min 32 s** |
| **semantic index** — 35,924 functions | **25 min 25 s, SIGKILLed, incomplete** |
| total | 30 min 01 s |

`Terminate orphan process: pid (2352) (argot)`. No newer run superseded it; this
is the job timeout.

Two things made it worse than it had to be, both independent of encoder speed:

1. **The fit ran without the repository's own `argot.toml`.** The action fits
   the PR's *base* tree, and the config is introduced *by the PR* — so every
   exclusion the author justified (vendored ZeosLib, generated `*_mfm.pas`,
   forked FCL units, demo apps) was ignored. 35,924 functions instead of the
   26,107 the same repo produces with the config applied: **+38% on precisely
   the run that already fails.**
2. **The cache it was building is discarded anyway.** A cache written on a
   `pull_request` event is scoped to that PR's merge ref — neither the default
   branch nor any other PR can read it. The producer that serves everyone is the
   post-merge `push: main` run.

## Nothing survived

`crates/argot-rules-voice/src/scoring/calibration.rs`: the additive groups'
`fit_language` pass (which builds the semantic index) runs at line 1710;
`write_atomic(output, …)` for `scorer-config.json` runs at line 1767 — **after**.

`crates/argot-rules-semantic/src/index.rs`: `build_with_reuse` embeds the whole
residual in one `embedder.embed(&texts)` call and only then calls
`cache.persist(&persist)` — the machine-wide embed cache is written **once, at
the end**.

So a fit killed at minute 30 loses:

- the voice model, which had been fully calibrated since **minute 4.5**;
- every one of the ~35,000 embeddings it computed.

The next run restarts from zero and fails identically. This is not a slow path,
it is a **non-converging** one. That is the defect; the duration is a symptom.

## Is the encoder tunable? Measurement

`crates/argot-rules-semantic/examples/embed_bench.rs` (deleted after this
record). 400 Pascal functions from the corpus — 42,985 tokens, mean 107,
p50 51, p90 202, p99 1,008, max 3,953. Host: 11-core Apple silicon; `cpu-*` rows
force `n_gpu_layers=0`, `gpu-*` rows use Metal. `digest` = SHA-256 over the
f16-canonicalised vectors: **equal digest ⇒ bit-identical findings**.

| mode | n_ctx | threads | workers | secs | fn/s | digest |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| cpu-seq | 8192 | default | 1 | 27.20 | 14.7 | `de1fa4c0` |
| cpu-seq | 4096 | default | 1 | 27.62 | 14.5 | `de1fa4c0` |
| cpu-seq | 2048 | default | 1 | 23.93 | 16.7 | `eb0ed593` |
| cpu-seq | 1024 | default | 1 | 18.83 | 21.2 | `6bb06624` |
| cpu-seq | 2048 | 1 | 1 | 68.21 | 5.9 | `eb0ed593` |
| cpu-seq | 2048 | 2 | 1 | 41.47 | 9.6 | `eb0ed593` |
| cpu-seq | 2048 | 4 | 1 | 31.51 | 12.7 | `eb0ed593` |
| cpu-seq | 2048 | 8 | 1 | **17.32** | **23.1** | `eb0ed593` |
| cpu-par | 2048 | 2 | 2 | 21.52 | 18.6 | `eb0ed593` |
| cpu-par | 2048 | 2 | 4 | 17.39 | 23.0 | `eb0ed593` |
| cpu-par | 2048 | 1 | 4 | 30.45 | 13.1 | `eb0ed593` |
| cpu-par | 2048 | 1 | 8 | 19.43 | 20.6 | `eb0ed593` |
| gpu-seq | 8192 | default | 1 | 5.51 | 72.6 | `65a4d46d` |
| gpu-seq | 2048 | default | 1 | 5.08 | 78.7 | `8bf42052` |
| gpu-par | 2048 | 1 | 4 | 4.62 | 86.5 | `8bf42052` |

### What it says

- **`N_CTX` is not a speed lever.** 8192 → 4096 is bit-identical (no function in
  the corpus exceeds 4096 tokens) and the same wall clock. It halves the KV and
  compute buffers, so it is worth doing for memory — not for time.
- **The 2048/1024 "speedups" are truncation, not optimisation.** The digest
  changes: those runs embed less of the longer functions. A semantics change
  that would need full re-validation, for ~1.3×.
- **Thread count does not change the vector.** `eb0ed593` is constant across
  1/2/4/8 threads. Parallelism is bit-safe here — the f16 canonicalisation
  absorbs the reduction-order jitter, exactly as `embedder.rs` claims.
- **argot never sets `n_threads` unless `ARGOT_THREADS` is set.** llama.cpp's
  default loses 1.38× against explicit 8 (23.93 s → 17.32 s) on this host. This
  is the one free, bit-identical win available.
- **Parallel contexts buy nothing** over intra-decode threading (17.39 s vs
  17.32 s). The idea is dead — intra-op parallelism already saturates.
- **CPU ceiling ≈ 23 fn/s**, GPU (Metal) ≈ 87 fn/s — 3.7×, and irrelevant to CI,
  which has no GPU.
- **CPU and GPU digests differ** (`de1fa4c0` vs `65a4d46d` at the same n_ctx).
  The f16 canonicalisation absorbs *within-backend* jitter, not *cross-backend*.
  A Metal fit on a laptop and a CPU fit in CI are different indices. Both are
  valid; the "bit-identical findings" guarantee is backend-local, and the
  machine-wide embed cache is keyed on text only. Not load-bearing today, but
  the docstring in `embedder.rs` overstates it.

### The arithmetic that matters

The measured CPU ceiling extrapolates cleanly onto the observed CI run:

- 35,924 fn ÷ 23 fn/s ≈ **26 min** — the run spent 25.4 min and had not finished.
- 26,107 fn (with `argot.toml` applied) ÷ 23 fn/s ≈ **19 min**.

Applying every safe tuning lever at once (explicit threads, right-sized n_ctx)
takes the seed from ~26 min to ~19 min. **A 30-minute job does not become
acceptable by becoming a 19-minute job.** Speed is not the answer.

## What this rules out

- Token capping — no fat tail: capping at 4,096 tokens retains 99.8% of total
  tokens, at 2,048 still 97.9%. Nothing to reclaim.
- Parallel embedding contexts — measured, no gain.
- Lowering `N_CTX` for speed — measured, no gain.
- A smaller model — would change every finding; not costed here.

## What it points at

The cost is real and irreducible at ~20 min for a 26k-function repo on a CPU
runner. Therefore the design must make that cost **survivable and rarely paid**,
not smaller:

1. **Make the fit produce durable value incrementally** — write
   `scorer-config.json` as soon as voice calibration completes (it does not
   depend on the additive groups: they write their own `.argot/` siblings), and
   persist embeddings to the machine cache in batches rather than once at the
   end. A killed fit then leaves a working guardrail plus N thousand reusable
   vectors, and the next run converges instead of repeating.
2. **Bound it** — a wall-clock budget for the index, safe only once (1) holds.
   An incomplete index must keep the semantic rules inactive: a partial haystack
   makes `redundant` miss (harmless) but skews `misplaced`'s placement
   calibration (not harmless).
3. **Never seed on a pull request** — the artifact is discarded by GitHub's
   cache scoping regardless. Voice-only on the seeding PR is a 4.5-minute run
   with real findings; `check` already degrades cleanly when the index is absent
   (`detector.rs:297` — "its absence just means no semantic layer").
4. **Fit with the head's config** — +38% corpus on the seeding run, for nothing.
5. **Set `n_threads`** — free, bit-identical, ~1.4×.

## Related

- `not-authored-here-signal.md` — the same lever upstream: vendored and
  generated trees never reach the embedder at all.
