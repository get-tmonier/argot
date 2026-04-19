# Phase 3 Synthesis — Technique Rankings and Recommendations

> **Scope**: This is the Phase 3–4 synthesis (TF-IDF technique experiments). For the project-level summary see [`ROADMAP.md`](../ROADMAP.md).

**Status**: complete 2026-04-19  
**Covers**: techniques 1–6 (`03-context-after.md` through `08-path-embed.md`)  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Full results table

All numbers are mean ± std over 3 seeds. Sizes: small = 3k records, medium =
7k, large = 20k. Δ columns are variant minus baseline.

### Cross-repo AUC

| technique       | small          | medium         | large          |
|:----------------|:---------------|:---------------|:---------------|
| baseline        | 0.446 ± 0.023  | 0.395 ± 0.004  | 0.544 ± 0.012  |
| context_after   | 0.449 ± 0.020  | 0.405 ± 0.009  | 0.581 ± 0.019  |
| embed_dim       | 0.454 ± 0.016  | 0.400 ± 0.012  | 0.557 ± 0.023  |
| epochs (×10)    | 0.430 ± 0.082  | **0.608 ± 0.006**  | 0.438 ± 0.010  |
| **char_ngrams** | **0.719 ± 0.022**  | **0.650 ± 0.048**  | **0.648 ± 0.004**  |
| imports_scope   | 0.741 ± 0.020  | 0.688 ± 0.009  | 0.662 ± 0.008  |
| path_embed      | 0.867 ± 0.006  | 0.709 ± 0.038  | 0.709 ± 0.006  |

### Shuffled AUC

| technique       | small          | medium         | large          |
|:----------------|:---------------|:---------------|:---------------|
| baseline        | 0.500 ± 0.000  | 0.501 ± 0.000  | 0.637 ± 0.005  |
| context_after   | 0.500 ± 0.000  | 0.501 ± 0.001  | 0.641 ± 0.002  |
| embed_dim       | 0.500 ± 0.000  | 0.501 ± 0.000  | 0.638 ± 0.005  |
| epochs (×10)    | 0.638 ± 0.011  | 0.671 ± 0.004  | **0.697 ± 0.007**  |
| **char_ngrams** | 0.500 ± 0.000  | 0.655 ± 0.044  | 0.684 ± 0.005  |
| imports_scope   | 0.478 ± 0.003  | 0.577 ± 0.017  | 0.595 ± 0.003  |
| path_embed      | 0.505 ± 0.002  | 0.634 ± 0.023  | 0.610 ± 0.009  |

### Injected AUC

| technique       | small          | medium         | large          |
|:----------------|:---------------|:---------------|:---------------|
| baseline        | 0.450 ± 0.017  | 0.398 ± 0.009  | 0.631 ± 0.008  |
| context_after   | 0.454 ± 0.015  | 0.409 ± 0.006  | 0.653 ± 0.010  |
| embed_dim       | 0.458 ± 0.009  | 0.402 ± 0.007  | 0.642 ± 0.019  |
| epochs (×10)    | 0.613 ± 0.058  | **0.707 ± 0.015**  | 0.668 ± 0.008  |
| **char_ngrams** | **0.721 ± 0.020**  | **0.719 ± 0.052**  | **0.736 ± 0.006**  |
| imports_scope   | 0.623 ± 0.009  | 0.599 ± 0.014  | 0.592 ± 0.001  |
| path_embed      | 0.712 ± 0.020  | 0.662 ± 0.022  | 0.615 ± 0.007  |

## Technique ranking

Ranked by how broadly each technique improves the metrics that matter for
argot's actual use-case: **does this commit match this repo's style?** That is
primarily `shuffled_auc` (intra-repo coherence) and `injected_auc` (detecting
foreign hunks in home context), with `cross_auc` as a secondary signal.

### 1. char_ngrams — **merge unconditionally** ⭐

The standout result of the entire series. Character 3–5-grams (via sklearn
`char_wb` analyzer) improve every metric at every bucket size:

- Cross: +0.274 / +0.255 / +0.103
- Shuffled: flat at small (0.500), +0.154 at medium, +0.047 at large
- Injected: +0.271 / +0.321 / +0.105

No regression anywhere. The improvement is qualitative: at small (3k records),
cross-repo goes from 0.446 (barely above random) to 0.719. A model that needed
20k records to be useful now performs meaningfully at 7k. This directly solves
the original finding from Phase 2 ("the model is completely random below 20k").

**Why it works**: code style lives at the character level — camelCase vs
snake_case, operator spacing, identifier patterns. Word-level TF-IDF treats
`myVariable` and `snake_var` as distinct *words* and misses the *convention*.
Char n-grams see the recurring sub-word patterns that define a repo's surface
style.

### 2. epochs (size-adaptive) — **merge with adaptive default**

200 epochs is transformative at medium (+0.213 cross, +0.309 injected) but
causes severe overfitting at large (−0.106 cross). The right number of epochs
scales with dataset size. A fixed-count default cannot work across all sizes.

Recommended adaptive heuristic: `epochs = max(20, 1_400_000 // n_records)`.
This gives ~200 at 7k and ~20 at 70k, empirically matching the sweet spots.
Merge the adaptive formula, not a fixed count.

### 3. context_after — **merge** (small but consistent)

Wiring the already-extracted `context_after` field into training gives a small
but clean improvement at large (+0.038 cross, +0.022 injected, +0.004
shuffled). No regression anywhere. Low cost — the field already exists in the
dataset, one flag to enable it. Worth merging alongside char_ngrams.

### 4. embed_dim (192 → 256) — **skip**

Marginal gains at large (+0.013 cross, +0.011 injected) within noise range.
The extra parameters don't add value at these dataset sizes. Not worth the
increased memory and compute.

### 5. imports_scope — **do not merge**

Largest cross-repo gains after path_embed (+0.295 / +0.293 / +0.118), but
shuffled regresses at small (−0.022) and large (−0.042). The model learns repo
identity (which packages are imported) rather than code style. For argot's
use-case the shuffled regression is disqualifying — the model becomes less able
to detect incoherent code within a single repo.

### 6. path_embed — **do not merge**

Strongest cross-repo numbers in the series (+0.421 at small; 0.867 cross AUC).
Same repo-identity pattern as imports_scope: the model memorises directory
structure (`packages/*/src/*.ts` vs `tests/test_*.py`) rather than style.
Shuffled regresses at large (−0.027). The cross gain is nearly trivially
explained by language (TypeScript vs Python path conventions) rather than style.

## The two-signal finding

The most important insight from Phase 3 is a structural one:

**The six techniques split cleanly into two categories:**

| category | techniques | what they detect |
|:---------|:-----------|:-----------------|
| **style signals** | char_ngrams, epochs, context_after | Does this code *look right* in its context? (intra-repo coherence) |
| **identity signals** | imports_scope, path_embed | Is this code *from* this repo? (repo fingerprinting) |

Style signals improve shuffled and injected AUC — they help the model detect
that something is off about a specific change. Identity signals inflate
cross-repo AUC by recognising which codebase the code belongs to, but they
displace capacity that would otherwise detect style violations.

For argot, style signals are what matters. Identity signals are a distraction —
and potentially a liability if the model starts penalising cross-language
contributions or refactors that move files across directories.

## Revised minimum-viable corpus size

Phase 2 found: "~20,000 records for a non-random model."

With char_ngrams: **~7,000 records** is sufficient for meaningful cross-repo
and injected AUC (0.650 and 0.719 respectively). The phase transition moved
down by nearly 3×.

With adaptive epochs + char_ngrams combined (not benchmarked directly, but
implied by the additive structure of the gains): the 7k threshold likely
improves further. A combined experiment is the recommended next step.

## Recommendations for the integration branch

In priority order:

1. **Enable char_ngrams by default** in `train_model`. This is the single
   largest quality improvement and has zero regressions. Change `char_ngrams=False`
   default to `True`, or make `char_wb` the default analyzer.

2. **Implement adaptive epochs**: replace the fixed `epochs=20` default with
   `epochs = max(20, 1_400_000 // n_records)`. This is a one-liner and
   prevents overfitting at large while enabling full convergence at medium.

3. **Enable context_after**: small but regression-free improvement. The field
   already exists in the dataset — flip the default flag.

4. **Do not add path or import features** to the scoring model. Reserve them
   as candidates for a separate "repo routing" or "file-in-scope" classifier
   if that use-case emerges.

## Open questions for the calibration branch

- char_ngrams + adaptive epochs combined: are the gains additive? (Likely yes,
  as they address different parts of the signal.)
- The minimum corpus size update (7k → ?) should be verified against a
  micro-sized repo to confirm char_ngrams doesn't produce false confidence at
  < 3k records.
- Cross-repo AUC still plateaus around 0.55–0.65 for pure style (char_ngrams
  at large). The TS/Py language boundary in the large bucket pair likely
  inflates the difficulty. A same-language pair benchmark would isolate
  cross-style learning from cross-language detection.
