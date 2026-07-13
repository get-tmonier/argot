# Evidence — `argot audit` runtime on large repos

**Date:** 2026-07-13 · **Branch:** `feat/audit` · **Status:** shipped on branch
**Brief:** `.scratch/audit-command/RUNTIME-BRIEF.md` · **Parent memo:**
[`audit-command.md`](audit-command.md)

## Goal

`argot audit`'s PRD target is ≤ ~2 min for the default 50-commit window on the
largest bench corpus. rocksdb (25.7k C++ / 2.8k Java / 416 Py functions, 840
hunks in the window) and guava were the outliers. All numbers below are **solo,
uncontended, Apple-Silicon laptop, release build
`--features semantic,arch,integrity`**, model pinned via
`ARGOT_SEMANTIC_MODEL` and an isolated `XDG_CACHE_HOME`.

## Step 1 — instrument first (nobody had ever split the phases)

Added `argot_core::timing` — RAII phase guards that print `[timing] <label>:
<s>s` to stderr, inert unless `ARGOT_TIMING` is truthy (one cached env lookup
per phase otherwise). Threaded through audit (worktree / fit / check /
attribution), fit (train / calibrate), calibrate (per-language: read, BPE,
call-receiver, candidates, probe, thresholds, evidence, and each semantic
sub-step; plus arch graph and the integrity mini-replay) and check (scorer
load, patch collect, the semantic/arch/integrity passes, statistical scoring).

### Baseline phase split (rocksdb, the numbers published red)

| phase | fresh audit | fit (HEAD) | seeded audit |
|---|--:|--:|--:|
| worktree add + seed | 0.3 | — | 0.3 |
| **fit: cpp semantic embed (25.8k fns)** | **771** | 737 | 46¹ |
| fit: cpp placement calibrate | 84 | 84 | 86 |
| fit: integrity mini-replay (150 commits) | 34 | 38 | 34 |
| fit: java + python + c embed | 40 | 45 | 5¹ |
| fit: cpp base calibrate (BPE/CR/cand/probe/thr/evidence)² | 62 | 62 | 61 |
| fit: other (java placement, misc) | ~6 | ~6 | ~6 |
| **fit total** | **1022** | **1000** | **262** |
| check: semantic embed (1531 diff fns) | 32 | — | 32 |
| check: semantic score candidates | 56 | — | 56 |
| check: statistical scoring (840 hunks)² | 24 | — | 24 |
| check: integrity pass | 6 | — | 6 |
| check: other (candidate extract, decode) | 6 | — | 6 |
| **check total** | **124** | — | **124** |
| attribution | 0.3 | — | 0.3 |
| **TOTAL** | **1148** | **1000** | **387** |

¹ seeded reuses the current fit's index as a seed → ~97.7% of embeddings
reused. ² base (non-semantic) scorers — out of scope for this work and
byte-locked (must stay byte-for-byte identical), so left untouched.

**The brief's "unaccounted ~300 s" seeded floor, resolved:** it is
placement-calibrate (86 s) + base cpp calibrate (61 s) + integrity replay
(34 s) + the whole check phase (124 s). Two of these — placement calibrate and
check-time candidate scoring — were **not** in the brief's ranked hypotheses;
the timing split surfaced them.

## Levers applied

### A. Machine-wide content-addressed embedding cache (the big structural win)

`~/.cache/argot/embeddings/<model-sha16>/` — immutable fixed-record f16
segments keyed by (embedding model, embed-text hash). Reuse now flows **across
checkouts**, not just within one `.argot/`: a fresh clone, or the audit's temp
worktree, of an already-seen repo serves vectors from the cache instead of
re-embedding. `build_with_reuse` resolves prior artifact → cache → fresh embed
and warms the cache from every source, so one fit of a repo makes every future
clone/worktree/audit of it a cache hit. Concurrent-safe by construction
(per-process segments, temp+rename, oldest-first eviction at a 512 MB cap).
Check-time query embeds route through it too — on a warm cache the 1531
diff-function embeds drop from 32 s to **0.04 s** (those functions were embedded
by the HEAD fit and are already cached).

**f16 canonicalisation** underpins it: `Embedder::embed` rounds every component
to f16 — the exact precision the index artifact and the cache store — so a
freshly computed, an artifact-reloaded, and a cache-served vector are
**bit-identical**. The encoder's f32 output jitters in its low bits run-to-run
(Metal reduction order), but rounds to the same f16; a unit test asserts repeat
embeds are bit-identical, and this is what lets a cache hit stand in for a fresh
embed without changing a finding. Safe by the same equality the baseline already
relied on: on the old build the fresh audit (all-f32 in-memory corpus) and the
seeded audit (mostly-f16, reused from the seed) produced **byte-identical
cards** — proof the card is invariant to f16-vs-f32 corpus vectors.

### B. Deterministic parallelism for the three measured hot phases (`par::par_map_indexed`)

All three are single-threaded, per-item-independent loops; each now fans out
over scoped threads with contiguous index chunks reassembled **in input order**,
so outputs are element-for-element identical to the sequential code.

| phase | before | after | speedup |
|---|--:|--:|--:|
| placement calibrate (cpp, 8k neighbour scans) | 84 s | 13 s | 6.3× |
| integrity mini-replay (150 commits) | 34 s | 7 s | 5× |
| check-time semantic scoring (1531 candidates) | 56 s | 3 s | 17× |

The 17× on check scoring is partly a fix, not just parallelism:
`RedundantScorer::new` builds corpus-wide IDF/DF tables over the whole index and
was being reconstructed **per candidate** (1531× on this window); it is now
built once per language, then the read-only per-candidate evaluations run in
parallel. `par` is compiled only under the semantic/integrity features; the base
build is untouched.

## Lever rejected — sequence batching (broke byte-identity)

Packing several sequences into one llama.cpp decode was implemented and
measured — but it was only **~1.2×** on Metal (embedding is per-token
compute-bound on this stack, not decode-count-bound) and, decisively, the
rocksdb byte-identity check caught it changing the findings: the packed-ubatch
pooling shifted one function's embedding low bits enough to **flip a cosine
tie**, so a `redundant` finding's nearest-code evidence pointed at a different —
equally similar (0.81) — `CopyPrefix` definition
(`compaction_job.cc:2617` → `compaction_job_stats_test.cc:531`). The seeded
audit, which reuses the old-build vectors from the seed, stayed byte-identical;
only the re-embedded fresh path diverged, isolating batching as the cause. Per
the brief's rule — *if a lever conflicts with byte-identical findings, drop the
lever, not the invariant* — batching was reverted to one sequence per decode.
The embed cache, not batching, is the lever that makes repeat encounters fast.

## Hypothesis B4 rejected — riding the integrity artifact in with the seed

The brief floated copying `.argot/integrity.json` into the audit worktree the
way the semantic index is seeded, to skip the 150-commit mini-replay. **This is
unsound, and the brief itself asked to verify the anchoring claim first.**
`integrity::fit_model` anchors its replay at the *fitted checkout's* `HEAD`
(`repo.head().peel_to_commit()`), walking that commit's first-parent history.
In the audit worktree that HEAD is the **base** commit (50 back); a normal
main-repo fit anchors at the repo HEAD. So the two artifacts measure **different
150-commit windows**, and the main-repo one's window *contains the very commits
under audit* — seeding it would let the audited code inform its own gates
(laundering) and would not even be the correct base-anchored model. The same
argument sinks seeding the semantic *reinvention* replay. Rejected on
correctness; parallelising the replay (5×, output-identical) captured the
available time instead.

## Final measurements (rocksdb, batching-dropped build, solo)

| scenario | baseline | now | change |
|---|--:|--:|--:|
| fresh audit — cold cache (never-seen repo) | 1148 s | **1013 s** | 1.13× |
| `fit` (HEAD), warm cache | 1000 s | **164 s** | 6.1× |
| **seeded audit** (after a fit) | 387 s | **136 s** (2.3 min) | 2.85× |
| fresh audit — warm cache (repo seen before) | — | **155 s** (2.6 min) | 7.4× vs cold |

Byte-identity: the audit card (`--format json`, **89 findings**, 840 hunks) is
**byte-for-byte identical** across cold-fresh, warm-fresh, and seeded on the
pinned rocksdb clone (`a40466d963a3`); fastapi cross-checked (25.5 s → 4.0 s
cold→warm, identical cards). The findings never move — only the clock does.

### guava sanity

| scenario | now |
|---|--:|
| fresh audit — cold cache | 282 s |
| second audit — warm cache | 51 s (byte-identical) |

## Verdict

The embedding wall is a first-encounter cost that only the (identity-breaking)
batching lever could have dented, so a **cold** fresh audit of a never-seen giant
repo stays embed-bound and improves only modestly (1148 → 1013 s). The
transformative wins are everywhere else: the machine-wide cache collapses every
**repeat** encounter to near-cache-speed — seeded audit **387 → 136 s (2.3 min,
2.85×)**, warm fresh audit **155 s (2.6 min, 7.4× vs cold)**, warm `fit`
**1000 → 164 s (6.1×)** — and the three parallelisations cut the non-embed floor
(placement + integrity + check scoring: 174 s → ~23 s). The seeded audit — the
common path, run after `argot init` — lands at 2.3 min, essentially the PRD's
~2 min target and a 2.85× cut. guava tells the same story (282 s cold → 51 s
warm). All with **byte-identical findings** (89 on rocksdb, unchanged across
every cache state) and the base build untouched.
