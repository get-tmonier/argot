# The check path is serial — diagnosis, and what parallelising it needs

**Date:** 2026-07-28 · **Status:** **done.** Diagnosed, then implemented the
same day. 47,0 s → 13,4 s on a 921-file changeset, output byte-identical,
35 of 35 bench corpora unchanged.

**Question:** the semantic pass is parallel. `check: score patches
(statistical)` is not. How much does that cost, and what stands in the way?

## The measurement

Scoring MSEide/MSEgui as one changeset — 921 files, 924 048 lines, every file's
content treated as added:

```sh
EMPTY=$(git commit-tree $(git hash-object -t tree /dev/null) -m e)
argot check "$EMPTY..4233521f2" \
  --rule voice=off --rule semantic=off --rule architecture=off --rule integrity=off
```

| | wall | CPU |
|---|--:|--:|
| every rule off, **including** the scripted ones | 47,0 s | **97 %** |
| the four scripted rules on | 48,7 s | 97 % |

Two things to read off this.

**`--rule voice=off` does not stop the voice scoring.** `VoiceDetector::enabled`
returns `true` unconditionally — "it owns the scan statistics (hunk/file counts
in the report meta)". So the 47 s *is* the statistical pass; the scripted rules
add 1,7 s (3,5 %) on top of it.

**97 % CPU is one core of eleven.** The same machine runs the sdl2 branch check
at 206 % because the semantic layer is parallel there. The base path is not.

Per-hunk this is ~51 ms, consistent across workloads (sdl2: 91 hunks, 5,1 s
*with* semantic on ≈ 56 ms/hunk). Nobody hands argot a whole repository, so
this is not a defect users hit today — but it is the ceiling on every large
changeset, and on `audit`, which replays hundreds of them (4 min 26 s here).

## Why it is serial

`score_patches` in `argot-rules-voice/src/detector.rs` is `for batch in patches`
— one iteration per file — and takes `&mut SequentialImportBpeScorer` from a
`HashMap`. The `&mut` is what blocks a `par_map_indexed`.

**The scoring itself is pure.** Everything mutable on the hot path is
incidental:

| mutable state | what it is | why it does not block the design |
|---|---|---|
| `SequentialImportBpeScorer::file_cache` | single-entry per-file memo of prose rows, data rows and file bindings | its own doc says "Pure memoization — scoring is byte-identical with or without it" |
| `CallReceiverScorer::hunks_scored` | diagnostic counter | `+= 1`, never read by scoring |
| `CallReceiverScorer::rare_branch_fire_count` | diagnostic counter | same |

Plus four cross-batch accumulators in the loop itself — `hunk_count`,
`file_counts`, `warned`, and `alerted_foreign_modules` (the per-changeset
novel-import dedup). The last one is order-dependent by design: the first
appearance of a foreign module alerts and the rest dedup.

## The shape of the change

1. **Lift the memo cache out of the scorer** into a batch-local value threaded
   through `score_hunk`. This removes hidden mutable state rather than adding
   synchronisation around it, and it is what makes the scorer `&self`.
2. **Counters to atomics**, or accumulate per worker and sum.
3. **Two phases.** Score every batch in parallel into per-batch results, then
   merge *in original order* applying the dedup and the counters. Order-
   dependent behaviour stays in the serial phase, so output is unchanged.

## Outcome

Implemented. The estimate below was wrong in the safe direction: **no call site
changed**, because `&mut self` → `&self` reborrows at the call. What the change
actually needed was mostly *deletion* —

- two `CallReceiverScorer` counters were **written and never read**, removed;
- the per-file memo moved to a **thread-local** (each worker takes a contiguous
  run of whole files, so it hits exactly as often as it did serially);
- four shape primitives memoed their language in a `Cell` → `OnceLock`;
- the two counters that *are* read → `AtomicUsize` / `Mutex`, contended only
  under the bench;
- `LanguageAdapter` and `ShapePrimitive` gained `: Sync`.

| workload | before | after |
|---|--:|--:|
| whole tree, 921 files / 924 048 lines | 47,0 s @ 97 % CPU | **13,4 s @ 407 %** |
| `argot audit --commits 400` | 4 min 26 s | **3 min 34 s** |
| sieghard, 239 hunks | 9,3 s | 7,7 s |

Byte-identical, three ways: every golden suite green; the full hit records for
sdl2, sieghard and X11_clean compare equal field for field (scores, thresholds,
hashes, evidence); and the full bench shows **35 of 35 pre-existing corpora with
identical false-positive rows, none differing**, headline 647/756 = 85,6 %,
0 errors.

## The estimate that was wrong

`score_hunk` has **26 call sites** across the CLI, the bench harness, the MCP
path, `sequential_golden.rs` and `model_snapshot.rs`. It is the hottest
function in the codebase and it is parity-locked by the golden suites: the
change is only acceptable if the output is byte-identical, which means the
goldens *and* a full 39-corpus bench to prove no drift.

That is a session's work with a 45-minute bench behind it. A half-finished
refactor of this path is worse than none, so it is written down rather than
started.

## Expected win

If the per-batch work parallelises at the ratio the semantic pass achieves,
the 47 s becomes single-digit seconds on this workload, and `argot audit`
— which replays the same scoring hundreds of times — gains proportionally.
Nothing here changes what argot reports; it is wall-clock only.
