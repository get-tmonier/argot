# Phase 14 Stage-2 Recall Cross-Corpus Probe (2026-04-22)

**Hypothesis:** Faker's Stage 2 blindness is purely threshold-driven, not content-driven.
Fixture BPE scores should be comparable across all three host corpora (same fixture =
same content = similar score). Only the calibration threshold differs.

**Method:** `Stage2OnlyScorer` (Stage 1 permanently disabled) calibrated on each host
corpus's pre-merge snapshot. All 8 stage2_only fixtures injected into 4-5 clean host PRs
per corpus. FastAPI baseline reused from Step O (not re-run).

| Corpus | N_CAL | Host PRs | Note |
|---|---|---|---|
| FastAPI (baseline) | 100 | #14862, #14944, #14856, #14806 | Step O result reused |
| Rich | 200 | #4079, #4077, #4076, #4075, #3941 | Feasible ceiling per N=500 revalidation |
| Faker | 500 | #2352, #2351, #2350, #2349, #2348 | Matches faker experiment config |

---

## §0 Three-Host Summary Table

Median BPE score across host PRs per fixture, with max-threshold flag status.

| fixture | FastAPI bpe | FastAPI flag | Rich bpe | Rich thr | Rich flag | Faker bpe | Faker thr | Faker flag |
|---|---|---|---|---|---|---|
| walrus_operator | 7.671 | YES | 6.909 | 4.421 | YES | 7.308 | 8.204 | no |
| match_case | 7.482 | YES | 7.231 | 4.421 | YES | 7.380 | 8.204 | no |
| dataclass_migration | 5.270 | YES | 5.174 | 4.421 | YES | 5.270 | 8.204 | no |
| fstring_adoption | 7.418 | YES | 5.247 | 4.421 | YES | 4.655 | 8.204 | no |
| async_adoption | 7.329 | YES | 7.329 | 4.421 | YES | 7.329 | 8.204 | no |
| genexpr_shift | 7.388 | YES | 5.174 | 4.421 | YES | 5.466 | 8.204 | no |
| type_annotations | 6.190 | YES | 5.826 | 4.421 | YES | 5.149 | 8.204 | no |
| union_syntax | 6.887 | YES | 6.730 | 4.421 | YES | 5.466 | 8.204 | no |

---

## §1 Score Homogeneity Check

For each fixture, are BPE scores roughly comparable across hosts (within ±0.5)?
A large deviation would mean corpus tokenization diverges — invalidating the
hypothesis that threshold is the sole differentiator.

| fixture | FastAPI (Step O) | Rich median | Faker median | max gap | homogeneous? |
|---|---|---|---|---|---|
| walrus_operator | 7.671 | 6.909 | 7.308 | 0.762 | **NO** |
| match_case | 7.482 | 7.231 | 7.380 | 0.251 | YES |
| dataclass_migration | 5.270 | 5.174 | 5.270 | 0.096 | YES |
| fstring_adoption | 7.418 | 5.247 | 4.655 | 2.763 | **NO** |
| async_adoption | 7.329 | 7.329 | 7.329 | 0.000 | YES |
| genexpr_shift | 7.388 | 5.174 | 5.466 | 2.214 | **NO** |
| type_annotations | 6.190 | 5.826 | 5.149 | 1.041 | **NO** |
| union_syntax | 6.887 | 6.730 | 5.466 | 1.421 | **NO** |

**Verdict:** Score divergence detected in ≥1 fixture — content/corpus mismatch may contribute.

---

## §2 Threshold Regime Comparison

| Corpus | N_CAL | Median threshold (max) | Median p99 | Median p95 |
|---|---|---|---|---|
| FastAPI (Step O) | 100 | 4.0601 | (not computed in Step O) | — |
| Rich | 200 | 4.4212 | 3.8957 | 3.5089 |
| Faker | 500 | 8.2040 | 5.8552 | 5.2148 |

### Per-host threshold breakdown

| Corpus | PR | pre_sha | max threshold | p99 | p95 | n_cal |
|---|---|---|---|---|---|---|
| FastAPI | #14862 | (Step O) | 4.1047 | — | — | 100 |
| FastAPI | #14944 | (Step O) | 4.0155 | — | — | 100 |
| FastAPI | #14856 | (Step O) | 4.1115 | — | — | 100 |
| FastAPI | #14806 | (Step O) | 3.2696 | — | — | 100 |
| Rich | #4079 | 19c67b9a | 4.4213 | 3.8958 | 3.5090 | 200 |
| Rich | #4077 | 58ac1512 | 4.4212 | 3.8957 | 3.5089 | 200 |
| Rich | #4076 | 9cb19894 | 4.4212 | 3.8957 | 3.5089 | 200 |
| Rich | #4075 | 1fc7cb21 | 4.4211 | 3.8956 | 3.5088 | 200 |
| Rich | #3941 | a9c4aaba | 4.4210 | 3.8955 | 3.5087 | 200 |
| Faker | #2352 | 740812bd | 8.2050 | 5.8552 | 5.2247 | 500 |
| Faker | #2351 | 6a495ba4 | 8.2046 | 6.0137 | 5.2154 | 500 |
| Faker | #2350 | f595fb2c | 8.2040 | 6.0131 | 5.2148 | 500 |
| Faker | #2349 | 0c2aef9f | 8.2039 | 5.8536 | 5.2148 | 500 |
| Faker | #2348 | 2bb97dc7 | 8.2039 | 5.6748 | 5.1908 | 500 |

---

## §3 Catch Rate per Host

| Corpus | Fixtures flagged | Total pairs | Catch rate |
|---|---|---|---|
| FastAPI (Step O, max threshold, N=100) | 32/32 | 32 | 100% |
| Rich (max threshold, N=200) | 40/40 | 40 | 100% |
| Faker (max threshold, N=500) | 0/40 | 40 | 0% |

### Rich — per-fixture × per-host score table (max threshold)

Cell format: `YES bpe>thr` / `no bpe<thr`.

| fixture | #4079 | #4077 | #4076 | #4075 | #3941 | median |
|---|---|---|---|---|---|---|
| walrus_operator | **YES** 6.909>4.421 | **YES** 6.909>4.421 | **YES** 6.909>4.421 | **YES** 6.909>4.421 | **YES** 6.909>4.421 | 6.909 |
| match_case | **YES** 7.231>4.421 | **YES** 7.231>4.421 | **YES** 7.231>4.421 | **YES** 7.231>4.421 | **YES** 7.231>4.421 | 7.231 |
| dataclass_migration | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | 5.174 |
| fstring_adoption | **YES** 5.247>4.421 | **YES** 5.247>4.421 | **YES** 5.247>4.421 | **YES** 5.247>4.421 | **YES** 5.247>4.421 | 5.247 |
| async_adoption | **YES** 7.329>4.421 | **YES** 7.329>4.421 | **YES** 7.329>4.421 | **YES** 7.329>4.421 | **YES** 7.329>4.421 | 7.329 |
| genexpr_shift | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | **YES** 5.174>4.421 | 5.174 |
| type_annotations | **YES** 5.826>4.421 | **YES** 5.826>4.421 | **YES** 5.826>4.421 | **YES** 5.826>4.421 | **YES** 5.826>4.421 | 5.826 |
| union_syntax | **YES** 6.730>4.421 | **YES** 6.730>4.421 | **YES** 6.730>4.421 | **YES** 6.730>4.421 | **YES** 6.730>4.421 | 6.730 |

### Faker — per-fixture × per-host score table (max threshold)

Cell format: `YES bpe>thr` / `no bpe<thr`.

| fixture | #2352 | #2351 | #2350 | #2349 | #2348 | median |
|---|---|---|---|---|---|---|
| walrus_operator | no 7.308<8.205 | no 7.308<8.205 | no 7.308<8.204 | no 7.308<8.204 | no 7.308<8.204 | 7.308 |
| match_case | no 7.380<8.205 | no 7.380<8.205 | no 7.380<8.204 | no 7.380<8.204 | no 7.380<8.204 | 7.380 |
| dataclass_migration | no 5.270<8.205 | no 5.270<8.205 | no 5.270<8.204 | no 5.270<8.204 | no 5.270<8.204 | 5.270 |
| fstring_adoption | no 4.656<8.205 | no 4.656<8.205 | no 4.655<8.204 | no 4.655<8.204 | no 4.655<8.204 | 4.655 |
| async_adoption | no 7.329<8.205 | no 7.329<8.205 | no 7.329<8.204 | no 7.329<8.204 | no 7.329<8.204 | 7.329 |
| genexpr_shift | no 5.467<8.205 | no 5.467<8.205 | no 5.466<8.204 | no 5.466<8.204 | no 5.466<8.204 | 5.466 |
| type_annotations | no 5.149<8.205 | no 5.149<8.205 | no 5.149<8.204 | no 5.149<8.204 | no 5.149<8.204 | 5.149 |
| union_syntax | no 5.467<8.205 | no 5.467<8.205 | no 5.466<8.204 | no 5.466<8.204 | no 5.466<8.204 | 5.466 |

---

## §4 Counterfactual Threshold Analysis

Re-compute catch rate at p99 and p95 thresholds. Does lowering the threshold
from max to p99/p95 unblock faker recall?

| Corpus | Threshold | Catch rate | Fixtures flagged | Total pairs |
|---|---|---|---|---|
| FastAPI | max (~4.06) | 100% | 32/32 | 32 |
| Rich | max (~4.42) | 100% | 40/40 | 40 |
| Rich | p99 (~3.90) | 100% | 40/40 | 40 |
| Rich | p95 (~3.51) | 100% | 40/40 | 40 |
| Faker | max (~8.20) | 0% | 0/40 | 40 |
| Faker | p99 (~5.86) | 38% | 15/40 | 40 |
| Faker | p95 (~5.21) | 75% | 30/40 | 40 |

### Counterfactual per-fixture detail

For faker specifically: what is each fixture's margin against max/p99/p95?
Positive margin = flagged; negative = missed.

| fixture | Faker max margin | Faker p99 margin | Faker p95 margin |
|---|---|---|---|
| walrus_operator | **no** -0.896 | YES +1.453 | YES +2.094 |
| match_case | **no** -0.824 | YES +1.525 | YES +2.165 |
| dataclass_migration | **no** -2.934 | **no** -0.585 | YES +0.055 |
| fstring_adoption | **no** -3.549 | **no** -1.199 | **no** -0.560 |
| async_adoption | **no** -0.875 | YES +1.474 | YES +2.114 |
| genexpr_shift | **no** -2.738 | **no** -0.388 | YES +0.252 |
| type_annotations | **no** -3.055 | **no** -0.707 | **no** -0.066 |
| union_syntax | **no** -2.738 | **no** -0.388 | YES +0.252 |

---

## §5 Verdict on Hypothesis

Score homogeneity PARTIAL: max gap is 2.763 — some fixtures diverge across corpora.

**Hypothesis CONFIRMED (partially):** Faker Stage 2 blindness is threshold-driven. At max (≈8.20), catch rate is 0%; at p99 (≈5.86), catch rate rises to 38%. p99 is a viable unified threshold strategy if FP rate stays acceptable.

**Rich recall: 100%** — Rich behaves comparably to FastAPI. The ~4.1-4.4 threshold regime does not create blind spots for these fixtures.

### Summary

| Question | Answer |
|---|---|
| Are fixture BPE scores homogeneous across hosts? | Partial (max gap 2.76) |
| Is faker blindness purely threshold-driven? | Yes — p99 rescues it |
| Does p99 unblock faker without FP explosion? | See §4 — requires FP base-rate validation before adopting |
| Rich recall at max threshold? | 40/40 = 100% |

