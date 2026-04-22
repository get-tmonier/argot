# Phase 14 Stage-2 Recall Cross-Corpus Probe — fix9 (2026-04-22)

**fix9 change:** Data-dominant files excluded from model A training corpus
(SequentialImportBpeScorer default exclude_data_dominant=True).
Expected: faker calibration floor drops from ~7.15 → ~4-5; catch rate rises from 38% → ≥80%.

**Method:** `Stage2OnlyScorer` (Stage 1 permanently disabled) calibrated on each host
corpus's pre-merge snapshot. All 8 stage2_only fixtures injected into 4-5 clean host PRs
per corpus. FastAPI baseline reused from Step O (not re-run).

| Corpus | N_CAL | Host PRs | Note |
|---|---|---|---|
| FastAPI (baseline) | 100 | #14862, #14944, #14856, #14806 | Step O result reused |
| Rich | 200 | #4079, #4077, #4076, #4075, #3941 | Regression check — expect same as fix8 |
| Faker | 250 | #2352, #2351, #2350, #2349, #2348 | Primary fix9 result |

---

## §0 Three-Host Summary Table

Median BPE score across host PRs per fixture, with max-threshold flag status.

| fixture | FastAPI bpe | FastAPI flag | Rich bpe | Rich thr | Rich flag | Faker bpe | Faker thr | Faker flag |
|---|---|---|---|---|---|---|
| walrus_operator | 7.671 | YES | 6.909 | 4.203 | YES | 7.308 | 5.325 | YES |
| match_case | 7.482 | YES | 7.231 | 4.203 | YES | 7.380 | 5.325 | YES |
| dataclass_migration | 5.270 | YES | 5.174 | 4.203 | YES | 5.270 | 5.325 | no |
| fstring_adoption | 7.418 | YES | 5.247 | 4.203 | YES | 7.418 | 5.325 | YES |
| async_adoption | 7.329 | YES | 7.329 | 4.203 | YES | 7.329 | 5.325 | YES |
| genexpr_shift | 7.388 | YES | 5.174 | 4.203 | YES | 6.204 | 5.325 | YES |
| type_annotations | 6.190 | YES | 5.826 | 4.203 | YES | 6.318 | 5.325 | YES |
| union_syntax | 6.887 | YES | 6.730 | 4.203 | YES | 7.264 | 5.325 | YES |

---

## §1 Score Homogeneity Check

For each fixture, are BPE scores roughly comparable across hosts (within ±0.5)?

| fixture | FastAPI (Step O) | Rich median | Faker median | max gap | homogeneous? |
|---|---|---|---|---|---|
| walrus_operator | 7.671 | 6.909 | 7.308 | 0.762 | **NO** |
| match_case | 7.482 | 7.231 | 7.380 | 0.251 | YES |
| dataclass_migration | 5.270 | 5.174 | 5.270 | 0.096 | YES |
| fstring_adoption | 7.418 | 5.247 | 7.418 | 2.171 | **NO** |
| async_adoption | 7.329 | 7.329 | 7.329 | 0.000 | YES |
| genexpr_shift | 7.388 | 5.174 | 6.204 | 2.214 | **NO** |
| type_annotations | 6.190 | 5.826 | 6.318 | 0.492 | YES |
| union_syntax | 6.887 | 6.730 | 7.264 | 0.535 | **NO** |

**Verdict:** Score divergence detected in ≥1 fixture — content/corpus mismatch may contribute.

---

## §2 Threshold Regime Comparison

| Corpus | N_CAL | Median threshold (max) | Median p99 | Median p95 |
|---|---|---|---|---|
| FastAPI (Step O) | 100 | 4.0601 | (not computed in Step O) | — |
| Rich | 200 | 4.2032 | 3.6739 | 3.2860 |
| Faker | 250 | 5.3251 | 4.6022 | 3.6693 |

### Per-host threshold breakdown

| Corpus | PR | pre_sha | max threshold | p99 | p95 | n_cal |
|---|---|---|---|---|---|---|
| FastAPI | #14862 | (Step O) | 4.1047 | — | — | 100 |
| FastAPI | #14944 | (Step O) | 4.0155 | — | — | 100 |
| FastAPI | #14856 | (Step O) | 4.1115 | — | — | 100 |
| FastAPI | #14806 | (Step O) | 3.2696 | — | — | 100 |
| Rich | #4079 | 19c67b9a | 4.2033 | 3.6740 | 3.2861 | 200 |
| Rich | #4077 | 58ac1512 | 4.2032 | 3.6739 | 3.2860 | 200 |
| Rich | #4076 | 9cb19894 | 4.2032 | 3.6739 | 3.2860 | 200 |
| Rich | #4075 | 1fc7cb21 | 4.2031 | 3.6738 | 3.2859 | 200 |
| Rich | #3941 | a9c4aaba | 4.2029 | 3.6736 | 3.2857 | 200 |
| Faker | #2352 | 740812bd | 5.3251 | 4.6022 | 3.6693 | 250 |
| Faker | #2351 | 6a495ba4 | 5.3251 | 4.6022 | 3.6693 | 250 |
| Faker | #2350 | f595fb2c | 5.3251 | 4.6022 | 3.6693 | 250 |
| Faker | #2349 | 0c2aef9f | 5.3246 | 4.6017 | 3.6688 | 250 |
| Faker | #2348 | 2bb97dc7 | 5.3246 | 4.6017 | 3.6688 | 250 |

---

## §3 Catch Rate per Host

| Corpus | Fixtures flagged | Total pairs | Catch rate |
|---|---|---|---|
| FastAPI (Step O, max threshold, N=100) | 32/32 | 32 | 100% |
| Rich (max threshold, N=200) | 40/40 | 40 | 100% |
| Faker (max threshold, N=250) | 35/40 | 40 | 88% |

### Rich — per-fixture × per-host score table (max threshold)

Cell format: `YES bpe>thr` / `no bpe<thr`.

| fixture | #4079 | #4077 | #4076 | #4075 | #3941 | median |
|---|---|---|---|---|---|---|
| walrus_operator | **YES** 6.909>4.203 | **YES** 6.909>4.203 | **YES** 6.909>4.203 | **YES** 6.909>4.203 | **YES** 6.909>4.203 | 6.909 |
| match_case | **YES** 7.231>4.203 | **YES** 7.231>4.203 | **YES** 7.231>4.203 | **YES** 7.231>4.203 | **YES** 7.231>4.203 | 7.231 |
| dataclass_migration | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | 5.174 |
| fstring_adoption | **YES** 5.247>4.203 | **YES** 5.247>4.203 | **YES** 5.247>4.203 | **YES** 5.247>4.203 | **YES** 5.247>4.203 | 5.247 |
| async_adoption | **YES** 7.329>4.203 | **YES** 7.329>4.203 | **YES** 7.329>4.203 | **YES** 7.329>4.203 | **YES** 7.329>4.203 | 7.329 |
| genexpr_shift | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | **YES** 5.174>4.203 | 5.174 |
| type_annotations | **YES** 5.826>4.203 | **YES** 5.826>4.203 | **YES** 5.826>4.203 | **YES** 5.826>4.203 | **YES** 5.826>4.203 | 5.826 |
| union_syntax | **YES** 6.730>4.203 | **YES** 6.730>4.203 | **YES** 6.730>4.203 | **YES** 6.730>4.203 | **YES** 6.730>4.203 | 6.730 |

### Faker — per-fixture × per-host score table (max threshold)

Cell format: `YES bpe>thr` / `no bpe<thr`.

| fixture | #2352 | #2351 | #2350 | #2349 | #2348 | median |
|---|---|---|---|---|---|---|
| walrus_operator | **YES** 7.308>5.325 | **YES** 7.308>5.325 | **YES** 7.308>5.325 | **YES** 7.308>5.325 | **YES** 7.308>5.325 | 7.308 |
| match_case | **YES** 7.380>5.325 | **YES** 7.380>5.325 | **YES** 7.380>5.325 | **YES** 7.380>5.325 | **YES** 7.380>5.325 | 7.380 |
| dataclass_migration | no 5.270<5.325 | no 5.270<5.325 | no 5.270<5.325 | no 5.270<5.325 | no 5.270<5.325 | 5.270 |
| fstring_adoption | **YES** 7.418>5.325 | **YES** 7.418>5.325 | **YES** 7.418>5.325 | **YES** 7.418>5.325 | **YES** 7.418>5.325 | 7.418 |
| async_adoption | **YES** 7.329>5.325 | **YES** 7.329>5.325 | **YES** 7.329>5.325 | **YES** 7.329>5.325 | **YES** 7.329>5.325 | 7.329 |
| genexpr_shift | **YES** 6.204>5.325 | **YES** 6.204>5.325 | **YES** 6.204>5.325 | **YES** 6.204>5.325 | **YES** 6.204>5.325 | 6.204 |
| type_annotations | **YES** 6.318>5.325 | **YES** 6.318>5.325 | **YES** 6.318>5.325 | **YES** 6.318>5.325 | **YES** 6.318>5.325 | 6.318 |
| union_syntax | **YES** 7.264>5.325 | **YES** 7.264>5.325 | **YES** 7.264>5.325 | **YES** 7.264>5.325 | **YES** 7.264>5.325 | 7.264 |

---

## §4 Counterfactual Threshold Analysis

Re-compute catch rate at p99 and p95 thresholds.

| Corpus | Threshold | Catch rate | Fixtures flagged | Total pairs |
|---|---|---|---|---|
| FastAPI | max (~4.06) | 100% | 32/32 | 32 |
| Rich | max (~4.20) | 100% | 40/40 | 40 |
| Rich | p99 (~3.67) | 100% | 40/40 | 40 |
| Rich | p95 (~3.29) | 100% | 40/40 | 40 |
| Faker | max (~5.33) | 88% | 35/40 | 40 |
| Faker | p99 (~4.60) | 100% | 40/40 | 40 |
| Faker | p95 (~3.67) | 100% | 40/40 | 40 |

### Counterfactual per-fixture detail (Faker)

| fixture | Faker max margin | Faker p99 margin | Faker p95 margin |
|---|---|---|---|
| walrus_operator | YES +1.983 | YES +2.706 | YES +3.639 |
| match_case | YES +2.055 | YES +2.778 | YES +3.711 |
| dataclass_migration | **no** -0.055 | YES +0.668 | YES +1.601 |
| fstring_adoption | YES +2.093 | YES +2.816 | YES +3.749 |
| async_adoption | YES +2.004 | YES +2.727 | YES +3.660 |
| genexpr_shift | YES +0.879 | YES +1.601 | YES +2.534 |
| type_annotations | YES +0.993 | YES +1.716 | YES +2.649 |
| union_syntax | YES +1.939 | YES +2.662 | YES +3.595 |

---

## §5 Verdict

**fix9 CONFIRMED:** Excluding locale files from model A training lifted faker catch rate to 88% (was 38% in fix8). Median threshold: 5.3251 (was ~7.15). The LLR inflation hypothesis is validated.

fix8 baseline: 38% catch rate, median threshold ~7.15
fix9 result:   88% catch rate, median threshold 5.3251

