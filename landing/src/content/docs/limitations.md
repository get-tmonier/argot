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

argot's benchmark harness runs the production scorer against ten pinned open-source repos — six
libraries (fastapi, rich, faker · hono, ink, faker-js) and four applications (Saleor, Wagtail ·
Excalidraw, Outline) — using a hand-crafted catalog of paradigm-break fixtures scored against
**hundreds of thousands of real PR hunks** as negative controls. Recent results: libraries 108 of
115 fixtures caught with FP ≤ 2.0% on all six; applications 45 of 56 caught with FP ≤ 0.5% on all
four (Python apps clear the library bar; TypeScript apps miss subtle structural breaks — see the
research log's application-corpora evidence).

## Modeling caveats

- **Needs enough source to calibrate.** The sampler looks for top-level functions/classes with ≥ 5
  body lines. Repos with fewer than ~100 sampleable units may get a noisier threshold.
- **Best on a consistent hand.** Highly polyglot repos, or repos with many contributors and no
  enforced style, are harder to model.
- **Subtle structural breaks on application code.** Legacy lifecycle methods, Redux-style store
  shapes and callback pyramids carry little import/callee signal and can score under the threshold
  on heterogeneous application corpora (excalidraw 9/14, outline 10/14).
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
| Push FP ≤ 1% and close the recall gap | Research era 14 concluded: four mechanisms refuted with documented evidence; FP holds ≤ 2.0% (libraries) / ≤ 0.5% (applications) |
| Validate on application corpora | ✅ Done — four application corpora benchmarked and published |
| Suppression mechanism | ✅ Shipped |
| Repo suitability check | ✅ Shipped (`argot inspect`) |
| Official CI integration | ✅ Shipped (Action + pre-commit + SARIF) |
| This documentation site | ✅ Live |

Already shipped since the early roadmap: **per-language calibration** for mixed monorepos, and a
**per-hunk evidence line** that names the tokens carrying each score.

Browse everything, including non-v1 work, at the
[issue tracker](https://github.com/get-tmonier/argot/issues).
