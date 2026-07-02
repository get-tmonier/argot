# Application-corpora validation (issue #66)

All previously validated corpora were libraries/frameworks — codebases that
impose conventions on themselves. Real users mostly maintain applications:
heterogeneous voice, more boilerplate, framework-driven code. Issue #66 asks
whether the headline numbers generalize, published honestly either way.

## Corpora

Four application corpora, pinned in `benchmarks/targets.yaml` (primary HEAD +
two history snapshots ~300/~800 commits back as control populations):

| Corpus | Language | Type |
|:---|:---|:---|
| saleor | Python | Django + GraphQL e-commerce platform |
| wagtail | Python | Django CMS (with a TS admin frontend) |
| excalidraw | TypeScript | React + canvas drawing app |
| outline | TypeScript | React + MobX / Koa + Sequelize knowledge base |

Sentry, Cal.com and n8n were considered and rejected for bench-compute
honesty (monorepo-scale corpora would multiply bench cost without adding
signal beyond these four). Catalogs are hand-crafted (14 fixtures × 7
categories per corpus), every category grep-verified foreign to the corpus's
own production code, host-injection metadata verified against the pinned
checkout.

## Harness lesson: polyglot applications

The first wagtail run reported **17.7% FP** — almost entirely
`client/src/**/*.ts` hunks scored by the Python-calibrated scorer. Wagtail is
a polyglot repo (Python package + TypeScript admin client); the bench's
single-language path scored every dataset record regardless of language,
which production `check` never does (it routes files to their language's
scorer). After language-filtering controls (commit `326f59a1`), wagtail's FP
is **0.34%**. Application repos are polyglot by default; the library corpora
never surfaced this because they are language-pure. This is also direct
evidence for the #41 multi-language work: a Python-only model on a polyglot
app silently says nothing about half the repo.

## Results (ship configuration)

| Corpus | Recall | FP rate | AUC | Threshold | Uncaught |
|:---|---:|---:|---:|---:|:---|
| saleor | 12/14 (85.7%) | 0.24% (31/12777) | 0.993 | 5.44 | raw_sql_2, print_debug_1 |
| wagtail | 14/14 (100%) | 0.34% (41/12155) | 0.999 | 4.67 | — |
| excalidraw | 9/14 (64.3%) | 0.43% (84/19408) | 0.957 | 5.76 | legacy_lifecycle_1/2, redux_store_2, callback_pyramid_2, vue_idioms_2 |
| outline | 10/14 (71.4%) | 0.46% (113/24482) | 0.879 | 5.00 | jquery_1, foreign_http_2, class_components_1/2 |

Application total: **45/56 (80.4%)**; every corpus FP ≤ 0.5%.

Success bar (from the issue): recall ≥ 85% and FP ≤ 2% per corpus.

- **Python applications pass.** saleor 12/14 at 0.26% FP, wagtail 14/14 at
  0.34% FP. FP on applications is actually *better* than on the library set
  (libraries: 0.4–2.1%) — application histories give larger, more diverse
  control populations, and the voice heterogeneity that was expected to
  inflate FP instead inflates the calibration threshold.
- **TypeScript applications miss the recall bar.** excalidraw 9/14 (64%),
  outline 10/14 (71%), FP ≤ 0.5% both. Every miss is a BPE-below-threshold
  case with import score 0: application thresholds are high (5.0–5.8 vs
  3.8–5.3 on libraries) because heterogeneous app code produces high
  calibration maxima, and subtle structural breaks (legacy React lifecycle,
  Redux boilerplate, callback pyramids, class components that genuinely
  still exist in the corpus) score 1.5–4.3 — far below. These are the same
  "negative-shape" residuals era 14 failed to reach on libraries
  (`era14-final.md` phase C), now with 8 more instances.

## Failure modes (tracked for future eras)

1. **Threshold inflation on heterogeneous corpora** — the max/K-seed-median
   threshold tracks the most surprising calibration hunk; applications have
   more of them. Same root as era-14 phase B's refutation.
2. **Structural (callee-free) breaks** — legacy lifecycle methods, Redux
   store shapes, and callback pyramids carry almost no callee/import signal;
   they need a shape mechanism, and all three era-14 phase C maths failed on
   exactly this shape.
3. **Polyglot-by-default** — handled at the harness level here; production
   handling is #41.

## Verdict for the README claims

Numbers are defensible with categorization: recall 88–100% / FP ≤ 2.1% on
libraries; recall 64–100% / FP ≤ 0.5% on applications, with the recall
spread explained by the structural-break residuals above. The combined
"voice breaks get flagged, FP stays rare" claim holds on applications; the
uniform-recall claim does not, and the README table now says so explicitly.
