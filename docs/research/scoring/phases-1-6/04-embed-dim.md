# Technique 2 — embed_dim (192 → 256)

**Branch**: `research/embed-dim`  
**Status**: complete 2026-04-18  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

A larger embedding dimension gives the encoder and predictor more capacity to
represent style patterns. The current default is 192. Testing 256 — a 33%
increase — may improve AUC at sizes where the model is already learning
something (≥ 20k). No effect expected at small/medium where the model is
random regardless.

## What changed

`train_model` in `train.py` accepts `embed_dim` (default 192). When set, it
uses that value for both `TokenEncoder` and `ArgotPredictor` instead of the
module-level constant.

Flag: `--embed-dim` on `argot-engine corpus benchmark`. Experiment runs with
`--embed-dim 256`. Default unchanged — no production behaviour change.

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds, small/medium/large
only (xlarge dropped — see `03-context-after.md §Decision`).

### Shuffled AUC

| bucket | baseline       | variant        | Δ      |
|:-------|:---------------|:---------------|:-------|
| small  | 0.500 ± 0.000  | 0.500 ± 0.000  | +0.000 |
| medium | 0.501 ± 0.001  | 0.501 ± 0.000  | +0.000 |
| large  | 0.637 ± 0.006  | 0.638 ± 0.007  | +0.001 |

### Cross-repo AUC

| bucket | baseline       | variant        | Δ      |
|:-------|:---------------|:---------------|:-------|
| small  | 0.446 ± 0.028  | 0.453 ± 0.020  | +0.007 |
| medium | 0.395 ± 0.005  | 0.400 ± 0.014  | +0.005 |
| large  | 0.544 ± 0.015  | 0.557 ± 0.028  | +0.013 |

### Injected AUC

| bucket | baseline       | variant        | Δ      |
|:-------|:---------------|:---------------|:-------|
| small  | 0.450 ± 0.020  | 0.458 ± 0.011  | +0.008 |
| medium | 0.398 ± 0.011  | 0.402 ± 0.009  | +0.004 |
| large  | 0.631 ± 0.010  | 0.642 ± 0.023  | +0.011 |

## Interpretation

**No effect at small/medium.** Shuffled AUC stays at ~0.50 — the model is
random at these sizes regardless of capacity. Expected.

**Modest lift at large (+0.013 cross, +0.011 injected).** Both cross-repo and
injected AUC improve at 20k. The lift technically clears the 0.01 stop rule,
but it is marginal: the cross-repo variance grows from ±0.015 to ±0.028,
meaning the delta (+0.013) is within the noise of the variant itself. This is
weak evidence — real but not reliable.

**Shuffled AUC essentially flat (+0.001).** The model's ability to distinguish
its own style from shuffled tokens barely moves. Most of the gain appears in
the harder tasks (cross-repo, injected) where the extra capacity may help the
encoder generalise slightly better.

**Verdict**: marginal pass on the stop rule at large. Worth including in a
final combined run in Phase 4, but not a strong standalone win. The capacity
increase doesn't hurt and adds negligible training time.
