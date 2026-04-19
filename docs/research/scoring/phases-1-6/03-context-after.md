# Technique 1 — context_after

**Branch**: `research/context-after`  
**Status**: complete 2026-04-18  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

The extractor already captures the tokens that appear _after_ the changed hunk
(`context_after`). Wiring this field into the TF-IDF context representation
gives the model a fuller picture of the surrounding code — a hunk in the middle
of a function differs from a hunk at the end. This should lift AUC, especially
at sizes where the model is already learning something (≥ 20k).

## What changed

`_load_records` in `corpus.py` now optionally retains `context_after` tokens
(text only, same slimming as `context_before`).

`train_model` in `train.py` accepts `use_context_after=True`; when set,
`context_after` token text is appended to `context_before` text before the
TF-IDF vectorizer sees it. `hunk_tokens` encoding is unchanged.

Flag: `--context-after` on `argot-engine corpus benchmark` (default off; no
behaviour change to the production path).

## Results

Variant uses 3 seeds for small/medium/large; xlarge killed after seed 0 (see
§Decision below) and is a single-seed estimate. Baseline is the Phase 2 mean ±
std (3 seeds) from `02-sizing-study.md`.

### Shuffled AUC

| bucket  | baseline       | variant        | Δ      |
|:--------|:---------------|:---------------|:-------|
| small   | 0.500 ± 0.000  | 0.500 ± 0.000  | +0.000 |
| medium  | 0.501 ± 0.001  | 0.501 ± 0.001  | +0.000 |
| large   | 0.637 ± 0.006  | 0.641 ± 0.002  | +0.004 |
| xlarge* | 0.707 ± 0.002  | 0.694          | −0.013 |

### Cross-repo AUC

| bucket  | baseline       | variant        | Δ      |
|:--------|:---------------|:---------------|:-------|
| small   | 0.446 ± 0.028  | 0.449 ± 0.022  | +0.003 |
| medium  | 0.395 ± 0.005  | 0.405 ± 0.010  | +0.010 |
| large   | 0.544 ± 0.015  | 0.582 ± 0.020  | **+0.038** |
| xlarge* | 0.549 ± 0.017  | 0.576          | +0.027 |

### Injected AUC

| bucket  | baseline       | variant        | Δ      |
|:--------|:---------------|:---------------|:-------|
| small   | 0.450 ± 0.020  | 0.454 ± 0.007  | +0.004 |
| medium  | 0.398 ± 0.011  | 0.409 ± 0.007  | +0.011 |
| large   | 0.631 ± 0.010  | 0.653 ± 0.010  | +0.022 |
| xlarge* | 0.695 ± 0.009  | 0.695          | +0.000 |

*single-seed estimate; not statistically reliable.

## Interpretation

**No effect at small/medium.** At 3k and 7k the model is still random
(shuffled ≈ 0.50), same as baseline. Adding more context tokens doesn't help
when the model hasn't learned anything yet.

**Meaningful cross-repo lift at large (+0.038).** At 20k, cross-repo AUC
climbs from 0.544 to 0.582 — the largest single-metric gain so far.
Shuffled (+0.004) and injected (+0.022) also improve modestly. The model
benefits from seeing more surrounding context when it has enough training data
to extract a style signal.

**Technique clears the stop-rule at large.** Cross-repo Δ = +0.038 > 0.01.
Code merits integration if the gains hold at xlarge; we can't confirm that
without a full xlarge run.

**Xlarge (single seed) is inconclusive.** Shuffled drops −0.013 (likely noise
with n=1), cross and injected are roughly flat. The xlarge run was killed at
seed 1 due to 22 GB peak RSS — see decision below.

## Decision: drop xlarge from technique experiments

Loading 60k records × reservoir-sample headroom causes peak RSS of ~22 GB in
the current architecture. That is impractical to run locally for every
technique experiment. The sizing study already showed the 3k/7k buckets are
effectively random regardless of technique, so the informative range is
**large (20k)** — where signal exists and the technique has visible effect.

**Going forward, all Phase 3 technique experiments run on small, medium, and
large only (3 seeds each).** Xlarge benchmarking is deferred to Phase 4
synthesis, where a single consolidated run can be justified by the highest-lift
techniques surviving Phase 3.

ROADMAP.md and DESIGN.md updated to reflect this.
