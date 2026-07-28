# What `argot check` actually costs, and the ignored tree that hides it

**Date:** 2026-07-29 · **Status:** the published 200 ms claim is corroborated —
and the investigation found a real startup cost nobody had measured.

**Question:** the hero says *"checks a diff in 200 ms"*. No evidence document
backed it, and the claim manifest records `performance.audit_timing` as
**unavailable — no performance claim approved**. Is the number true?

## It is true, on a normal working tree

Best-of-20 wall clock, release binary, `ARGOT_OFFLINE=1`, on a clean local clone
of this repository at the same HEAD with the same fit artifacts:

| changeset | files | latency |
|---|--:|--:|
| `HEAD~1..HEAD` | 2 | **148 ms** |
| `HEAD~4..HEAD` | 8 | **170 ms** |
| `HEAD~20..HEAD` | 19 | **173 ms** |
| `HEAD~60..HEAD` | 194 | **764 ms** |

A pull-request-sized changeset lands comfortably under 200 ms, and a 194-file
changeset — far past what anyone reviews at once — still finishes in under a
second. The machine carried a load average of ~4 throughout, so every figure is
an **upper bound**.

Note the shape: 2 files to 19 files costs +25 ms, while 19 to 194 costs +591 ms.
Startup dominates a normal diff; only an unusually large one becomes
throughput-bound.

## The finding: check walks directories git ignores

The same commands in the **development** working copy of the same repository:

| working copy | entries on disk | `argot check`, nothing to score |
|---|--:|--:|
| clean clone | 3 223 | **231 ms** |
| dev copy | 1 026 989 | **2 710 ms** |

A 12× slowdown, on the same repository, at the same commit, with the same
artifacts. The difference is `target/`: **854 895 entries** of build output
against 2 565 tracked files.

It is not contention — `/usr/bin/time` reports **2,06 s user + 0,53 s sys**, so
the process genuinely burns the CPU. A sampling profile is dominated by
`__opendir2` and allocator traffic, which is what walking a million directory
entries looks like.

`target/` is in `.gitignore`. Check pays to discover that on every invocation.

**Who this hits:** every Rust repository with a warm `target/`, every JavaScript
one with `node_modules`, every Python one with a `.venv` — that is to say, most
working copies, all day, in the interactive loop the 200 ms claim is selling.
The benchmark never saw it because harness clones are freshly checked out and
have no build output.

Filed as its own task; the fix is to source the walk from git's index (or honour
ignore rules before descending) rather than to change any published number.

## Two corrections worth recording

- **Measure the right thing.** The first pass timed `argot check --range …`.
  There is no `--range` flag; the reference is positional. All four "results"
  were the argument parser rejecting the command in ~27 ms. A timing harness
  that never checks the exit code measures process startup and reports it as
  product latency.
- **A development machine is not a representative one.** The first honest
  numbers — 2,4 to 3,8 s — were real, reproducible, and led to the conclusion
  that the published claim was wrong by more than 10×. It was not. The
  measurement was taken in the one working copy on the machine carrying a
  million build artefacts. Always reproduce a performance surprise on a clean
  checkout before believing what it says about the product.
