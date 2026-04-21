# Phase 13 Experiment 14 — Ensemble (BPE-tfidf + AST-contrastive): Faker Break Scoring (2026-04-21)

**Scorer:** `max(bpe_score, ast_score)` — union ensemble, no parameter tuning
**BPE axis:** `_score_hunk` with `_EPSILON=1e-7`, `aggregation=max`, UnixCoder BPE tokenizer
**AST axis:** `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max', depth=3)`
**model_A (both axes):** 722 faker source files (faker/sources/model_a/)
**model_B (BPE):** generic_tokens_bpe.json (CPython 3.13 stdlib)
**model_B (AST):** generic_treelets.json (CPython 3.13 stdlib, 4.7M treelets)
**Goal:** Determine whether taking the pointwise max of both axes achieves clean separation
on all 5 faker paradigm-breaks.

FastAPI sanity check: AUC = **0.9742** ✓ (expected: [0.90, 1.00])

---

## 1. Break Scores — All Three Axes Side-by-Side

| fixture | category | BPE score | AST score | Ensemble (max) |
|---|---|---|---|---|
| break_mimesis_alt_1 | mimesis_alt | 4.2043 | **6.3969** | 6.3969 |
| break_threading_provider_1 | threading_provider | **6.9589** | 5.9923 | 6.9589 |
| break_sqlalchemy_sink_1 | sqlalchemy_sink | **6.9568** | 4.5696 | 6.9568 |
| break_numpy_random_1 | numpy_random | 4.5933 | **4.9501** | 4.9501 |
| break_requests_source_1 | requests_source | **7.3802** | 4.3220 | 7.3802 |

The ensemble correctly selects the stronger axis for each break: AST wins on
`mimesis_alt` and `numpy_random`; BPE wins on the remaining three. However, the
ensemble score for `numpy_random` is only 4.9501 — the weakest of the five.

---

## 2. Ensemble Calibration Distribution (n=159)

| stat | BPE calibration | AST calibration | Ensemble calibration |
|---|---|---|---|
| min | -0.7986 | 0.0000 | 0.0000 |
| p50 | 2.6347 | 0.0000 | 2.6347 |
| p90 | 4.5538 | 2.2850 | 4.5538 |
| p99 | 5.8221 | 3.9061 | 5.8221 |
| max | 7.3732 | 3.9866 | **7.3732** |

The ensemble calibration max is identical to the BPE calibration max (7.3732).
This is the key diagnostic: for each of the 159 ordinary faker hunks, the
ensemble takes `max(bpe_cal, ast_cal)`. Because the BPE calibration max hunk
already scores 7.3732 on the BPE axis — and the AST axis never exceeds 3.9866 —
the ensemble calibration max is locked to the BPE ceiling.

---

## 3. Ensemble Separation Metrics

| metric | BPE-alone | AST-alone | Ensemble |
|---|---|---|---|
| max(calibration) | 7.3732 | 3.9866 | **7.3732** |
| p99(calibration) | 5.8221 | 3.9061 | **5.8221** |
| min(break_scores) | 4.2043 | 4.3220 | **4.9501** |
| **margin_vs_max** | **-3.1689** | **+0.3355** | **-2.4231** |
| **margin_vs_p99** | -1.6178 | +0.4159 | -0.8720 |

The ensemble margin_vs_max is **-2.4231** — substantially worse than AST-alone
(+0.3355). The ensemble does not help: it promoted the break minimum from 4.20
(BPE) to 4.95 (ensemble) by rescuing `mimesis_alt` and `numpy_random` via the
AST axis, but simultaneously promoted the calibration maximum from 3.99 (AST)
to 7.37 (BPE), erasing all separation.

---

## 4. Key Diagnostic: Why the Ensemble Fails

The root cause is **asymmetric calibration ceiling**: the two axes have
fundamentally different upper tails on ordinary faker code.

- **AST calibration max = 3.99.** Most ordinary faker hunks (locale string
  tables, data lists) score 0 on the AST axis because their parse-tree shape
  is idiomatic. The right tail is thin and capped at 3.99.

- **BPE calibration max = 7.37.** One ordinary faker hunk scores 7.3732 —
  higher than four of the five break fixtures. This single hunk is the reason
  BPE-alone fails (margin_vs_max = -3.17). On the AST axis that same hunk is
  unremarkable (it uses idiomatic faker structure), so its AST score is low,
  but the ensemble promotes it to max(7.37, ast_low) = 7.37.

**The ensemble selects the worst ceiling, not the best floor.**

When the two calibration distributions have different maxima, a max-ensemble
promotes the higher maximum to be the new calibration ceiling. Any break that
scores below that ceiling — in this case `numpy_random` at 4.95 — remains
undetectable regardless of how strong it is on the AST axis (4.95 > 3.99).

The specific hunk that causes the BPE calibration max to be 7.37 drives down
`requests_source` as well (margin_vs_max = +0.007, barely positive), but
`numpy_random` is the break most hurt by the ensemble: it scores 4.95 ensemble
vs a calibration max of 7.37, a gap of -2.42.

### Did BPE rescue `requests_source`?

In exp #13 (AST-alone), `requests_source` had the slimmest margin: AST score
4.3220, margin_vs_max +0.3355. In the ensemble, `requests_source` is taken on
the BPE axis (7.3802), which is higher — but so is the calibration max (7.3732).
The ensemble margin for `requests_source` is only **+0.0070**, versus AST-alone
+0.3355. The BPE axis raised the break score and the calibration ceiling in
lockstep: no net gain.

---

## 5. Verdict

**REGRESSED**

The max-ensemble scores worse than AST-alone on every separation metric:

- `margin_vs_max`: -2.4231 (ensemble) vs +0.3355 (AST-alone)
- `margin_vs_p99`: -0.8720 (ensemble) vs +0.4159 (AST-alone)

The ensemble does not achieve clean separation. `numpy_random` ensemble score
(4.9501) sits well below the ensemble calibration max (7.3732). There is no
threshold that separates all 5 breaks from all 159 calibration hunks on the
ensemble axis.

This is not a surprising result in retrospect: **the max-ensemble of two
correlated scorers sharing the same calibration set cannot improve the
separation margin if the weaker scorer's calibration ceiling dominates**. The
BPE calibration max is set by a single outlier faker hunk that happens to use
tokens foreign to the generic BPE reference; that hunk is not a paradigm break,
but BPE cannot distinguish it from one.

### Why AST-alone is strictly better than the ensemble

AST-alone succeeded because its calibration max (3.99) is determined purely by
the AST structure of ordinary faker code — locale string lists and data tables
that are structurally vanilla. The BPE outlier hunk is invisible to AST because
its parse-tree shape is normal. By introducing the BPE axis into the ensemble,
we import that BPE outlier hunk's inflated score into the ensemble's calibration
ceiling, erasing the clean gap.

---

## 6. Phase 13 Recommendation (Updated)

| scorer | faker verdict | margin_vs_max | notes |
|---|---|---|---|
| BPE-tfidf | REGRESSED (FULL OVERLAP) | -3.1689 | strong on vocab-foreign breaks, bad calibration ceiling |
| AST-contrastive | **CLEAN** | **+0.3355** | all 5 breaks above max(cal); slim but real margin |
| Ensemble (max) | REGRESSED | -2.4231 | BPE ceiling dominates; worse than AST-alone |

**Do not ensemble BPE-tfidf and AST-contrastive for faker-type repos.**

The AST-alone result from exp #13 stands as the strongest faker result in Phase 13.
The recommended production scorer for faker-type repos is:

```
ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max', depth=3)
threshold = 4.0  (above max(calibration)=3.9866, 0 FP on 159 hunks)
```

For repos where BPE-tfidf has clean calibration (FastAPI, rich), BPE-alone
remains the better choice (AUC 1.00, simpler). The two scorers should be
treated as independent, per-corpus choices rather than combined universally.

A smarter ensemble — e.g., product-of-probabilities or calibration-normalised
sum — might avoid this ceiling problem, but requires per-scorer calibration
normalisation (CDF mapping) before combining. That is Phase 14 scope.
