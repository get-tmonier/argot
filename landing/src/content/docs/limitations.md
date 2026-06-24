---
title: Limitations
description: argot is alpha. Here's what's honest about where it works, where it doesn't, and the v1 roadmap.
group: Reference
order: 8
---

argot is **alpha** software. It ships honest benchmarks and a public research log, but real gaps
remain — both in the model and in the surfaces around it. The
[GitHub issue tracker](https://github.com/get-tmonier/argot/issues) is the source of truth.

## Where it works today

argot's benchmark harness runs the production scorer against six pinned open-source repos — fastapi,
rich, faker (Python) and hono, ink, faker-js (TypeScript) — using a hand-crafted catalog of
paradigm-break fixtures scored against **hundreds of thousands of real PR hunks** as negative
controls. Recent results: 108 of 115 fixtures caught, with a false-positive rate ≤ 2.0% on all six
corpora, and a reproducible threshold (CV = 0% across seeds).

## Modeling caveats

- **Needs enough source to calibrate.** The sampler looks for top-level functions/classes with ≥ 5
  body lines. Repos with fewer than ~100 sampleable units may get a noisier threshold.
- **Best on a consistent hand.** Highly polyglot repos, or repos with many contributors and no
  enforced style, are harder to model.
- **Validation corpus is library-only.** All six benchmarked repos are libraries/frameworks.
  Application code may behave differently; the numbers aren't proven there yet.
- **Noisier on very small or brand-new hunks** — less context to score against.

## Surface gaps

These are the adoption-blockers we're building toward v1:

- **No suppression mechanism yet** — no `.argotignore`, inline comments, or `argot mute`.
- **No editor integration** — CLI-only today; no LSP server or extension.
- **No official CI package** — no published GitHub Action, pre-commit hook, or SARIF output.
- **No suitability check** — running `fit` then `check` is the only way to learn whether argot suits
  your repo.

## What v1 needs

| Goal | Why it blocks v1 |
|---|---|
| Push FP ≤ 1% and close the recall gap | Trust at the gate |
| Validate on application corpora | Prove it beyond libraries |
| Suppression mechanism | One stubborn FP shouldn't be un-silenceable |
| Repo suitability check | Tell users up front if it'll work |
| Official CI integration | Action + pre-commit + SARIF |
| This documentation site | Tutorials, how-tos, reference |

Already shipped since the early roadmap: **per-language calibration** for mixed monorepos, and a
**per-hunk evidence line** that names the tokens carrying each score.

Browse everything, including non-v1 work, at the
[issue tracker](https://github.com/get-tmonier/argot/issues).
