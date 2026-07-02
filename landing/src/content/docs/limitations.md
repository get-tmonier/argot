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

argot's benchmark harness runs against ten pinned open-source repos — six libraries (fastapi,
rich, faker · hono, ink, faker-js) and four applications (Saleor, Wagtail · Excalidraw, Outline).
The headline numbers come from the **production path**: every break fixture
is planted into its host file on disk, staged with real git, and judged by the actual
`argot fit` → `argot check --staged` pipeline, with each corpus's last 30 real commits replayed
as the false-positive control. Recent results: libraries **111 of 115** fixtures caught (five
corpora at 100%, false positives 0% on five of six); applications **49 of 56** caught (Saleor and
Wagtail at 100%). The remaining gaps are documented with refutation evidence in the
[research log](https://github.com/get-tmonier/argot/tree/main/docs/research).

## Modeling caveats

- **Needs enough source to calibrate.** The sampler looks for top-level functions/classes with ≥ 5
  body lines. Repos with fewer than ~100 sampleable units may get a noisier threshold.
- **Best on a consistent hand.** Highly polyglot repos, or repos with many contributors and no
  enforced style, are harder to model.
- **Subtle structural breaks on heterogeneous application code.** Callback pyramids and
  framework-idiom shapes carry little import/callee/convention signal when the host repo's own
  code legitimately reaches the same shapes (excalidraw 9/14; three candidate mechanisms for this
  class were scouted and refuted with documented evidence).
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
| Push FP ≤ 1% and close the recall gap | Production-path recall 160/171 (93.6%) with FP ≈ 0 on 7 of 10 corpora; residuals documented with refutation evidence |
| Validate on application corpora | ✅ Done — four application corpora benchmarked and published |
| Suppression mechanism | ✅ Shipped |
| Repo suitability check | ✅ Shipped (`argot inspect`) |
| Official CI integration | ✅ Shipped (Action + pre-commit + SARIF) |
| This documentation site | ✅ Live |

Already shipped since the early roadmap: **per-language calibration** for mixed monorepos, and a
**per-hunk evidence line** that names the tokens carrying each score.

Browse everything, including non-v1 work, at the
[issue tracker](https://github.com/get-tmonier/argot/issues).
