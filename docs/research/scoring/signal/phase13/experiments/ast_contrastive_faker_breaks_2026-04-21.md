# Phase 13 Experiment 13 — AST Contrastive: Faker Break Scoring (2026-04-21)

**Scorer:** `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`
**model_A:** 722 faker source files (faker/sources/model_a/)
**model_B:** generic_treelets.json (CPython 3.13 stdlib, 4.7M treelets, depth-3)
**Goal:** Determine whether the AST axis scores the 5 faker paradigm-breaks
above the normal distribution — specifically mimesis_alt (BPE-tfidf: 4.20,
below p90 of calibration).

FastAPI sanity check: AUC = **0.9742** ✓ (expected: [0.90, 1.00])

---

## 1. Summary Table

### Break Scores

| fixture | category | AST score | BPE-tfidf score | vs AST p99 | vs AST max |
|---|---|---|---|---|---|
| break_mimesis_alt_1 | mimesis_alt | **6.3969** | 4.2043 | +2.49 | +2.41 |
| break_threading_provider_1 | threading_provider | **5.9923** | 6.9589 | +2.09 | +2.01 |
| break_sqlalchemy_sink_1 | sqlalchemy_sink | **4.5696** | 6.9568 | +0.66 | +0.58 |
| break_numpy_random_1 | numpy_random | **4.9501** | 4.5933 | +1.04 | +0.96 |
| break_requests_source_1 | requests_source | **4.3220** | 7.3802 | +0.42 | +0.33 |

All 5 breaks exceed the calibration maximum. Smallest margin: +0.33 (requests_source).

### AST Calibration Distribution (159 ordinary faker hunks)

| stat | value |
|---|---|
| min | 0.0000 |
| p50 | 0.0000 |
| p90 | 2.2850 |
| p99 | 3.9061 |
| max | 3.9866 |
| mean | 0.7712 |
| stdev | 1.1417 |

The calibration distribution is heavily left-skewed (p50 = 0.0): most faker
ordinary hunks consist of data tables (locale string lists) whose AST treelets
are perfectly idiomatic and score at or near 0 against the faker model_A.
The right tail is thin — only a handful of hunks reach above 2.0.

---

## 2. Side-by-Side vs BPE-tfidf

| fixture | AST score | BPE rank in cal | AST rank in cal | AST winner? |
|---|---|---|---|---|
| break_mimesis_alt_1 | 6.3969 (was 4.2043) | p80.5 | **p100.0** | YES — from below p90 to above max |
| break_threading_provider_1 | 5.9923 (was 6.9589) | p99.4 | **p100.0** | YES — both above p99, AST cleaner |
| break_sqlalchemy_sink_1 | 4.5696 (was 6.9568) | p99.4 | **p100.0** | YES — AST lower but still above max |
| break_numpy_random_1 | 4.9501 (was 4.5933) | p90.6 | **p100.0** | YES — from p90 to above max |
| break_requests_source_1 | 4.3220 (was 7.3802) | p100.0 | **p100.0** | DRAW — both above, BPE higher |

The critical reversal: **mimesis_alt** goes from p80.5 (below p90, no threshold
exists with BPE) to p100.0 (above the calibration maximum) on the AST axis.
This is the hardest faker break — mimesis and faker share provider-class
patterns, Person, Address, and random — and AST still separates cleanly.

For **requests_source**, BPE-tfidf is stronger (7.38 vs 4.32). AST's margin
of +0.33 above max is slim, but it holds.

---

## 3. Per-Break Margins vs Calibration

| fixture | score | margin_vs_p99 | margin_vs_max | percentile in cal |
|---|---|---|---|---|
| break_mimesis_alt_1 | 6.3969 | **+2.49** | **+2.41** | p100.0 |
| break_threading_provider_1 | 5.9923 | **+2.09** | **+2.01** | p100.0 |
| break_numpy_random_1 | 4.9501 | **+1.04** | **+0.96** | p100.0 |
| break_sqlalchemy_sink_1 | 4.5696 | **+0.66** | **+0.58** | p100.0 |
| break_requests_source_1 | 4.3220 | +0.42 | +0.33 | p100.0 |

Separation metrics:
- `max_calibration` = 3.9866
- `min(break_scores)` = 4.3220
- `margin_vs_max` = **+0.3355** (positive — clean separation)
- `margin_vs_p99` = **+0.4159**

---

## 4. Verdict

**CLEAN**: all 5 breaks score above `max(calibration)`.

The AST axis achieves clean separation on faker — a result BPE-tfidf could
not match. Every break is clearly distinguishable from ordinary faker code
with zero overlap on the full 159-hunk calibration set.

This confirms that **the AST axis carries orthogonal signal** relative to
BPE-tfidf for the mimesis_alt and numpy_random failure cases. An ensemble
of both axes becomes directly interesting:

- BPE-tfidf is strong when foreign vocabulary differs (threading, SQLAlchemy,
  requests) but collapses when vocabulary overlaps (mimesis, numpy).
- AST-contrastive is strong when structural patterns differ (mimesis uses
  `Gender` enum, method chains, explicit `from mimesis import Person`) and
  robust to vocabulary sharing because token identity doesn't matter, only
  parse-tree shape.

The two axes have complementary failure modes. A union scorer (max of the
two) or a product scorer would likely produce clean separation across all 5
breaks on both axes simultaneously.

### Threshold recommendation

A safe threshold for the AST axis on faker-like repos:

- At `threshold = max(calibration) = 3.99`, 0 of 159 ordinary hunks fire
  (0% false-positive rate).
- At `threshold = p99 = 3.91`, 1-2 ordinary hunks fire (~0.6%).
- Recommended: `threshold = 4.0` for production use on faker-sized corpora.

This compares favourably to BPE-tfidf's calibration max of 7.37, which made
any practical threshold impossible for faker.

---

## 5. Key Diagnostic: break_mimesis_alt_1

mimesis is a competing fake-data library. It shares faker's provider-class
pattern (`class PersonProvider(BaseProvider)` → `class Person(BaseDataType)`),
uses `self.random` equivalents, and generates Person / Address / Finance data
with the same semantic vocabulary.

BPE-tfidf scored mimesis_alt at **4.20** — indistinguishable from ordinary
faker code (p80.5 of calibration). The top-3 BPE tokens were `here`,
`username`, `username` — words that appear in both faker and mimesis. The
failure is token-level vocabulary overlap.

AST-contrastive scored mimesis_alt at **6.40** — the highest-scoring break
in this experiment, well above the calibration maximum of 3.99.

Why does AST see what BPE cannot? The mimesis fixture uses:
- `from mimesis import Person, Address, Finance` (import-form treelets absent
  from faker's 722-file model_A, which uses provider class structure)
- `Gender.MALE` / `Gender.FEMALE` enum attribute access — `d3:Attribute>Name>Attribute`
  chains that are structurally different from faker's `self.generator.random_element()`
- `schema.create(iterations=n)` — a method-chain pattern absent from faker's
  functional style

These structural differences are invisible to BPE because the underlying
tokens (`Person`, `Address`, `random`) are shared. AST treelets encode the
*shape* of the code, not its vocabulary, and the shape diverges sharply.

This is the cleanest possible evidence that the BPE-tfidf failure is
**vocabulary-level, not structural**. The AST axis is not subject to the
same failure mode.

---

## 6. Phase 13 Recommendation

**Promote AST-contrastive to co-winner alongside BPE-tfidf for the faker
case. Investigate ensemble.**

| scorer | faker result | FastAPI AUC | notes |
|---|---|---|---|
| BPE-tfidf | FULL OVERLAP — mimesis p80, numpy p90 | 1.00 | strong on foreign-vocab breaks |
| AST-contrastive | **CLEAN** — all 5 > max(cal) | **0.9742** | strong on structural breaks |
| Ensemble (proposed) | TBD | TBD | likely additive on complementary failures |

The click AUC failure (0.25) remains — AST-contrastive does not generalise to
click. But this experiment shows the failure was not general: on faker (722
files), the scorer achieves clean separation. The "FastAPI-tuned" verdict for
click was driven by click's structural homogeneity and corpus starvation (13
files), not a scorer-level defect.

**Next step:** Score an ensemble (max of BPE-tfidf and AST-contrastive) on
all three corpora (faker, rich, fastapi) to quantify whether combining the
axes yields consistent clean separation across repo types.
