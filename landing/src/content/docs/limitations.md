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

The honest picture, across **27 repos in 8 languages**. argot's gated job — catching a **novel
pattern** the repo has never used (a foreign import, a foreign API, a foreign concurrency library) —
lands **48 of 49 = 98%**, and all 8 language corpora clear the ≥ 85% bar. On **edits to existing
files** the false-alarm rate is **1.05% aggregate**, and **24 of 27 repos** sit under the ≤ 2% gate.
The worst residuals we publish rather than hide: **ink 8.7%** (a repo first adopting Node built-ins like
`setImmediate`/`parseInt` it hadn't used at fit), **bat 7.4%** (a genuine `git2` → `gix` dependency
migration plus Rust std), and **fastapi 2.2%** (`annotated_doc`/`pwdlib` adopted across separate
commits) — each the *first use* of a dependency the repo had never touched, which is exactly what
argot is built to flag. Several fixes in the #92 pass drove the FP down: a **Python relative-import
error-recovery guard** (a mid-fragment `from ._compat import v2` no longer leaks the imported symbol
as a phantom foreign module), a **per-changeset novel-import dedup** (one commit adding the same new
dependency across many files
alerts once, not once per file), a **repo-declared-symbol snapshot** (a bare call or `Type::method`
to a function/class/type the repo itself declares is internal cross-file code, not foreign — this
alone took rocksdb from 4.0% to 1.5%), and a **hunk-level foreign-reach gate** (one foreign callee no
longer flags every hunk in the file).

The harder classes — naming shape, and semantic/API misuse *within* a library the repo already
uses — are **secondary coverage**: argot reports them but never gates on them, and it admits it does
not catch them reliably. That's a **proven limit**, not a tuning gap — see the modeling caveats
below. The dependable value today is the novel-pattern class: foreign imports and strongly foreign
API surfaces.

## Modeling caveats

- **Needs enough source to calibrate.** The sampler looks for top-level functions/classes with ≥ 5
  body lines. Repos with fewer than ~100 sampleable units may get a noisier threshold.
- **Best on a consistent hand.** Highly polyglot repos, or repos with many contributors and no
  enforced style, are harder to model.
- **Subtle structural breaks on heterogeneous application code.** Callback pyramids and
  framework-idiom shapes carry little import/callee signal when the host repo's own code
  legitimately reaches the same shapes (three candidate mechanisms for this class were scouted and
  refuted with documented evidence).
- **In-vocabulary breaks often score in the same range as legitimate new code.** The scorer is a
  token-rarity model; an honest threshold that keeps false positives low also lets many
  wrong-error-discipline / naming-shape breaks through — the **secondary classes** argot reports
  but never gates on. This is a **proven limit**, not a tuning gap. A pretrained-code-embedding
  manifold-outlier and per-token MLM surprise were both scouted and both plateau at ~0.65 AUC once
  fairly controlled; decisively, a minimal-pair test (a wrong-error-discipline break vs its own
  idiomatic twin, only the error mechanism swapped) leaves the pretrained code embedding at
  **cosine 0.996** — the break is invisible. Documented in the
  [Phase B evidence](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-phaseB-recall-limit.md).
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
| Push FP ≤ 2% (existing files) and grow secondary-class coverage | Under the leak-free protocol: 1.05% aggregate existing-file FP, 24 of 27 corpora under the gate; gated novel-pattern catch 48/49 = 98% — honest tables in the [#92 evidence](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-honest-rebench.md) |
| Validate on application corpora | ✅ Done — four application corpora benchmarked and published |
| Suppression mechanism | ✅ Shipped |
| Repo suitability check | ✅ Shipped (`argot inspect`) |
| Official CI integration | ✅ Shipped (Action + pre-commit + SARIF) |
| This documentation site | ✅ Live |

Already shipped since the early roadmap: **per-language calibration** for mixed monorepos, and a
**per-hunk evidence line** that names the tokens carrying each score.

Browse everything, including non-v1 work, at the
[issue tracker](https://github.com/get-tmonier/argot/issues).
