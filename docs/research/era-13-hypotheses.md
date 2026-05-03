# Era 13 — Hypothesis Space

## The Unsolved Problem

Post era-12 routing fix, the production scorer catches 105/115 catalog
fixtures (91.3% fixture-count recall, 91.5% per-category mean recall).
Ten fixtures remain uncaught:

| Corpus | Recall | FP | Uncaught |
|:---|---:|---:|:---|
| fastapi  | 30/32 (93.8%) | 0.572% | `validation_2`, `exception_handling_4` |
| rich     | 16/16 (100%)  | 1.225% | — |
| faker    | 15/16 (93.8%) | 1.957% | `synthetic_formula_1` |
| hono     | 15/17 (88.2%) | 0.514% | `hono_validation_2`, `hono_middleware_3` |
| ink      | 16/17 (94.1%) | 0.541% | `ink_dom_access_2` |
| faker-js | 13/17 (76.5%) | 0.911% | `error_flip_2`, `foreign_rng_1`, `http_sink_2`, `runtime_fetch_1` |

The 10 misses split into **three** diagnosable shapes (per pre-registered
triage of each fixture's hunk source):

| Shape | Count | Mechanism | Existing infra targets it? |
|:---|---:|:---|:---|
| **Cluster-rare attestation** (callee in 1–3 of N cluster files) | 5 | era-11's boolean union treats "1/63" same as "63/63" | Yes — Phase 10 plumbed but bench-inert |
| **Control-flow / AST-shape anomaly** (return-where-raise, missing-fallback, unusual middleware) | 4 | callee set is typical for the cluster; the *shape* is the anomaly | No — needs new signal class (Phase 4) |
| **Negative-shape** (cluster-typical callees absent, replaced by primitives) | 1 | hunk does not call anything cluster-typical; no rare callees and no shape anomaly | No — fundamentally outside callee/shape framing |

Per-fixture primary bucketing (used to compute realistic EV per phase):

| Fixture | Primary bucket | Rationale |
|:---|:---|:---|
| `foreign_rng_1` (faker-js) | cluster-rare | `Math.random` in 1/63 large cluster; control flow normal |
| `http_sink_2` (faker-js) | cluster-rare | `fetch` in 1/63 large cluster; control flow normal |
| `error_flip_2` (faker-js) | cluster-rare (fragile) | `Error` in 1/24 *small* cluster; Phase 10 evidence doc warns size-naïve `R=1` will fire on cal hunks too — Phase 4 is the more reliable lever |
| `validation_2` (fastapi) | cluster-rare | `app.patch`/`app.delete` in 1–2/289; voluptuous import lives **outside** the hunk so is undetectable by hunk-scoped scoring; control flow is normal try/except/raise |
| `hono_validation_2` (hono) | cluster-rare | `c.json` in 3/99; manual type-guard pattern is structurally plain (two early returns before success) |
| `exception_handling_4` (fastapi) | **AST-shape** (reclassified) | The paradigm break is `return JSONResponse` from except blocks instead of `raise HTTPException` — a return/raise count anomaly across 4 except blocks. The rare `app.put`/`app.delete` callees are incidental URL-method coverage; they may give Phase 2 a side-channel catch but the structural signal is more specific. |
| `runtime_fetch_1` (faker-js) | AST-shape | top-level `fetch` outside any handler; pattern absent from cluster |
| `hono_middleware_3` (hono) | AST-shape | unusual middleware shape vs cluster typical |
| `ink_dom_access_2` (ink) | AST-shape | direct DOM access where cluster routes through ink primitives |
| `synthetic_formula_1` (faker) | **negative-shape** | hunk: three plain functions returning f-strings (`{ns}_{seq:08d}`, `"-".join(parts)`, etc.). No `fake.X()` calls, no foreign callees, no unusual control flow. Anomaly = absence of cluster-typical callees. **No Phase 1–4 catches this.** |

Cluster-rare diagnostic detail (callee/cluster counts from
[`era12-phase10-cluster-rare-threshold.md`](evidence/era12-phase10-cluster-rare-threshold.md)):

| Fixture | Callee | Cluster size | Count in cluster |
|:---|:---|---:|---:|
| `foreign_rng_1` (faker-js) | `Math.random` | 63 | 1 |
| `http_sink_2` (faker-js) | `fetch` | 63 | 1 |
| `error_flip_2` (faker-js) | `Error` | 24 | 1 |
| `validation_2` (fastapi) | `app.patch` / `app.delete` | 289 | 1–2 |
| `hono_validation_2` (hono) | `c.json` | 99 | 3 |

## What's Been Tried — Do Not Retry

| Approach | Era | Outcome | Why this can't reach the residuals |
|:---|:---|:---|:---|
| Frozen-encoder embedding distance (cosine, Mahalanobis, whitened-Euclidean, context centroid) | 12 P6.x / P7.x / P8.c | failed (≤1/5) | locale-data tail dominates per-corpus threshold under any encoder metric |
| Per-token MLM (with and without context) | 12 P8 / P8.1 | failed (AUC 0.52 random on stripped code) | scoring hunk in isolation can't detect contextual anomaly; full-context variant collapses on locale-data tail |
| Per-token NN to context tokens | 12 P8e | failed (0/5) | NN distance only 38–52% of way to fjs threshold |
| MAX-z ensemble of (6.4 + 8d) | 12 P8f | failed (0/5; demoted RT_2) | breadth traded for specificity |
| Hardcoded framework-literal rules | 12 P9 | caught 3/5 BUT non-shippable | violates "no hardcoded domain knowledge" project rule |
| Larger pretrained encoders | n/a | not attempted | failure mode is the encoder's notion of similarity, not its size |

## Goals (pre-registered)

| Gate | Target |
|:---|:---|
| **G1 · Recall** | Fixture-count recall ≥ **94.0%** (108/115). Stretch: **96.5%** (111/115). |
| **G2 · Per-corpus FP** | All 6 corpora ≤ era-11 amended ceiling **2.5%**. |
| **G3 · No regression** | Zero of the 105 currently-caught fixtures regress to uncaught. |
| **G4 · Faker-js floor** | Faker-js recall ≥ **88%** (15/17). |
| **G5 · Threshold stability** | Max per-corpus threshold CV ≤ 3% (era-10 standard). |
| **G6 · No hardcoded domain knowledge** | All Phase 10 + AST queries derive from corpus statistics. |

Recall ceilings under the corrected taxonomy:

- **Architectural ceiling: 9/10** (114/115 = 99.1%). `synthetic_formula_1`
  is a negative-shape anomaly that no callee-distribution or
  control-flow rule can flag; reaching 100% requires an architectural
  change (negative-shape detection / Phase 5 reframed).
- **Phase 2 reliable ceiling: +4 catches** (109/115 = 94.8%). Targets:
  `foreign_rng_1`, `http_sink_2`, `validation_2`, `hono_validation_2`.
  G1 floor clears on Phase 2 alone if the size-conditional sweep finds
  a passing config.
- **Phase 2 + Phase 4 ceiling: +8 catches** (113/115 = 98.3%). Adds the
  4 AST-shape residuals (`exception_handling_4`, `runtime_fetch_1`,
  `hono_middleware_3`, `ink_dom_access_2`). Stretch G1 (96.5%) clears
  if Phase 4 delivers ≥ 2 of its 4 targets.
- **Fragile +1: `error_flip_2`** (114/115). Requires either a
  size-conditional Phase 2 setting that doesn't blow up small-cluster
  FPs, or Phase 4 picking up the throw-where-cluster-doesn't shape.

## Phases

### Phase 1 — Phase 10 plumbing audit (½ day)

Verify whether `cluster_rare_threshold` is firing end-to-end in the full
bench. The era-12 evidence shows a discrepancy:

| Test | Threshold (rare=0) | Threshold (rare=2) | Delta |
|:---|---:|---:|---:|
| Standalone probe (small cal sample) | 3.94 | 8.94 | **+5.0** ✓ |
| Full bench (K=7 multi-seed median) | 4.86 | 4.86 | **0.0** ✗ |

The standalone probe shows the rule firing on calibration hunks. The
full bench shows threshold completely unchanged. The era-12 memo
attributed this to "K=7 median smoothing OR cal samples not hitting
rare-attested hunks" — both plausible, but **also exactly the shape of
a parameter-plumbing bug** (`cluster_rare_threshold=2` getting dropped
on some boundary so the rule never fires in the full bench). If the
rule were firing symmetrically as the memo theorises, threshold should
move; that it doesn't is a clue.

**Tasks:**

1. Trace the parameter through `bench CLI → RunConfig → build_scorer →
   calibrate_multi_seed → CallReceiverScorer.weighted_contribution_for_file`.
   Confirm it isn't dropped on any boundary.
2. Add an instrumented counter in `weighted_contribution_for_file` that
   increments only when the rare branch fires. Run bench with
   `cluster_rare_threshold=2`; confirm non-zero fire counts on both
   fixture and calibration paths.
3. Run bench with `threshold_n_seeds=1` and `cluster_rare_threshold=2`.
   If threshold jumps by ~+5.0 → median was masking. If it stays at
   0.0 → parameter isn't reaching the cal path.

**Decision rule:**
- Plumbing bug found → fix it, then proceed to Phase 2. Recall does
  not move from this fix alone (see EV note below).
- No plumbing bug → proceed to Phase 2 (the symmetric-firing reading
  is correct; fix the calibration policy instead).

**Expected EV:** Phase 1 is a **precondition gate**, not a recall
lever. The Phase 10 evidence doc establishes that the threshold-
inflation cancellation is fundamental to `max(cal_scores)`
thresholding combined with a contribution that fires symmetrically on
calibration and fixture hunks: even if the rule fires correctly in
the full bench, fixture and cal scores both move by `cluster_bonus`
in lockstep, and net catch impact is zero. Phase 2's percentile
change is what actually decouples the two; Phase 1 only ensures we
trust Phase 2's premise.

### Phase 2 — Size-conditional rare + percentile threshold sweep (1–2 days)

Run after Phase 1 regardless of plumbing-bug verdict (Phase 1 is a
precondition gate, not a substitute).

Two coupled changes:

1. **Replace** `_compute_threshold(cal_scores, threshold_percentile=None)`
   (takes max) with a percentile threshold so a few inflated cal scores
   don't move the threshold. The fixture's rare-bonus still adds to its
   score, but the threshold doesn't move with it.
2. **Add** a `cluster_size_min` axis to the rare-attestation rule. The
   Phase 10 evidence doc (option 4) flags that a global integer
   `cluster_rare_threshold` conflates two phenomena: in *large*
   clusters "1 of 63" is genuinely anomalous (rare on cal, rare on
   fixtures), but in *small* clusters "1 of 24" is typical for many
   callees (fires on cal too, no leverage). Size-conditioning decouples
   them.

The new rule:

```
rule fires iff cluster_size >= cluster_size_min
           AND cluster_count[c] <= cluster_rare_threshold
```

**Sweep grid** (12 configs, same budget as the original plan):
`cluster_size_min ∈ {0, 20}` × `cluster_rare_threshold ∈ {1, 2}` ×
`threshold_percentile ∈ {p95, p99, max}`.

Dropped vs original plan: `R = 3` (no motivation in the evidence doc;
firing on more cal hunks worsens cancellation), `p97.5` (redundant
between p95 and p99). Added: `S_min` axis, `R = 1` (Phase 10
evidence doc option 4 motivates the extreme-rare case for large
clusters).

**Per-corpus pre-registered targets** (FP expressed as **headroom
budgets** off current, reserving ~½ of the gap to G2's 2.5% ceiling
for Phase 4):

| Corpus | Current FP | Phase 2 budget | Phase 2 ceiling | Phase 4 budget remaining | Recall must reach ≥ |
|:---|---:|---:|---:|---:|:---|
| fastapi  | 0.572% | +1.0pp | 1.57% | +0.93pp | current + 1 (`validation_2`) |
| rich     | 1.225% | +0.6pp | 1.83% | +0.67pp | 100% (no regression; no Phase 2 target) |
| faker    | 1.957% | +0.3pp | 2.26% | +0.24pp | current (no Phase 2 target; `synthetic_formula_1` is negative-shape) |
| hono     | 0.514% | +1.0pp | 1.51% | +0.99pp | current + 1 (`hono_validation_2`) |
| ink      | 0.541% | +1.0pp | 1.54% | +0.96pp | current (no Phase 2 target) |
| faker-js | 0.911% | +0.8pp | 1.71% | +0.79pp | 88% (15/17) — 2 reliable + `error_flip_2` fragile |

Note: rich/faker/ink have **no Phase 2 residual** targeted. Their
budgets are tighter on purpose — if Phase 2 spills FP onto these
corpora the sweep is over-generous and the (S_min, R, percentile)
triple should be rejected even if it catches faker-js residuals.

**Decision rule:** ship the (S_min, R, percentile) triple with the
most catches subject to:
- **(a)** Phase 2-target corpora (fastapi, hono, faker-js) FP cost ≤ budget; AND
- **(b)** non-target corpora (rich, faker, ink) FP cost ≤ budget.

If no triple passes both, document the bound and ship
`cluster_rare_threshold=0` (status quo).

**Risk:** percentile thresholding is a calibration-policy change with
broader FP impact. Must verify all 6 corpora, not just fjs.

### Phase 3 — Bench symmetry audit (1–2 days)

Era 12's routing bug existed because catalog and real-PR scoring used
different code paths to resolve `file_path`. The lesson is sharp:
*asymmetric scoring paths are where leaks live*. Sweep the rest of the
bench harness for siblings of that pathology before believing any
remaining residual is genuinely unflaggable.

**Inventory of paths to compare:**

1. `_score_fixtures` (catalog) vs `_score_pr_hunks` (real PRs):
   file_path resolution, prose-blanking, typicality short-circuits,
   host-injection coverage.
2. Calibration path vs scoring path: identical preprocessing? Same
   `synthesize_hunk_in_host`? Same `_strip_break_meta` treatment?
3. Fixtures **without** `host_file` metadata: do any still fall back
   to the catalog-phantom path? If so, they're scored under the old
   defeated routing.
4. `is_atypical_file` short-circuit: does it fire asymmetrically on
   synthesized vs real content for any other corpus?

**Output:** an audit memo, plus fixes for any asymmetry found. Each
fix re-bench and measure.

**Expected EV:** unknown — could be 0 catches, could be another +2-3
catches like era 12. Cheap insurance.

### Phase 4 — AST-shape / control-flow features (1–2 weeks)

Targets the 4 residuals classified as AST-shape primary
(`exception_handling_4`, `runtime_fetch_1`, `hono_middleware_3`,
`ink_dom_access_2`) plus `error_flip_2` as backup if Phase 2's small-
cluster firing doesn't surface it. The anomaly is structural:
returns where the cluster typically raises, missing-fallback patterns,
unusual exception-handler shapes, top-level fetch outside any handler.
No callee-set or embedding rule will catch these.

Cheap, rule-based, no hardcoded domain knowledge.

**Sub-phases (run all per "no early-stopping"):**

| 4.1 | Tree-sitter query catalogue: extract per-hunk control-flow primitives — return/throw counts, try/catch shapes, conditional-fallback presence, async/await density, exception-handler arities. Statistics-only, no hardcoding. |
|---|---|
| 4.2 | Build per-cluster distributions of those primitives at fit time, alongside `cluster_attested`. |
| 4.3 | Per-hunk anomaly score: KL divergence (or χ²) of hunk's primitive distribution vs cluster's typical. |
| 4.4 | Calibrate per-corpus FP threshold; bench against all 115 fixtures. Check FP stability + threshold CV. |
| 4.5 | If 4.4 clears gates: ship as Stage 1.6 (additive penalty alongside call-receiver `cluster_bonus`). If not: document the bound and the cross-domain AUC pattern. |

**Pre-registered question:** does the AST-shape penalty catch ≥ 2 of
the 4 non-rare residuals at FP ≤ era-12 levels + 0.3pp on each corpus?

**Risk:** control-flow signals worked on isolated corpora in earlier
eras but collapsed cross-domain (era 2 — AST-structural signals were
FastAPI-tuned and collapsed on rich, click). The cluster-conditional
framing (compare hunk to *its cluster's* control-flow distribution,
not a global typical) is the hedge against re-running into that
failure mode.

**Honest EV:** 30–50%. Higher if Phase 1+2 already deliver the
cluster-rare residuals (then Phase 4 only has to clear the remaining
3–4 control-flow ones).

### Phase 5 — Synthetic mutation generation + negative-shape detection (DEFERRED)

Two deferred directions, both unlocked by the same infrastructure:

1. **Synthetic mutation at scale.** Combine `synthesize_hunk_in_host`
   with an AST-mutation generator → 10k+ synthetic break/control pairs
   at the data-generating distribution of real PRs. With that volume,
   supervised classifiers stop overfitting to "is this a catalog file."
2. **Negative-shape detection.** `synthetic_formula_1` is the only
   residual whose anomaly is "callees that should be present aren't."
   No callee-distribution or control-flow rule fires. The signal would
   need to compare the hunk's callee set to the cluster's *expected*
   callee distribution and flag underuse, not just rare presence. This
   is a different signal class from anything currently in the scorer.

Run if Phases 1–4 clear stretch G1 (≥ 96.5%) but the user wants the
remaining 1–2 residuals (architectural ceiling 99.1%, true ceiling
100% requires the negative-shape direction).

## What we will NOT do

| Approach | Reason |
|:---|:---|
| Further frozen-encoder embedding-distance variants | Era 12 exhausted this. Locale-data tail is the binding constraint; encoder choice doesn't change it. |
| Larger pretrained encoders | Same reason — failure mode isn't encoder size. |
| Additional MLM / per-token surprise variants | Phase 8.x exhausted this. Per-token MLM is essentially random on stripped code. |
| Hardcoded framework literals (`{"axios", "fetch", "hono", ...}`) | Project rule. Phase 9 was caught by this rule and is non-shippable. |
| Per-corpus path-pattern hardcoding (`src/locales/* → strict`) | Same project rule. Banned since era 1. |

## Execution order

| Day | Phase | Output |
|---:|:---|:---|
| 1   | Phase 1 plumbing audit | bug-or-not verdict; if bug, fix + re-bench |
| 2–3 | Phase 2 percentile sweep (only if Phase 1 was clean) | shipped (percentile, rare-threshold) pair or documented bound |
| 4–5 | Phase 3 symmetry audit | audit memo + any fixes found |
| Week 2 | Phase 4 AST-shape | shipped Stage 1.6 or documented bound |
| Final  | Cumulative re-bench across 6 corpora; ship best config that clears G1–G6 | era-13 evidence doc + narrative |

## Headline targets

- **Conservative** (Phase 2 size-conditional sweep finds a passing
  config on the 4 reliable cluster-rare targets): recall **94.8%**
  (109/115). Clears G1 floor; faker-js 15/17 (88.2%) clears G4.
- **Stretch** (Phase 2 reliable + Phase 4 delivers ≥ 2 of 4
  AST-shape): recall **96.5–98.3%** (111–113/115).
- **Architectural ceiling** (everything in Phases 1–4 lands):
  **99.1%** (114/115). `synthetic_formula_1` remains uncaught
  (negative-shape; outside Phase 1–4 framing).
- **Floor** (everything stalls): documented bounds on
  cluster-rare/calibration interaction (sweep didn't find a passing
  config), on AST-shape cross-domain stability (era-2 collapse
  recurs), and on negative-shape fixtures requiring a new signal
  class. Still useful, era-12-style.

## Era-12 → Era-13 Transition

Strict 115-fixture verdict parity is the standing rule. Apply at
era-13 dispatch.

Era-13 dispatch is a single short prompt: "Read
`docs/research/era-13-hypotheses.md` and execute Phase 1 (Phase 10
plumbing audit) per the specification there. Report findings before
proceeding to Phase 2."

## End of Document
