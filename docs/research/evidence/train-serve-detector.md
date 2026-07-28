# The voice-not-where-the-work-is signal, validated on real repositories

**Date:** 2026-07-28 · **Status:** positive — fires on the real mis-scope, quiet
where configuration already fixed it, and it found two more the same day.

**Question:** uos learned 64 % of its voice from demo code and produced 141 of
its 145 false alarms in the library that demos merely call. Finding that took a
benchmark outlier and a day of investigation, and **argot never said a word.**
Can it detect its own mis-scoping?

## The measurement

Per top-level directory, compare its share of the **corpus** (what shapes the
voice) with its share of recent **churn** (what gets written, and therefore
reviewed). A directory over 25 % of the corpus taking less than a third of that
in changes is teaching a voice the repository is not judged against. No findings
needed, so it runs at fit and surfaces as a yellow reason in `argot inspect`.

Churn is counted over **exactly the files the corpus holds today** — see below;
getting that wrong is what made the first version report argot as broken.

## Validated against real repositories

| corpus | `argot.toml`? | verdict | signal |
|---|---|---|---|
| **mormot2** | **none** | ready with notes | `ex/` shapes **58 %** of the voice, takes **1 %** of changes (383 files, 7 changed) |
| **castle-engine** | yes | ready with notes | `examples/` shapes **27 %**, takes **3 %** (708 files, 42 changed) |
| uos | yes | **ready** | silent — its config excludes `examples/` |
| mseide-msegui | yes | **ready** | silent |
| argot itself | yes | — | silent: `crates/` is 90,7 % of the voice against 84,2 % of the work |

It fires where the mis-scope is real, and goes quiet the moment configuration
fixes it — uos is the control, since its config was written the same day *from*
the manual investigation this signal is meant to replace.

**It found two more while being validated.** mormot2 carries no `argot.toml` at
all and learns 58 % of its voice from `ex/`; castle-engine's config excludes its
vendored trees but not `examples/`. Neither had been noticed.

## Two bugs the unit tests could not catch

Both surfaced only by running it on a real repository, because the tests fed
synthetic relative paths:

- **`rel_to_repo(path, repo)` takes the path first.** Called the other way round,
  every corpus file bucketed to `"."` and argot reported *itself* 100 %
  mismatched.
- **Churn and corpus must be counted over the same population.** An unrestricted
  walk put 44 % of argot's churn in `benchmarks/` (excluded from the voice) and
  31 % in `engine/` and `cli/` — **directories the Rust port deleted**. A
  400-commit window reaches into layouts that no longer exist, and `crates/`,
  the whole source tree, read as a mismatch at 91 % against 22 %.

## Why the thresholds are what they are

25 % of the corpus before a directory matters at all, and a 3:1 gap before it is
a finding. Three-to-one is deliberately conservative: a large, stable core is
normal, and reporting it would train people to skip the note. castle-engine's
27 %/3 % clears it; a tree at 30 %/15 % does not.

## What it is for

A mis-configured argot fails **silently** — false alarms people learn to scroll
past, or real code never flagged. Neither looks like an error, and setup is one
afternoon while the model has to stay honest for years. This is the first of the
signals meant to close that gap, and the one that would have caught uos on its
own.
