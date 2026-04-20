# FastAPI Stage 3 sweep — corpus pre-filtering (similarity to break fixtures)

**Date:** 2026-04-20  
**Base config:** JepaCustomScorer(epochs=20, lr=1e-4, flat schedule, depth=4, mlp_dim=1024) — Stage 2 winner  
**Seeds:** {0, 1, 2}  
**Gate:** mean_delta ≥ 0.20 AND std_delta ≤ 0.02  
**Grid:** τ ∈ {top-1%, top-5%} corpus records dropped by max cosine similarity to break fixtures  

## Raw

| config | seed | delta | gate |
|---|---|---|---|
| filtered_tau1 | 0 | 0.1828 | ✗ |
| filtered_tau1 | 1 | 0.2169 | ✓ |
| filtered_tau1 | 2 | 0.2052 | ✓ |
| filtered_tau5 | 0 | 0.1658 | ✗ |
| filtered_tau5 | 1 | 0.1774 | ✗ |
| filtered_tau5 | 2 | 0.1805 | ✗ |

## Summary

| config | mean_delta | std_delta | gate |
|---|---|---|---|
| filtered_tau1 | 0.2016 | 0.0173 | ✓ |
| filtered_tau5 | 0.1746 | 0.0078 | ✗ |

## Verdict

**Stage 2 winner (`flat_d4m1024`) remains best.** `filtered_tau1` technically clears both gates (mean=0.202, std=0.017) but is strictly inferior to `flat_d4m1024` on both metrics (0.203 mean, 0.010 std). `filtered_tau5` regresses below the gate.

## Analysis

**Filtering hypothesis is backwards.** The intuition was: remove corpus records that look like break fixtures so the predictor isn't trained to expect them. But break fixtures are never used during training — the predictor only learns from corpus records. Filtering break-similar corpus records removes *legitimate FastAPI code* that happens to share surface patterns with the test fixtures, shrinking useful training signal rather than removing noise.

**Regression scales with τ.** tau=1% (−20 records) produces a modest regression; tau=5% (−100 records) produces a larger one (mean 0.175 vs 0.203). This confirms that every filtered record was contributing positively to the predictor's understanding of normal FastAPI code.

**`filtered_tau1` gate pass is noise, not signal.** Its mean (0.202) is within one std of `flat_d4m1024` (0.203), and its higher std (0.017 vs 0.010) reflects the reduced training set introducing more variance. Not a meaningful improvement.

## Future directions

**Diversity-based corpus filtering** is a more principled alternative: instead of filtering by similarity to break fixtures, keep corpus records that maximally cover the embedding space (e.g. greedy farthest-point selection or k-means representative sampling). This directly attacks the "small corpus = undertrained predictor" problem — ensuring training data is maximally informative per record rather than redundant. Worth exploring in a future phase if the corpus size cannot be increased.

## Next step

Stage 4: inference ensemble on `flat_d6m1024` (mean=0.221, std=0.024 unensembled). Averaging N predictor runs should reduce std below 0.02 while preserving the higher mean — potentially yielding a stronger final config than `flat_d4m1024`.
