# Semantic layer — F2 placement (misplacement) benchmark

**Date:** 2026-07-08 · **Branch:** `feat/semantic-layer`

F2 "misplacement" ("the right code, wrong package") is language-agnostic (it votes
the areas of a function's nearest cross-file neighbours) and was previously
UNMEASURED beyond exploration. This memo measures both axes honestly across all 31
corpora and records the verdict: **F2 is a low-rate, low-precision ADVISORY channel**
— no placement.rs change was warranted.

## Transplant recall (catch)

`benchmarks/sem_analysis.py` mirrors the exact production placement rule (k-NN area
vote at depth 3, per-area belongs-norm, ≤0.05 in-area ceiling, top-2 concentration
≥0.8, abstain-on-new-dirs) against the real `.argot/semantic-index.json`. For each
sampled function it re-files the function into a random *foreign existing* area and
checks the rule fires `misplaced` — a synthetic transplant, isolated from F1
(which, at check time, would claim an outright duplicate first).

Target ≥40% per corpus. Measured on **29 of 31 corpora spanning all 11 languages**
(the vectorised `sem_analysis.py` now handles the large C# indexes too; the two
gaps are commander and express — single-package repos with no second area to
transplant into). Median **66%**, from ~99% (rich, a two-area flat repo where every
function's home is unambiguous) down. **Three corpora land below 40%** — hono 35%,
jellyfin 36%, outline 36% — small/cohesive repos where sibling modules share so much
shape that a transplanted function's neighbours don't concentrate enough to clear the
0.8 gate. That conservatism is deliberate: the same gate keeps in-place over-fire and
clean-commit false-fire low (below), and loosening it to lift those three re-inflates
the false-fire rate. An honest recall floor, not a bug. Per-language spot values:
Go 50 (gh-cli), C 83 (redis), Java 68 (junit5), Ruby 76 (rubocop), Rust 85 (ripgrep),
C++ 86 (fmt), PHP 56–58 (laravel/composer), C# 36–67 (jellyfin/powershell),
Python 50–99, TS 35–70.

## Clean-commit misplaced FP (false-fire)

`benchmarks/sem_fp.py` replays real newer commits through `argot check --commit` and
counts `misplaced` fires (window 150, leak-free). F2 is unchanged by the F1 fix
(placement.rs untouched), so these are stable.

**Result: ≤ 2%/hunk on 27 of 31 corpora; 0.78%/hunk in aggregate** (201 fires over
25.8 k replayed hunks). The four above 2% are all cohesive multi-module monorepos or
tiny-hunk samples:

| corpus | misplaced/hunk | why |
|---|---|---|
| fmt (C++) | 20% | tiny replay (7 fires / 35 hunks) — noise |
| junit5 (Java) | 5.0% | cross-module similarity across cohesive `junit-platform-*` modules |
| homebrew (Ruby) | 2.9% | sibling formula/cask modules parallel each other |
| powershell (C#) | 2.8% | tiny replay (3 fires / 109 hunks) — noise |

## Labelled precision (3 independent adversarial judges, default false-alarm)

- **excalidraw** (28 fires): **2 genuine, 26 false-alarm** (unanimous) — 1.17% true.
  The 2 genuine are text-handle geometry / hit-testing filed at the package root
  whose canonical home is `packages/element`.
- **wagtail** (7 fires): **0 genuine, 7 false-alarm** — reasonable-but-debatable
  placements (a `VersionNumber` util whose only consumer is a controller, a subclass
  override correctly in its contrib module, a near-dup queryset method).

So F2's *precision* is low — most fires are cross-cutting helpers, feature-co-located
app code, public entry points, subclass overrides, and new modules that legitimately
parallel an existing one. But the *rate* is low (≤2%/hunk), so the absolute nuisance
is small, and the fires point at real architectural neighbours worth a glance.

## Verdict — no placement.rs change

The multi-module false-fires (laravel/junit5) and the hono recall miss are two sides
of the same inherent limit: judging placement from embedding neighbours alone cannot
tell a *legitimately parallel* sibling module from a *misplaced* function. A
principled fix (e.g. abstain when the function's own area holds too few indexed
functions to establish a locality norm) was considered but not shipped: the rate is
already within the ≤2% band on all but cohesive-multi-module corpora, F2 recall is
already conservative, and chasing the residual risks that recall for a sub-2% gain on
an ADVISORY channel. F2 ships as the lower-recall but far quieter of the two advisory
senses (0.78%/hunk vs reinvention's ~5%), honestly labelled. Like `redundant`, a
`misplaced` finding is advisory — never folded into the gated catch/over-fire metric —
but it fires at the mildest (`unusual`) tier and so still contributes to the exit code
(mute or raise `--min-severity` to drop it from the gate). (Full per-corpus recall + FP
table: benchmarks page.)
