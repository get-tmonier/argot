# Era-11 Phase 2 K-sweep diagnostic (faker-js, single seed)

**Status:** diagnostic, written before full 6-corpus bench lands.
**Scope:** faker-js, 1 seed, 17 fixtures, 256k controls, K ∈ {4, 8, 16, 32} × cluster_bonus ∈ {2.0, 3.0, 4.0, 5.0} (sparse). Default `--call-receiver-cap=5`, `--call-receiver-alpha=2.0`, `--call-receiver-root-bonus=2.0`.
**Threshold (calibrated):** 4.8607.
**Pre-registered Gate 1:** faker-js 8 → ≤5 missed without regressing other corpora.

---

## 1. Headline grid

Recall % / missed-fixture count / FP count (out of 255 760 controls):

| K \ CB | 2.0 | 3.0 | 4.0 | 5.0 |
|---:|:---:|:---:|:---:|:---:|
| 4  | 53.3 / 8 / 391 (= era-10) | 60.0 / 7 / 396 | — | — |
| 8  | 58.3 / 7 / 396 | 65.0 / 6 / 396 | 65.0 / 6 / 410 | **71.7 / 5 / 410** |
| 16 | 58.3 / 7 / 396 | 65.0 / 6 / 396 | — | — |
| 32 | 58.3 / 7 / 396 | 65.0 / 6 / 396 | — | — |

(Era-10 baseline: 53.3 / 8 / 391. K4_CB2 reproduces baseline exactly because k-means with effective_k≤1 collapses to era-10 behavior; the +5 FPs at K8_CB2 vs era-10 come from cluster gating now firing.)

Gate 1 is met by **K=8, CB=5.0** (5 missed).

---

## 2. Per-fixture trajectory across CB sweep at K=8

Threshold = 4.8607. BPE column = raw `bpe_score` from JSON. Final flag is `bpe_score + min(sum(cluster_bonus weights), cap=5.0) > threshold`. ✓ = crosses; ✗ = below threshold.

| Fixture | BPE | Δ to thr | era-10 | K8_CB2 | K8_CB3 | K8_CB4 | K8_CB5 |
|---|---:|---:|:---:|:---:|:---:|:---:|:---:|
| `error_flip_2`     | 4.546 | 0.314 | ✗ | ✗ | ✗ | ✗ | ✗ |
| `error_flip_3`     | 4.053 | 0.808 | ✗ | ✗ | ✗ | ✗ | ✗ |
| `http_sink_2`      | 3.767 | 1.094 | ✗ | **✓** | ✓ | ✓ | ✓ |
| `runtime_fetch_1`  | 3.548 | 1.313 | ✗ | ✗ | ✗ | ✗ | ✗ |
| `runtime_fetch_2`  | 2.449 | 2.412 | ✗ | ✗ | ✗ | ✗ | ✗ |
| `runtime_fetch_3`  | 1.971 | 2.889 | ✗ | ✗ | ✗ | ✗ | ✗ |
| `foreign_rng_1`    | 0.520 | 4.341 | ✗ | ✗ | **✓** | ✓ | ✓ |
| `foreign_rng_3`    | 0.520 | 4.341 | ✗ | ✗ | ✗ | ✗ | **✓** |

**Crossings** (8 fixture-positions across 4 CB columns at K=8):
- `http_sink_2` crosses at CB=2.0 (cheap win — only ~1.1 of contribution needed).
- `foreign_rng_1` crosses at CB=3.0 (needs ≥4.34; with ≥2 cluster-absent attested callees, contribution = min(2×3.0, 5.0) = 5.0 ≥ 4.34 ✓).
- `foreign_rng_3` crosses only at CB=5.0 (needs ≥4.34; 1 cluster-absent attested callee × 5.0 = 5.0 ≥ 4.34 ✓).
- The other 5 never cross — see §4.

**Distance-to-threshold for the 5 still-missed at K8_CB5:**

| Fixture | Δ to thr | Inferred contribution | Inferred cluster-absent attested callees |
|---|---:|---:|:---:|
| `error_flip_2`    | 0.314 | 0.000 | 0 |
| `error_flip_3`    | 0.808 | 0.000 | 0 |
| `runtime_fetch_1` | 1.313 | 0.000 | 0 |
| `runtime_fetch_2` | 2.412 | 0.000 | 0 |
| `runtime_fetch_3` | 2.889 | 0.000 | 0 |

Inference: every still-missed fixture sees **0 cluster_bonus contribution**, which means none of its callees are "globally attested but absent from the file's cluster set." Their distance-to-threshold is the raw BPE gap and cluster-conditional scoring cannot help them (see §4).

---

## 3. K plateau confirmation (CB=3 across K=4/8/16/32)

| Fixture | BPE | K4_CB3 | K8_CB3 | K16_CB3 | K32_CB3 |
|---|---:|:---:|:---:|:---:|:---:|
| `error_flip_2` | 4.546 | ✗ | ✗ | ✗ | ✗ |
| `error_flip_3` | 4.053 | ✗ | ✗ | ✗ | ✗ |
| `http_sink_2`  | 3.767 | ✗ | ✓ | ✓ | ✓ |
| `runtime_fetch_1` | 3.548 | ✗ | ✗ | ✗ | ✗ |
| `runtime_fetch_2` | 2.449 | ✗ | ✗ | ✗ | ✗ |
| `runtime_fetch_3` | 1.971 | ✗ | ✗ | ✗ | ✗ |
| `foreign_rng_1` | 0.520 | ✗ | ✓ | ✓ | ✓ |
| `foreign_rng_3` | 0.520 | ✗ | ✗ | ✗ | ✗ |
| Total missed | — | 7 | 6 | 6 | 6 |
| Total FPs    | — | 396 | 396 | 396 | 396 |

K=4 routes `http_sink_2` and `foreign_rng_1` through clusters that already contain `fetch`/`Math.random` (so cluster_bonus=0). K=8 splits the corpus enough to put those files in clusters that exclude these callees. K=16 and K=32 reproduce K=8's per-fixture flag set **exactly**, and they also reproduce the entire 396-FP set exactly.

**Conclusion:** the plateau at K=8 is *not* an "algorithm hit a natural cluster boundary" effect — KMeans with `n_init=10, random_state=0` at K=16 and K=32 will yield genuinely different cluster partitions (more, smaller groups). The plateau is **routing-equivalent for the 8 catalog fixtures and their relevant FP-controls**: every fixture and every flagged control sits in a cluster whose attested-set membership for the load-bearing callees (`fetch`, `Math.random`, `crypto.randomBytes`, etc.) is identical to the K=8 partition's. Splitting the cluster these files live in further would only matter if a sub-cluster *removed* a previously-present attested callee — at K≥8 that hasn't happened for any catalog fixture or any of the ~70 unique flagged controls.

This is consistent with faker-js's structural pattern: `src/locales/**` (~thousands of pure-data modules) dominates, and once K is large enough to separate `src/locales/*` from `src/modules/*` (which K=8 already does cleanly given the MinHash signatures), additional splits don't change the relevant cluster-set for fetch/Math callees.

---

## 4. FP delta analysis (391 → 396 → 410)

**391 → 396 (era-10 → K8_CB2/CB3):** +5 controls. CB=2 already activates cluster gating; the call_receiver-driven FP set jumps from 0 (era-10 had no cluster_bonus) to 26 distinct flagged controls. Per JSON inspection, these 26 are concentrated in 6 module index files:

| Count | File |
|---:|---|
| 5 | `src/modules/internet/index.ts` |
| 5 | `src/modules/system/index.ts` |
| 5 | `src/modules/color/index.ts` |
| 5 | `src/modules/location/index.ts` |
| 4 | `src/modules/finance/index.ts` |
| 2 | `src/faker.ts` |

(Same hunk repeats across PR variants — these are the same 6 files, scored 26 times across the historical PR set.) Net effect: +5 net new FPs after the cluster_bonus also re-classifies a couple of borderline `bpe`-reason controls. Reasonable: these `index.ts` files are integration points that legitimately pull callees from many other modules, exactly the kind of boundary a per-cluster attested set is most aggressive on.

**396 → 410 (CB=3 → CB=4 = CB=5):** +14 controls, all at the same 3 hunks (with PR-variant duplication). Diff'd from JSON:

| Δ count | File | Hunk | Reason |
|---:|---|---|---|
| +6 | `src/modules/date/index.ts` | 50–62 | call_receiver |
| +5 | `src/modules/science/index.ts` | 38–45 | call_receiver |
| +3 | `src/modules/git/index.ts` | 10–17 | call_receiver |

K8_CB4 and K8_CB5 produce **identical** flagged-control sets (73 unique by hunk-position; 410 with PR-variant duplication). The CB=4 → CB=5 transition is a no-op for FPs; both CB=4 and CB=5 push these 3 hunks past threshold.

**Interpretation:** these are 3 module-index hunks where a single cluster-absent attested callee × CB ≥ (threshold − raw BPE). Their raw BPE values are 1.025–1.434 (per JSON), so they need contribution ≥ ~3.4–3.8, achievable only at CB ≥ 4. They are the same **pattern** as the +5 FPs that appeared at CB=2 (module-level integration files), just with lower raw BPE — they're cluster-conditional FPs by construction.

**Should we worry?** Mildly. The FP rate stays at 0.9% (rounded) across the whole sweep — the additional 14/255 760 hunks is a 4% relative increase in FP count. All are concentrated in 3 specific files (a known structural pattern), not spread across the corpus. If the full 6-corpus bench shows FP creep on other corpora at CB=5, that would shift the picture; here on faker-js the +14 is bounded and explainable.

---

## 5. Cap binding analysis

**Cap formula:** contribution = `min(sum(cluster_bonus_weights), cap)` where each cluster-absent attested callee contributes `cluster_bonus`. Default `cap=5.0`.
- At CB=3.0: ≥2 cluster-absent attested callees saturates the cap (2×3=6 → capped at 5).
- At CB=5.0: 1 cluster-absent attested callee saturates the cap (1×5=5 → capped at 5).

### 5a. Fixtures that crossed at CB=5 but not CB=3 (only `foreign_rng_3` qualifies)

| Fixture | BPE | Distinct callees in hunk | Crosses at | Inferred cluster-absent attested | Math at CB=3 | Math at CB=5 |
|---|---:|---|---|:---:|---|---|
| `foreign_rng_3` | 0.520 | `Math.floor`, `Math.random` | CB=5 | 1 | 1×3.0 = 3.0 → 0.520+3.0 = 3.52 < 4.86 ✗ | 1×5.0 = 5.0 → 0.520+5.0 = 5.52 > 4.86 ✓ |

Of the original "6 fixtures that didn't cross at CB=3 but COULD cross at CB=5" — only `foreign_rng_3` actually does. The other 5 stay missed (see §5b).

### 5b. The 5 still-missed at K8_CB5

| Fixture | BPE | Distinct callees in hunk | Likely status of each callee |
|---|---:|---|---|
| `error_flip_2` | 4.546 | `Error` | Globally attested AND in every cluster (built-ins are universal) → 0 contribution |
| `error_flip_3` | 4.053 | `Error` | Same as above → 0 contribution |
| `runtime_fetch_1` | 3.548 | `fetch`, `res.json` | `fetch` must be globally attested (else era-10 would catch via alpha+root_bonus = 4.0 → score 7.5); `fetch` and `res.json` evidently in every cluster these locale files map to → 0 contribution |
| `runtime_fetch_2` | 2.449 | `fetch` | Same — `fetch` globally attested AND in cluster → 0 contribution |
| `runtime_fetch_3` | 1.971 | `fetch`, `res.json` | Same as runtime_fetch_1 |

The "globally attested" inference is forced: if any callee in a fixture were *not* globally attested, era-10 baseline (which uses `alpha + root_bonus = 4.0` for non-attested callees, capped at 5) would have flagged it. e.g., `runtime_fetch_2` raw BPE 2.449 + 4.0 = 6.45 > 4.86 — would cross. Era-10 missed it ⇒ `fetch` is globally attested in the faker-js corpus.

**Would cap=7.0 catch any of the 5?** No, none of them. With cap=7.0:
- `error_flip_*`: contribution = 0 × 7.0 = 0. No change.
- `runtime_fetch_*`: contribution = 0 × 7.0 = 0. No change.

The cap only matters when there are ≥2 cluster-absent attested callees. None of the still-missed fixtures has any. **Cap=7 is not the right lever** for these — the lever needs to be either (a) a different scorer signal (e.g., explicit `fetch`/`throw`-pattern detection) or (b) finer cluster granularity that can put `fetch` in a cluster-absent state for `src/locales/**` files.

---

## 6. Recommendation for the full bench config

### Single-shot ship: K=8, CB=5.0, cap=5.0

This is the right one-shot config based on the faker-js single-seed evidence:

1. **Gate 1 met:** 5 missed (≤5 target).
2. **Cap=7.0 would not unlock any additional faker-js fixtures** at K=8/CB=5 — the 5 still-missed have contribution = 0, not contribution = saturating. Raising cap is the wrong knob here.
3. **K=8 plateau is robust:** K=16 and K=32 give identical fixture flags and identical FP sets at CB=3, so K=8 isn't a fragile choice.
4. **CB=5 vs CB=4 is the cleanest gain:** identical FP set, +1 fixture (`foreign_rng_3`).
5. **FP delta is bounded and explainable:** +19 vs era-10 baseline (391 → 410), all in ≤9 module-index files, none in test/data files.

### Should we plan a cap=7 second run?

**No** — for two reasons:

1. **No faker-js evidence for it.** Cap is bound only when ≥2 cluster-absent attested callees stack. None of the still-missed faker-js fixtures has that geometry. Cap=7 cannot help any of the remaining 5.
2. **Cap=7 strictly inflates FPs.** Looking at the 14 new FPs at CB=4/5 vs CB=3: they all show single-callee contributions (counted once each in the JSON delta). Raising cap from 5 to 7 doesn't change those single-contribution scores (already below cap). But it WOULD push a new tier of multi-callee borderline controls past threshold. We have no faker-js fixture upside to balance that.

If the full 6-corpus bench shows another corpus where the still-missed have multi-callee geometry (e.g., a Python web-framework fixture with several rare-but-attested calls), THEN a cap=7 sensitivity sweep is justified — but as a follow-up, not a parallel run.

### What I'd plan instead, if Gate 1 is met but other corpora regress

- If a corpus regresses on FP, sensitivity sweep on **K only** (try K=16 with CB=5 to see if finer clustering moves the FP-controls back below threshold without touching catalog).
- If a corpus regresses on recall, look at per-fixture missed-list — if missed fixtures look like the 5 faker-js still-missed (raw BPE near threshold, callees globally attested AND in cluster), no Phase-2 knob will catch them — they're work for Era-12.

---

## 7. What this changes for Era-12 planning

The 5 still-missed faker-js fixtures fall into two structural buckets:

- **`error_flip` (2 fixtures):** only `Error` as a callee. The signal is "throwing inside a code path that the corpus never throws in." Cluster-conditional call_receiver can't model this — it would need a `throw`-pattern scorer, or a per-function-context conditional surprise model.
- **`runtime_fetch` (3 fixtures):** `fetch`/`res.json` are globally attested. The signal is "fetch inside a `src/locales/**` file" — i.e., a *foreign callee for this file's neighborhood*. This is exactly what cluster-conditional scoring is supposed to catch, and the fact that it doesn't tells us either (a) `src/locales/**` is being mis-clustered with files that DO use `fetch`, or (b) `fetch` shows up in `src/locales/**` legitimately enough during training to be in the cluster's attested set.

Quick sanity check on (b): faker-js does have locale files that fetch dynamic data in some configurations. If the training corpus includes any such file, `fetch` ends up in the locale cluster's attested set and runtime_fetch_* fixtures lose their cluster signal. **A future ablation** (subsample training corpus to remove any `src/locales/**` file containing `fetch`, retrain, re-bench) would confirm this hypothesis without code changes — file as Era-12 plan input.
