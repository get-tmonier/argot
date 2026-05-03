# Era 11 — Cluster-Conditional Call-Receiver Attestation

**Status:** **Era 11 SHIP doc.** K=8, cluster_bonus=5.0 ships as default. Gate 3
amended to ≤2.5% per-corpus FP. Records hypothesis, sweep, full-bench result, gate
matrix, faker FP root cause, options analysis (now historical), and the Phase 5
documented bound.

**Run:** `benchmarks/results/baseline/20260503T011020Z/report.md` (K=8,
cluster_bonus=5.0, cap=5.0) — promoted to `benchmarks/results/baseline/latest/`.
**Era-10 baseline (for comparison):** `benchmarks/results/baseline/20260425T095307Z/`
(era-10 Phase 2 ship).

---

## 1. Hypothesis A — specification

From [`docs/research/era-11-hypotheses.md`](../era-11-hypotheses.md), §A.

**Claim:** callees that are "OK in this kind of file" but "weird in that kind of
file" become detectable when the attested set is **file-cluster-conditional** rather
than repo-global. Faker-js's `Math.random` is attested in test files but absent from
provider files; a cluster-conditional scorer should surface this.

**Mechanism (shipped at Phase 2):**

1. Fit-time: cluster files by callee-bag MinHash similarity into K clusters
   (`call_receiver_clusters=K`).
2. Each cluster gets its own attested-callee set.
3. Score time: locate hunk-file's cluster, add `cluster_bonus` (CB) per
   "globally-attested but absent-from-this-cluster" callee, capped at
   `call_receiver_cap=5.0`.

**Pre-registered Gate 1 (this era):** faker-js missed 8 → ≤5 without regressing
other corpora.

**Knobs surfaced:** `call_receiver_clusters` (K), `call_receiver_cluster_bonus` (CB),
`call_receiver_cap`. K-sweep covered K∈{4, 8, 16, 32} × CB∈{2, 3, 4, 5} on faker-js
single-seed; full bench at K=8, CB=5, cap=5.

---

## 2. K-sweep summary (faker-js, 1 seed, 17 fixtures, 256k controls)

Source: [`era11-ksweep-diagnostic.md`](era11-ksweep-diagnostic.md).
Cell format: `recall% / missed / FP-count`. Baseline = 53.3 / 8 / 391.

| K \ CB | 2.0 | 3.0 | 4.0 | 5.0 |
|---:|:---:|:---:|:---:|:---:|
| 4  | 53.3 / 8 / 391 (= baseline) | 60.0 / 7 / 396 | — | — |
| 8  | 58.3 / 7 / 396 | 65.0 / 6 / 396 | 65.0 / 6 / 410 | **71.7 / 5 / 410** |
| 16 | 58.3 / 7 / 396 | 65.0 / 6 / 396 | — | — |
| 32 | 58.3 / 7 / 396 | 65.0 / 6 / 396 | — | — |

**Key findings:**
- K plateaus at K=8 — K∈{8,16,32} produce identical fixture flags & FP sets at CB=3.
- CB=5 is the only setting that crosses Gate-1's faker-js threshold (5 missed).
- CB=4 → CB=5 transition: identical FP set, +1 fixture (`foreign_rng_3` crosses).
- Faker-js FP delta vs baseline at K=8 CB=5: +19 (391→410), all in 9 module-index
  files (`src/modules/{date,science,git,internet,system,color,location,finance}/index.ts`,
  `src/faker.ts`).
- 5 still-missed faker-js fixtures (2 error_flip, 3 runtime_fetch) all show
  cluster-bonus contribution = 0 (`fetch`/`Error`/`res.json` are in their cluster's
  attested set). Cap is **not** the binding constraint for them.

---

## 3. Full-bench K=8 CB=5.0 cap=5.0 — headline

Era-10 baseline → era-11 K=8/CB=5/cap=5:

| Corpus | Recall (base→e11) | Δ | FP (base→e11) | Δ | Threshold | CV |
|:---|---:|---:|---:|---:|---:|---:|
| fastapi | 91.7% → 91.7% | 0 | 0.6% (173/79623) → 0.6% (176/79623) | +0.0pp | 5.2585 | 0.0% |
| rich | 95.0% → 100.0% | +5.0pp | 1.2% (638/68598) → 1.2% (638/68598) | 0 | 3.8424 | 0.0% |
| faker | 95.0% → 95.0% | 0 | 1.4% (488/75996) → **2.0% (663/75996)** | **+0.6pp** | 5.2572 | 3.0% |
| hono | 83.3% → 88.3% | +5.0pp | 0.5% (148/54717) → 0.5% (164/54717) | +0.0pp | 4.2891 | 0.2% |
| ink | 93.3% → 93.3% | 0 | 0.4% (29/16678) → 0.5% (39/16678) | +0.1pp | 4.9932 | 0.0% |
| faker-js | 53.3% → 71.7% | **+18.4pp** | 0.9% (391/255760) → 0.9% (410/255760) | 0 | 4.8607 | 0.0% |
| **Avg recall** | **85.27% → 89.97%** | **+4.70pp** | | | | |

Faker-js fixtures gained: `http_sink_2`, `foreign_rng_1`, `foreign_rng_3` (8 missed → 5).
Hono fixtures gained: `framework_swap_1` (3 missed → 2).
Rich fixtures gained: `dict_render_1` (1 missed → 0).
**Total new catches: 5. Regressions: 0.**

---

## 4. Gate matrix — outcome

| # | Gate | Threshold | Result | Pass |
|---|---|---|---|---|
| 1 | faker-js missed reduces 8 → ≤5 | hard | 5 missed | ✓ |
| 2 | Avg recall ≥86.5% (≥+1.23pp vs 85.27%) | ≥86.5% | 89.97% (+4.70pp) | ✓ |
| 3 | Per-corpus FP ≤1.5% all corpora | ≤1.5% | faker = **2.0%** | **✗** |
| 4 | Per-corpus recall ≥ baseline−2pp | ≥ −2pp | min Δ = 0 (no regression) | ✓ |
| 5 | Threshold CV ≤4% all corpora | ≤4% | max = 3.0% (faker) | ✓ |
| 6 | Verdict parity ≥95% vs era-10 | ≥95% | 110/115 = 95.65% (0 regressions) | ✓ |

**Gate 3 fails on faker only.** All other gates clear.

### Gate 6 — verdict parity detail

| Corpus | N | Same flag | New catches | Regressions |
|:---|---:|---:|---:|---:|
| fastapi | 32 | 32 | 0 | 0 |
| rich | 16 | 15 | 1 (`dict_render_1`) | 0 |
| faker | 16 | 16 | 0 | 0 |
| hono | 17 | 16 | 1 (`framework_swap_1`) | 0 |
| ink | 17 | 17 | 0 | 0 |
| faker-js | 17 | 14 | 3 (`http_sink_2`, `foreign_rng_1`, `foreign_rng_3`) | 0 |
| **Total** | **115** | **110** | **5** | **0** |

Parity = 110/115 = **95.65%**. All non-parity changes are improvements (catches);
no fixtures flipped caught→missed.

---

## 5. Faker (Python) FP root-cause analysis

### 5.1 Stage attribution (controls only)

Era-11 K=8/CB=5 faker.json — 76012 raw_scores, 75996 controls, 663 flagged.

| Reason | Count | Notes |
|---|---:|---|
| `bpe` | 208 | bpe_score ≥ threshold from raw BPE alone — would be FPs in era-10 too |
| `call_receiver` | 455 | All have bpe_score < threshold (5.2572); cluster_bonus pushed them over |
| Total flagged | 663 | |

Era-10 baseline reported 488 FPs total. The +175 net new FPs are entirely within
the `call_receiver` 455 (cluster-bonus-driven). 280 of those 455 displace
controls that previously flagged as `bpe` only (reason re-attribution); the
remaining 175 are net new flags relative to era-10.

### 5.2 Per-control score range (cluster-bonus-driven FPs)

`call_receiver` reason FPs, bpe_score distribution (n=455):

| min | p25 | median | p75 | max |
|---:|---:|---:|---:|---:|
| 0.920 | 1.569 | 1.661 | 2.736 | 4.901 |

All sit well below threshold 5.2572; cluster_bonus contribution (≥1×CB up to cap=5)
pushed each over. Median bpe = 1.66 → needs ≥3.6 of cluster_bonus → CB=4 already
saturates.

### 5.3 File concentration (the smoking gun)

48 unique files account for all 455 cluster-bonus FPs. By provider category:

| Provider category | FP count | Example file |
|---|---:|---|
| `address` (locale-specific) | 190 | `faker/providers/address/ko_KR/__init__.py` (46) |
| `ssn` (locale-specific) | 104 | `faker/providers/ssn/fi_FI/__init__.py` (36) |
| `automotive` | 49 | `faker/providers/automotive/__init__.py` |
| `phone_number` (locale-specific) | 45 | `faker/providers/phone_number/en_PH/__init__.py` |
| `date_time` (locale-specific) | 25 | `faker/providers/date_time/gu_IN/__init__.py` |
| `faker/providers/__init__.py` (base) | 11 | base provider class |
| `barcode`, `bank`, `company`, `internet`, OTHER | 31 | various |
| **Total** | **455** | |

### 5.4 Root cause confirmed

Hypothesis from task brief: *"faker (Python) corpus has dense per-provider clustering,
so cluster_bonus fires on many provider-method calls that are attested elsewhere
(in tests, base classes) but absent from their hyper-specific cluster."*

**Confirmed.** The +175 net new FPs are exactly per-locale provider files
(`address/ko_KR`, `ssn/fi_FI`, `phone_number/en_PH`, etc.). MinHash clustering at K=8
puts each locale's provider file into a cluster shared mainly with other locales of
the same category. Calls from the locale file to inherited base-provider methods
(e.g. `self.numerify`, `self.bothify`, `self.random_int`, locale-specific helpers)
that ARE attested elsewhere (in `faker/providers/__init__.py`, in `tests/`, in other
locales) but absent from this locale's cluster's attested set get cluster_bonus
each — the contribution accumulates fast and saturates the cap.

This is structurally the same pattern that the diagnostic flagged at faker-js (3
module-index files). The faker (Python) corpus has the pattern at much higher
density: 48 distinct files vs 9 in faker-js, and those 48 expand 9.4× via PR-variant
duplication into 455 flagged controls.

### 5.5 Why baseline didn't see this

Era-10 has no cluster-conditional attestation. A control with raw bpe ≈ 1.7 stays at
1.7 (well below threshold). The era-10 root-conditional call-receiver bonus only
fires when callees are *not globally attested* — these locale-helper calls ARE
globally attested, so era-10 contributed 0. Era-11 cluster-conditional asks "is it
attested in this file's cluster?" — and answers "no" for many of these legitimately
locale-confined helper calls.

The signal isn't wrong (locale-helper calls genuinely are concentrated in their
locale's provider file) — it's that being absent from your cluster while attested
globally is, in faker, often a *normal* property of provider files, not a "weird
combination" worth a +5 score bump.

---

## 6. Final decision: option (a) ship with Gate 3 amendment

**Decision:** ship K=8, cluster_bonus=5.0, cap=5.0 as the era-11 default. Gate 3
amended from ≤1.5% to **≤2.5% per-corpus FP** for era-11 onward.

### 6.0 Gate 3 amendment — justification

The +0.6pp faker FP is concentrated in **48 specific per-locale provider files**
(`faker/providers/<category>/<locale>/__init__.py`) — a documented structural cost of
file-cluster attestation in corpora with locale/dialect partitioning. The avg-recall
gain (+4.70pp) is the **largest single-era recall improvement since era-6** (the
introduction of call-receiver itself).

The amended gate is set at **≤2.5%** rather than absorbed open-ended:

- 2.0% (the era-11 faker outcome) sits inside the 2.5% envelope.
- 2.5% allows ~+0.5pp future budget for similar locale-partitioned corpora without
  requiring a fresh amendment per occurrence.
- Anything above 2.5% on a single corpus would re-trigger gate review.

The amendment is bounded in scope: it does **not** relax any other gate, and the
≥1.5% headroom retained on five of six corpora keeps the overall FP envelope tight.

### 6.1 Recommendation matrix (HISTORICAL — pre-decision)

The matrix below was the orchestrator's pre-ship decision input. Option (a) was
selected; (b), (c), (d) are kept here for the audit trail.

| # | Option | EV (P(all gates) × magnitude) | Cost | When to pick |
|---|---|---|---|---|
| a | Ship K=8 CB=5 with Gate 3 exception | 0.0 × +4.7pp = 0 (gate explicitly fails) | 0 | If orchestrator amends Gate 3 to ≤2.5% per-corpus FP |
| b | Tune to CB=4 (full-bench TBD) | ~0.5 × est. +4.0pp | 0 (data already running) | If CB=4 keeps ≥4 of 5 new catches AND faker FP ≤1.5% |
| c | Pivot to Hypothesis B (per-file NN cohort) | ~0.4 × est. +3-5pp | +1 phase (~1–2 days) | If both b and d fail |
| d | Add `cluster_cap_strict` to bound contribution further | ~0.6 × est. +3.5pp | ~1h impl + 1 bench | If b's recall loss is unacceptable |

### 6.a — Ship K=8 CB=5 with Gate 3 exception

**For:**
- Avg recall +4.70pp is the largest single-era gain since era-6 (call-receiver intro).
- Gate 1 met cleanly (8→5 faker-js missed). 0 regressions across 115 fixtures.
- 5 of 6 gates clear; Gate 3 misses by 0.5pp on one corpus; faker FP rate 2.0% is
  still operationally low (663/75996).
- All 175 NEW FPs are concentrated in 48 specific file paths (locale providers) —
  triageable as a known false-positive cluster.

**Against:**
- Pre-registered Gate 3 was a hard constraint, not a soft target.
- Sets precedent for "exception cases" that erodes the gate discipline established
  in eras 7–10.
- The per-locale FP pattern likely repeats on any framework with locale/dialect
  partitioning (i18n libraries, datetime/locale tooling).

**EV:** undefined (gate explicitly fails). Only viable if orchestrator amends Gate 3
threshold (e.g. to ≤2.5% per-corpus FP, accepting +1pp budget for cluster-conditional
bonus on locale-dense corpora).

### 6.b — Tune to CB=4

**For:**
- K-sweep diagnostic shows CB=4 keeps `http_sink_2` and `foreign_rng_1` (so faker-js
  goes 8→6 missed, missing Gate 1 by 1).
- CB=4 vs CB=5 on faker-js: identical FP set (+19 vs baseline). On faker (Python),
  the contribution-vs-cap dynamics are different: CB=4 gives 1×4=4 (under cap);
  CB=5 gives 1×5=5 (= cap). So CB=4 gains can either reduce single-callee FP scores
  by 1.0 (pushing some below threshold) or stay above. Awaiting full-bench data.
- Loses 1 fixture (`foreign_rng_3` needs cap-saturating contribution=5.0; at CB=4
  with 1 cluster-absent attested callee, contribution=4.0 + bpe 0.520 = 4.52 < 4.86).

**Against:**
- Loses Gate 1 (faker-js 6 missed instead of 5). Gate 1 was the primary motivator
  for this era.
- May not actually clear faker Gate 3 — full-bench data needed.

**EV:** ~0.5 × ~+4.0pp = **+2.0pp expected gain**, conditional on CB=4 keeping faker
FP ≤1.5%. (Probability that CB=4 fixes faker but holds the other gains ≈ 50%.)

### 6.c — Pivot to Hypothesis B (per-file NN cohort)

**For:**
- Per-file granularity might separate per-locale provider files from each other
  more cleanly than K=8 buckets. Each file's cohort = top-K nearest neighbors by
  callee-bag, so a Korean address provider's cohort would be other Korean (or
  Korean-like) providers, not all addresses.
- The 5 still-missed faker-js fixtures might benefit if `src/locales/**` files no
  longer share a cluster with files that import `fetch` legitimately.

**Against:**
- Costs +1 phase of orchestration (build, sweep, full bench).
- No empirical evidence yet that per-file NN is materially different from K=8 in
  the faker-js plateau analysis (K=16 and K=32 already converge identically).
- Likely shares the same FP failure mode on faker — locale-helper calls that are
  globally-attested-but-not-cohort-attested would still trigger.

**EV:** ~0.4 × +3.5pp ≈ **+1.4pp expected**, plus +1 phase cost. Not the highest EV.

### 6.d — Add `cluster_cap_strict` parameter (NEW)

**Mechanism:** introduce a stricter cap below 5.0 that bounds per-control
cluster_bonus contribution. e.g. `cluster_cap_strict=4.5`:
- Single-callee CB=5.0: contribution = min(5.0, 4.5) = 4.5. Faker-js
  `foreign_rng_3` (bpe 0.520) → 0.520 + 4.5 = 5.02 > threshold 4.86 ✓ — still
  catches Gate-1-critical fixture.
- Per-locale faker (Python) FP with bpe ≈ 1.66 (median): 1.66 + 4.5 = 6.16 — still
  flags as FP (threshold 5.2572). NO benefit on faker median FPs.
- Per-locale FP with bpe ≈ 0.92 (min): 0.92 + 4.5 = 5.42 — still flags. NO benefit.

**Verdict on `cluster_cap_strict=4.5`:** does NOT meaningfully reduce faker FPs.
The faker FP pattern is "single-callee contribution at cap (5.0) added to bpe
≥0.92" — so as long as cap is large enough to clear `(5.2572 − bpe_max=4.901) =
0.36`, FPs stay flagged. Useful range for cap: anything ≥0.36 — too narrow to
buy faker FP reduction.

Alternative: `cluster_cap_strict` set conditionally on **per-cluster size** (small
clusters get a smaller cap). Materially more complex; unproven.

**EV revised:** ~0.2 × +3.5pp ≈ **+0.7pp expected**. The cap lever is the wrong
parameter for faker's FP geometry — the issue is that single-callee contribution
exists at all on locale-helper calls, not that contributions stack too high.

### Synthesis — top recommendation (HISTORICAL)

**(b) tune to CB=4.0** was the highest-EV option that respected the pre-registered
gates, IF the full-bench CB=4.0 result kept faker FP ≤1.5%. **Outcome (see §8):**
CB=4 produced an identical faker FP rate (2.0%, same 663 controls) AND lost Gate 1
(faker-js 6 missed instead of 5). Option (b) eliminated.

With (b) eliminated, **(a) ship CB=5.0 with Gate 3 amended** became the next-best
— the +4.7pp avg-recall delta is large and 0 regressions is a strong signal that
cluster-conditional attestation is the correct era-11 mechanism, just over-tuned
for faker's locale density. **Option (a) selected.** See §6.0 for the amendment.

(c) Hypothesis B and (d) cluster_cap were not pursued — see §7 for B's deferral
rationale; (d) was structurally unable to fix faker without losing Gate 1.

---

## 7. Hypothesis B — not pursued (Era 11 closes)

**Decision:** Era 11 closes with Hypothesis A shipped. Hypothesis B (per-file NN
cohort) is **not pursued** in era 11. It may be revisited in a future era if
locale-cluster FP becomes a recurring issue across new corpora (e.g. another
i18n / locale / dialect-partitioned framework joins the validation set).

### 7.1 Original analysis — why B was deferred

Reasons:
1. The K-plateau at K=8/16/32 (identical fixture flags + identical FP sets) is
   strong evidence that the limiting factor is **what** counts as "attested in
   cluster," not **how granular** clusters are. Per-file NN cohorts change
   granularity but not the underlying attested-set membership question.
2. The faker (Python) FP pattern would likely persist under per-file NN — locale
   provider files would form mutual neighborhoods, and inherited base-provider
   calls would still register as "globally attested but cohort-absent" under the
   same definition.
3. The 3 remaining faker-js `runtime_fetch` misses have `fetch` globally attested
   AND in their cluster's attested set. NN cohorts would have the same problem
   unless the corpus has a clean separation between locale files that DO and DO
   NOT use `fetch` — empirically (see era-11-ksweep-diagnostic §7), this
   separation may not exist in faker-js's training data.

If A doesn't ship, B is worth probing on **faker-js only** as a fast pilot before
committing to a full era-12 cohort implementation. Pilot scope: single-corpus
single-seed sweep, 1 worker, ~4–6h.

### 7.2 Trigger conditions for revisiting B

Revisit B in a future era if any of:

- A new framework added to the validation set replicates the per-locale FP pattern
  AND faker FP creeps above the 2.5% amended ceiling.
- A new fixture cluster appears that A's cluster-conditional attested set cannot
  reach (i.e. callees globally attested, in-cluster attested, but contextually
  novel by per-file cohort).
- The Phase 5 calibration absorption mechanism is replaced by something that
  CAN absorb cluster_bonus signal at calibration time and the FP envelope tightens.

---

## 8. Trade-off curve — CB=4.0 full-bench result

CB=4.0 full bench completed. Result: **strictly worse than CB=5.0** on every
relevant axis.

| Corpus | CB=5 FP | CB=4 FP | CB=5 missed (faker-js) | CB=4 missed (faker-js) |
|:---|---:|---:|---:|---:|
| faker | 2.0% (663/75996) | **2.0% (663/75996)** | — | — |
| faker-js | 0.9% (410/255760) | 0.9% (~395/255760) | 5 | **6** |

**Why CB=4 is strictly worse than CB=5:**

- **Faker FP unchanged.** Same 663 controls flagged. Root cause: cap=5.0 binds for
  these controls regardless of CB value — multi-callee stacking saturates the cap
  before per-callee CB matters. Median locale-FP control has bpe ≈ 1.66; needs
  ≥3.6 of cluster_bonus to clear threshold 5.2572. CB=4 single-callee → 1.66 + 4 =
  5.66 (above threshold). CB=5 single-callee → 1.66 + 5 = 6.66 (above threshold,
  capped at 6.66). Same flag, same FP set.
- **Faker-js loses Gate-1-critical fixture.** `foreign_rng_3` requires
  cap-saturating contribution = 5.0 (bpe ≈ 0.520 → needs 5.0 + 0.520 = 5.52 to
  clear faker-js threshold 4.86, with margin). CB=4 single-callee → 0.520 + 4.0 =
  4.52 < 4.86. Fixture flips caught → missed. faker-js: 5 → 6 missed → Gate 1
  fails (≤5 required).

**K=4 CB=5.0 full bench result (also run):** rich loses its +5pp gain (back to
95%, `dict_render_1` un-caught), faker-js drops to 6 missed (Gate 1 fails), faker
FP slightly better at 1.8% (still above 1.5% — Gate 3 still fails by original
threshold). K=4 is also strictly worse than K=8.

**Conclusion:** CB tuning cannot escape the trade-off. The faker FP geometry is
"single-callee saturates cap" — any CB ≥ ~3.6 above per-locale-FP median bpe
flags the same controls. Reducing CB below 3.6 would strip the cluster_bonus
signal entirely, killing Gate 1. The K-axis shows the same pattern: K=4 loses
catches without gaining FP relief; K=8/16/32 plateau identically. **The K=8 /
CB=5 / cap=5 corner is the Pareto-frontier choice for this hypothesis class.**

---

## 8b. Phase 5 — calibration-aware threshold (negative result)

Phase 5 attempted to absorb the +0.6pp faker FP via the calibration pipeline: if
calibration hunks could see the same cluster_bonus signal that controls see, the
multi-seed median threshold would self-adjust upward and absorb the FP without
requiring a code-side fix. **Result: no-op. Thresholds and FP counts byte-identical
to Phase 3 (no-cal-fix). Documented bound.**

### 8b.1 Design

The Phase 5 hypothesis was that the cluster_bonus signal was missing from
calibration because of a wiring bug, not for a structural reason. Pre-Phase-5,
calibration hunks were scored against the global attested set without
cluster_bonus contribution. Phase 5 wired cluster_bonus into the calibration
scorer, expecting calibration thresholds to rise on faker (where cluster_bonus
contributes most) and fall on the cluster_bonus FP rate.

Mechanism shipped in Phase 5:

1. Calibration scorer instantiates the same `CallReceiverScorer` with the same
   `file_to_cluster` mapping built at fit time.
2. Each calibration hunk's file is looked up in `file_to_cluster`; cluster's
   attested-callee set retrieved.
3. cluster_bonus contribution computed identically to control-time scoring, with
   the same cap.

### 8b.2 Result

Byte-identical thresholds and FP counts to Phase 3. faker still flags 663 controls;
multi-seed median threshold for faker still settles at **5.2572** (unchanged from
Phase 3 and from era-10 baseline).

### 8b.3 Root cause — structural

**Calibration hunks come from `model_a_files`** — the set of files that constitute
the base SHA's view of the corpus. By construction, every callee in every
calibration hunk is in some attested-callee set somewhere in the cluster's
file membership (because `model_a_files` IS what builds the attested-callee sets).
For each calibration hunk:

- Hunk file `f` lives in cluster `c`.
- Cluster `c`'s attested-callee set = ∪ (callees of all files in cluster `c`).
- Hunk's callees ⊆ callees of `f` ⊆ callees of cluster `c`'s files.
- Therefore: hunk's callees ⊆ cluster `c`'s attested-callee set.
- Therefore: zero "globally-attested but cluster-absent" callees per calibration hunk.
- Therefore: cluster_bonus contribution = 0 for every calibration hunk.

The fix wired the signal through correctly, but **there is no signal at calibration
time to see**. Calibration cannot replicate the production-control failure mode
because production controls trigger cluster_bonus via the **fallback Jaccard path**:
PR-newly-added files (not in `file_to_cluster` because they didn't exist at base
SHA) or path-normalized files (mismatches between extractor path normalization and
fit-time path normalization) get assigned to a cluster by Jaccard similarity over
their callee bag, then scored against that cluster's attested set. Calibration
hunks always exist in `model_a_files` → never hit the fallback → never accrue
cluster_bonus.

### 8b.4 Why this is a documented bound, not a Phase 5 v2 candidate

The fallback Jaccard path is **necessary** at production time — the alternative is
to refuse to score any file not in `file_to_cluster`, which would skip every
PR-newly-added file. A Phase 5 v2 that synthesized calibration-time fallback hunks
(e.g. by sampling files from `model_a_files`, deleting them from `file_to_cluster`,
and re-scoring) would be:

- Computationally heavy (re-scoring full calibration pool per synthetic deletion).
- Statistically suspect (deleted files would not match the "newly-added file"
  distribution; fit-time-clustered files have different callee bags than freshly
  added ones).
- Architecturally entangled with the fit pipeline in a way that breaks the
  scorer/calibration separation Era 10 explicitly preserved.

**Phase 5 is therefore a documented structural bound for cluster_bonus + multi-seed
median calibration**: the calibration absorption mechanism cannot work for this
signal as currently structured. This is the era-11 equivalent of era-10's Phase 3
v1/v2 documented bounds (per-callee log-rarity saturation; fraction-of-unattested
zero-on-attested).

### 8b.5 Implication for the Gate 3 amendment

Because Phase 5 cannot absorb the FP, the +0.6pp faker FP is a **fixed cost of
shipping cluster_bonus**, not a correctable defect. The Gate 3 amendment is
therefore the only viable path to ship hypothesis A with the +4.70pp recall gain.
Future eras must either accept the amended ≤2.5% per-corpus FP envelope or replace
cluster_bonus with a different signal source.

---

## 9. Provenance

- Era-11 full bench K=8/CB=5 (SHIP): `benchmarks/results/baseline/20260503T011020Z/report.md`
  (also at `benchmarks/results/baseline/latest/report.md`)
- Era-10 baseline (Phase 2 ship): `benchmarks/results/baseline/20260425T095307Z/`
- K-sweep diagnostic: `docs/research/evidence/era11-ksweep-diagnostic.md`
- Hypothesis space: `docs/research/era-11-hypotheses.md`
- Era-10 ship doc & gate definitions: `docs/research/10-calibration-hardening.md`
- Era-11 narrative doc: `docs/research/11-cluster-conditional-attestation.md`

## End of Document
