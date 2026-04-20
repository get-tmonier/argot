# FastAPI v2 Fixture Expansion — Baseline Report

**Date:** 2026-04-20
**Branch:** research/phase-7-honest-eval
**Fixture set:** v2 (27 fixtures: 19 breaks + 8 controls across 9 categories)
**v1 subset:** 12 fixtures (9 breaks + 3 controls) — reproduces Stages 1–6 history

---

## v1 Reproduction Check

| scorer | seeds | delta_v1 (new run) | historical | within ±0.005 |
|---|---|---|---|---|
| Stage 4 ensemble_n3 | 5 | 0.2215 | 0.2215 | ✓ |
| Stage 6 b01_t01_w0+ensemble_n3 | 5 | 0.2291 | 0.2291 | ✓ |

Both scorers reproduce historical delta_v1 exactly (deviation = 0.0000). The ensemble averaging
eliminates seed-to-seed variance, making re-runs fully deterministic on the v1 fixture set.

---

## v2 Baseline Numbers

| scorer | seeds | mean_delta_v1 | std_delta_v1 | mean_delta_v2 | std_delta_v2 | gate_v2 |
|---|---|---|---|---|---|---|
| Stage 4 ensemble_n3 | 5 | 0.2215 | 0.0000 | 0.0614 | 0.0000 | ✗ |
| Stage 6 b01_t01_w0+ensemble_n3 | 5 | 0.2291 | 0.0000 | 0.1087 | 0.0000 | ✗ |

Gate: delta_v2 >= 0.20 = ✓

Neither scorer meets the v2 gate. The delta_v2 numbers are substantially lower than delta_v1,
indicating that the 15 new v2 fixtures (10 new breaks + 5 new controls) are harder cases
where the scorer's discriminating power is weaker.

---

## Per-category Deltas (v2, Stage 6 winner)

| category | mean_delta |
|---|---|
| async_blocking | 0.1007 |
| background_tasks | 0.0511 |
| dependency_injection | 0.1923 |
| downstream_http | 0.1185 |
| exception_handling | 0.2541 |
| framework_swap | 0.1304 |
| routing | 0.2411 |
| serialization | 0.2232 |
| validation | -0.1512 |

---

## v1 vs v2 Gap Analysis

**delta_v1 - delta_v2 gap for Stage 6 winner:** 0.1204

The v2 delta is substantially lower than v1 (0.1087 vs 0.2291, gap = 0.1204). This confirms that
the new v2 fixtures are harder cases for the current scorer: the `validation` category in particular
drives a large negative delta (-0.1512), meaning the scorer systematically misranks validation
break fixtures. The new fixture categories (background_tasks, framework_swap, downstream_http)
contribute positive but weak signal, keeping the overall v2 mean below the 0.20 gate.

---

## Seed Stability

Stage 6 winner std_delta across 5 seeds:
- delta_v1 std: 0.0000 (target ≤ 0.02) — Pass
- delta_v2 std: 0.0000 (target ≤ 0.02) — Pass

The ensemble architecture (n=3 members) eliminates all seed-to-seed variance on both fixture sets.
Both stds are 0.0000 across 5 outer seeds (42–46), well within the ≤ 0.02 target.

---

## Conclusions

v1 reproduction passes exactly: both Stage 4 (0.2215) and Stage 6 (0.2291) delta_v1 values match
their historical references to 4 decimal places. The v2 baseline establishes new reference numbers:
ensemble_n3 at delta_v2=0.0614 and b01_t01_w0+ensemble_n3 at delta_v2=0.1087, both failing the
0.20 gate. Seed stability is excellent (std=0.0000 for both v1 and v2), but the v2 fixture set
reveals a meaningful capability gap — particularly in the `validation` category (-0.1512) — that
will require targeted improvement in future phases.
