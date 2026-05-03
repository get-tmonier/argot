# Era 12 Phase 10 — Cluster-rare attestation (deferred)

**Date**: 2026-05-03
**Branch**: `feat/era-12-ml-stage`
**Status**: plumbed + unit-tested (5 tests), bench-inert under current
calibration. Default `cluster_rare_threshold = 0` (off). Future work.

---

## TL;DR

Era 11's cluster-conditional attestation marks a callee as cluster-
absent (firing `cluster_bonus`) only when it's missing from the cluster's
attested-set union. A callee attested in ONE file of a 63-file cluster
is treated identically to one attested in all 63. Phase 10 generalises
this: a callee attested in ≤ N cluster files (configurable threshold)
is treated as effectively cluster-absent, so `cluster_bonus` fires.

**Mechanism is correct** (5 unit tests pass; standalone calibration
probe shows threshold delta = exactly 5.0 with `cluster_rare_threshold=2`
when the cal sample contains a rare-attested hunk).

**Bench-inert** because `cluster_bonus` fires on calibration hunks too,
inflating the threshold by the same amount it would have lifted any
fixture. Under `max(cal_scores)` thresholding, the inflation cancels
the gain. Net effect on faker-js: 0 marginal catches over the routing-
fix-only baseline.

The infrastructure ships dormant (default off, no behavioural change)
to be picked up by a follow-up that addresses the calibration
interaction.

---

## Motivation

After the routing fix (see [`era12-routing-fix.md`](era12-routing-fix.md))
recovered most of era 11's "missing" residuals on faker-js, four
fixtures stayed uncaught with the same diagnostic shape:

| Fixture | Callee | Cluster (size) | Count in cluster | Why era-11 misses |
|---|---|---|---:|---|
| `foreign_rng_1` | `Math.random` | fjs cluster 2 (63) | 1 | attested → no cluster_bonus |
| `http_sink_2` | `fetch` | fjs cluster 2 (63) | 1 | attested → no cluster_bonus |
| `error_flip_2` | `Error` | fjs cluster 6 (24) | 1 | attested → no cluster_bonus |

These callees are **technically present** in the host file's cluster (in
1 of 63 / 1 of 24 cluster files) but functionally absent for the role
the host file plays. Era 11's boolean union treats "present in 1 file"
the same as "present in 63 files." That binary collapse is the gap.

Across all six corpora the same shape applies to several other fixtures:
`fastapi/validation_2` (`app.patch` / `app.delete` in 1–2 of 289 cluster
files), `fastapi/exception_handling_4` (`app.put`, `app.delete` in 1–2 of
289), `hono/hono_validation_2` (`c.json` in 3 of 99). All would benefit
from a "rare-but-attested = cluster-absent" rule.

---

## Design

`CallReceiverScorer` gains:

- `cluster_callee_counts: dict[int, dict[str, int]]` — per cluster, the
  number of cluster files containing each callee. Built alongside
  `cluster_attested` in `_build_clusters`.
- `cluster_sizes: dict[int, int]` — per cluster, the number of files.
- `cluster_rare_threshold: int = 0` — constructor parameter, opt-in.

Scoring rule extended (in `weighted_contribution_for_file`):

```
for c in callees (distinct, after dedup):
  if c not in attested AND root in attested_roots: alpha + root_bonus
  if c not in attested:                            alpha
  if c in attested AND c not in cluster_set:       cluster_bonus       # existing
  if (cluster_rare_threshold > 0
      AND cluster_counts[c] <= cluster_rare_threshold):
                                                    cluster_bonus       # NEW
  if c in attested AND c in cluster_set:           0
```

`threshold = 0` preserves the existing boolean behaviour exactly.
`threshold = 2` says "callees in at most 2 cluster files don't really
count as attested for this cluster's purposes."

Plumbed through:

- `engine/argot/scoring/scorers/call_receiver.py` (build + scoring)
- `engine/argot/scoring/scorers/sequential_import_bpe.py` (constructor pass-through)
- `engine/argot/scoring/calibration/__init__.py` (multi-seed calibration pass-through)
- `engine/argot/check.py` (read from scorer-config.json)
- `benchmarks/src/argot_bench/score.py` (`build_scorer` parameter)
- `benchmarks/src/argot_bench/run.py` (RunConfig field)
- `benchmarks/src/argot_bench/cli.py` (`--call-receiver-cluster-rare-threshold N` flag)

---

## Tests (5 in `engine/argot/tests/test_call_receiver_clustering.py`)

- `test_cluster_rare_threshold_default_zero_preserves_era11` — threshold=0
  is byte-identical to the boolean-union behaviour.
- `test_cluster_rare_threshold_fires_when_callee_in_few_files` — with
  threshold=2 and a callee in 1 cluster file, scoring returns
  `cluster_bonus` (5.0).
- `test_cluster_rare_threshold_does_not_fire_for_common_callees` — a
  callee in many cluster files does NOT fire.
- `test_cluster_rare_threshold_preserves_unattested_path` — unattested
  callees still get alpha or alpha+root_bonus, not cluster_bonus.
- `test_cluster_callee_counts_dict_populated` — counts dict is populated
  iff `n_clusters > 1`.

All 5 pass.

---

## Diagnosis: why bench-inert despite correct mechanism

Standalone probe (extracted from `_phase10_calibration_probe.py`,
removed after diagnosis): with the same model_a_files + cluster config
the bench uses, calling `calibrate_multi_seed(call_receiver_cluster_rare_threshold=2)`
returns threshold = **8.94** versus threshold = **3.94** with
`call_receiver_cluster_rare_threshold=0`. **Delta = exactly 5.0** =
`cluster_bonus`.

That confirms the mechanism fires: at least one calibration hunk has a
rare-attested callee, gets `cluster_bonus = 5.0` added to its score, and
becomes the new max(cal_scores).

Under `_compute_threshold(cal_scores, threshold_percentile=None)` →
`max(cal_scores)`, the threshold rises by exactly the cluster_bonus.

A fixture hunk with the same rare-attested callee gets the same
+5.0 contribution. So:

```
adjusted_bpe (fixture) = bpe_score + 5.0   # rare-threshold fires
threshold              = max(cal_score)    # also +5.0 from cal hunk
                                             firing
```

The two move in lockstep. The fixture's adjusted_bpe is exactly the
same distance above (or below) the threshold as it would have been
without the rare-threshold rule. Net catch impact: zero.

The full bench (5 outer seeds × 7 inner = 35 calibrations) showed
threshold_mean unchanged between rare=0 and rare=2 (4.86 in both),
which is consistent with the median across calibrations smoothing out
the inflation, OR with the bench's specific cal samples not hitting
rare-attested hunks. Either way, the result is the same: **no
fixture gains a flag from rare-threshold under the current calibration
geometry**.

---

## What would unblock Phase 10

The threshold-inflation cancellation is fundamental to `max(cal_scores)`
thresholding combined with a contribution that fires symmetrically on
calibration and fixture hunks. To make Phase 10 deliver, the calibration
side has to absorb the rare-threshold contribution differently from
the fixture side.

Options to investigate:

1. **Switch the threshold percentile.** With `threshold_percentile=95.0`
   instead of None (max), a single inflated calibration score doesn't
   set the threshold; only the bulk of the distribution does. If
   rare-threshold firing is sparse (<5% of cal hunks), it stops
   inflating the threshold but still fires for fixtures. The downside:
   p95 thresholding is a more general policy change with broader
   side-effects on FP rates.
2. **Asymmetric calibration.** Compute calibration scores with
   `cluster_rare_threshold=0` (preserve current threshold) but score
   fixtures with `cluster_rare_threshold=2`. Hard to defend
   philosophically — "calibrate one way, score another" is a smell.
3. **Cap the cluster_bonus contribution at calibration time only.**
   E.g. cluster_bonus during calibration = 0; during fixture scoring =
   5. Same smell as (2).
4. **Change the rule to fire only on extreme-rare cases.** A "callee
   appears in exactly 1 of N cluster files for N > 20" rule (vs
   "appears in ≤ 2") would catch faker-js's clean cases (`Math.random` in
   1/63, `fetch` in 1/63) while not firing on faker-js's small cluster 6
   (24 files) where most attested callees count 1. That asymmetry
   between cluster sizes might give fixtures more leverage than cal
   hunks. Speculative.
5. **Drop the rule entirely** and instead address the same fixtures
   via control-flow / AST-shape features (the era-15 candidate from
   the closure status memo). For `error_flip_*` this is necessary
   anyway because `Error` is too common a callee for any
   frequency-based rule to surface cleanly.

Honest read: option (1) is the cheapest test (~1 hour to swap
threshold_percentile in calibration config and re-bench). Option (5)
is the right long-term direction.

---

## Surface area cost of leaving Phase 10 in

The infrastructure adds:

- 2 new attributes on `CallReceiverScorer` (`cluster_callee_counts`,
  `cluster_sizes`) — populated whenever clusters are built.
  Memory: O(cluster_count × distinct_callees_per_cluster), well-bounded.
- 1 new constructor parameter (`cluster_rare_threshold`, default 0).
- 1 new `elif` branch in `weighted_contribution_for_file` — gated on
  `cluster_rare_threshold > 0`, so default-config paths are unaffected.
- Plumbing through `SequentialImportBpeScorer`, `calibrate_multi_seed`,
  `build_scorer`, `RunConfig`, `cli.py`. Each just forwards the
  parameter.
- 5 unit tests + bench CLI flag.

Net: ~80 lines of plumbing + 5 tests, all behind a default-zero gate.
No production behaviour change. Cost-of-keeping is low; cost-of-
removing-and-re-adding-later is also low. Choosing to keep on the
"infrastructure ready when the calibration question is answered"
basis.

---

## Provenance

- Implementation: `engine/argot/scoring/scorers/call_receiver.py` (search for `cluster_rare_threshold`)
- Tests: `engine/argot/tests/test_call_receiver_clustering.py` (search for `cluster_rare`)
- CLI flag: `benchmarks/src/argot_bench/cli.py` (`--call-receiver-cluster-rare-threshold`)
- Bench inertness measured against `benchmarks/results/20260503T123914Z` (rare=2) vs `20260503T122355Z` (rare=0). Identical thresholds, identical catches.
- Standalone probe (since deleted): `calibrate_multi_seed(call_receiver_cluster_rare_threshold=2)` returns +5.0 threshold delta on a small cal sample, confirming the mechanism fires at the calibration layer.
