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

**In July 2026 we found and fixed a measurement flaw** ([#92](https://github.com/get-tmonier/argot/issues/92)):
the benchmark's false-positive control replayed commits the model had trained on, and the
threshold calibration had the same leak one layer down — so published FP numbers were near zero
by construction. Everything below is from the corrected, leak-free protocol: FP from a
**temporal holdout** (fit at an old commit, replay only commits the model never saw, split by
whether the file existed at fit time), recall from curated break fixtures spliced into real host
files and judged by the actual `argot fit` → `argot check --staged` pipeline. Full tables with
bootstrap confidence intervals live in the
[re-measurement evidence](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-honest-rebench.md).

The honest picture, in one paragraph: on **edits to existing files**, 10 of 24 benchmarked
corpora meet our ≤ 2% false-positive gate and most of the rest sit between 2% and 7%, with a few
genuinely red (bat 11.5%, jellyfin 9.7%, rubocop 7.0%, fastapi 6.6%, rocksdb 6.2%). On **new files**,
what used to flood (excalidraw 21%, redis 61%, fmt 57%) is now largely fixed by a separate
**new-file threshold** calibrated by scoring each fit file as if newly added — 8 corpora crossed
back under the ≤ 5% gate with zero regression on existing-file FP or recall
([#92 Phase A](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-phaseA-diagnosis.md)).
The new-file red that remains is import-dominated (rocksdb 40% — Python tooling in a C++ repo;
redis 32% — vendored third-party C; fmt 20% and laravel 11.5% — new-feature files that add a
dependency), where a foreign-import tripwire cannot tell a new dependency from a break. Recall on
the mature Python corpora is strong (fastapi/faker/saleor/wagtail 100%, rich 69%); on the *hard*
curated break classes in the other languages — wrong error discipline, wrong concurrency, API
misuse within libraries the repo already uses, naming shape — it ranges **21–62%**, a **proven
limit**: an embedding manifold-outlier and per-token MLM surprise were both attempted and both
plateau at ~0.65 AUC once fairly controlled. The dependable value today is the tripwire class:
foreign imports and strongly foreign API surfaces.

## Modeling caveats

- **Needs enough source to calibrate.** The sampler looks for top-level functions/classes with ≥ 5
  body lines. Repos with fewer than ~100 sampleable units may get a noisier threshold.
- **Best on a consistent hand.** Highly polyglot repos, or repos with many contributors and no
  enforced style, are harder to model.
- **Subtle structural breaks on heterogeneous application code.** Callback pyramids and
  framework-idiom shapes carry little import/callee/convention signal when the host repo's own
  code legitimately reaches the same shapes (excalidraw 9/14; three candidate mechanisms for this
  class were scouted and refuted with documented evidence).
- **In-vocabulary breaks often score in the same range as legitimate new code.** The scorer is a
  token-rarity model; an honest threshold that keeps false positives low also lets many
  wrong-error-discipline / naming-shape breaks through (the 21–62% hard-class recall above). This
  is a **proven limit**, not a tuning gap: a pretrained-code-embedding manifold-outlier and
  per-token MLM surprise were both scouted and both plateau at ~0.65 AUC once fairly controlled
  (below the 0.85 bar) — a hunk-level scorer cannot resolve a one-token semantic deviation buried
  in otherwise-idiomatic code. Documented in the
  [Phase B evidence](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-phaseB-pertoken-mlm.md).
- **Voice-novel commits flag proportionally.** New feature areas score as new voice until the
  next `argot fit`; a stale model amplifies this, so `check` warns when the fit is ≥ 10 commits
  old (refits take seconds on the model artifact).
- **Noisier on very small or brand-new hunks** — less context to score against.

## Surface gaps

These are the adoption-blockers we're building toward v1:

- ~~No suppression mechanism~~ — **shipped**: `.argotignore`, `argot: ignore-next-line` comments,
  `.argot/suppressions.yaml`, and `argot mute` / `list-mutes` / `review-mutes`.
- **No editor integration** — CLI-only today; no LSP server or extension.
- ~~No official CI package~~ — **shipped**: composite GitHub Action, pre-commit hook, and
  `argot check --format sarif|json`.
- ~~No suitability check~~ — **shipped**: `argot inspect` reports corpus composition, calibration
  health, and a suitability verdict before you commit to a fit.

## What v1 needs

| Goal | Status |
|---|---|
| Push FP ≤ 2% (existing files) and close the recall gap | Under the leak-free protocol: 10 of 24 corpora meet the FP gate; hard-class recall 21–100% by language — honest tables in the [#92 evidence](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-honest-rebench.md) |
| Validate on application corpora | ✅ Done — four application corpora benchmarked and published |
| Suppression mechanism | ✅ Shipped |
| Repo suitability check | ✅ Shipped (`argot inspect`) |
| Official CI integration | ✅ Shipped (Action + pre-commit + SARIF) |
| This documentation site | ✅ Live |

Already shipped since the early roadmap: **per-language calibration** for mixed monorepos, and a
**per-hunk evidence line** that names the tokens carrying each score.

Browse everything, including non-v1 work, at the
[issue tracker](https://github.com/get-tmonier/argot/issues).
