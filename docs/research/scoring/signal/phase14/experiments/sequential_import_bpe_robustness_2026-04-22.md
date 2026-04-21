# Phase 14 Experiment 2b — Sequential pipeline robustness under random calibrations (2026-04-22)

**Scorer:** `SequentialImportBpeScorer`

**Hypothesis:** The STRONG verdict from exp #2 holds when FastAPI and rich calibrations
are rebuilt from real source corpora (n=100 random hunks, seeds 0–4) instead of curated
control fixture sets (n=20 and n=10 respectively).

**Pre-registered stability bands:**
- STABLE: threshold CV < 5% AND recall = 100% on all 5 seeds
- FRAGILE: threshold CV in [5%, 15%) OR recall varies by ≤1 break across seeds
- UNSTABLE: threshold CV ≥ 15% OR recall varies by >1 break across seeds

**Overall verdict:** READY if both domains STABLE and faker holdout valid; NEEDS_WORK otherwise.

---

## 1. Source Corpus

| domain | source | n source files | n hunk candidates | exclusions |
|---|---|---|---|---|
| FastAPI | `.argot/research/repos/fastapi` (shallow clone) | 496 | 367 | tests/, docs/, examples/, scripts/, benchmarks/ |
| rich | `.argot/research/repos/rich` (shallow clone) | 106 | 237 | tests/, docs/ |
| faker | `acceptance/catalog/faker/sources/model_a/` (existing) | 722 | 722 (fixed, from sampled_hunks.jsonl) | n/a (pre-curated) |

Faker calibration: 159 hunks from `sampled_hunks.jsonl` (no resampling).
FastAPI and rich calibration: 100 hunks per seed, sampled from source corpus.

---

## 2. Per-seed Threshold Table

### FastAPI (n_breaks=31, n_controls=20)

| seed | threshold | recall | n_flagged | n_fp | fp_rate |
|---|---|---|---|---|---|
| 0 | 4.0185 | 100% | 31/31 | 20 | 100% |
| 1 | 4.0185 | 100% | 31/31 | 20 | 100% |
| 2 | 4.0668 | 100% | 31/31 | 20 | 100% |
| 3 | 4.2039 | 100% | 31/31 | 20 | 100% |
| 4 | 4.2039 | 100% | 31/31 | 20 | 100% |
| **stats** | mean=4.1023 std=0.0848 CV=2.1% | — | — | — | — |

### Rich (n_breaks=10, n_controls=10)

| seed | threshold | recall | n_flagged | n_fp | fp_rate |
|---|---|---|---|---|---|
| 0 | 4.4015 | 100% | 10/10 | 0 | 0% |
| 1 | 4.8159 | 100% | 10/10 | 0 | 0% |
| 2 | 4.8159 | 100% | 10/10 | 0 | 0% |
| 3 | 4.8159 | 100% | 10/10 | 0 | 0% |
| 4 | 3.7046 | 100% | 10/10 | 0 | 0% |
| **stats** | mean=4.5107 std=0.4338 CV=9.6% | — | — | — | — |

---

## 3. break_ansi_raw_2 Tracking (thin-margin break from exp #2)

Exp #2 margin: bpe_score=5.6851 vs threshold=5.5984 (+0.087).

| seed | bpe_score | threshold | margin | flagged |
|---|---|---|---|---|
| 0 | 5.6851 | 4.4015 | +1.2836 | YES |
| 1 | 5.6851 | 4.8159 | +0.8692 | YES |
| 2 | 5.6851 | 4.8159 | +0.8692 | YES |
| 3 | 5.6851 | 4.8159 | +0.8692 | YES |
| 4 | 5.6851 | 3.7046 | +1.9805 | YES |

---

## 4. Per-break Minimum Margin Across 5 Seeds

Minimum margin = min over seeds of (bpe_score - threshold).
Negative margin means the break was NOT flagged on that seed.

### FastAPI

| name | category | min_margin | always_flagged |
|---|---|---|---|
| paradigm_break_flask_routing | routing | +2.6575 | YES |
| paradigm_break_django_cbv | framework_swap | +2.6575 | YES |
| paradigm_break_aiohttp_handler | framework_swap | +3.6438 | YES |
| paradigm_break_manual_validation | validation | +2.6575 | YES |
| paradigm_break_subtle_wrong_exception | exception_handling | +3.4686 | YES |
| paradigm_break_subtle_manual_status_check | downstream_http | +3.4686 | YES |
| paradigm_break_subtle_sync_endpoint | async_blocking | +3.4686 | YES |
| paradigm_break_subtle_exception_swallow | exception_handling | +3.4686 | YES |
| paradigm_break_starlette_mount | routing | +2.6575 | YES |
| paradigm_break_tornado_handler | framework_swap | +3.1567 | YES |
| paradigm_break_voluptuous_validation | validation | +3.8551 | YES |
| paradigm_break_cerberus_validation | validation | +2.6575 | YES |
| paradigm_break_manual_json_response | serialization | +3.8551 | YES |
| paradigm_break_bare_except | exception_handling | +2.6575 | YES |
| paradigm_break_event_loop_blocking | async_blocking | +3.1845 | YES |
| paradigm_break_sync_requests_in_async | downstream_http | +3.0010 | YES |
| paradigm_break_concurrent_futures_background | background_tasks | +4.2208 | YES |
| paradigm_break_sync_file_io_async | async_blocking | +3.6003 | YES |
| paradigm_break_multiprocessing_background | background_tasks | +3.8551 | YES |
| paradigm_break_queue_carryover | background_tasks | +4.2208 | YES |
| paradigm_break_atexit_background | background_tasks | +4.2208 | YES |
| paradigm_break_manual_generator_drain | dependency_injection | +2.6575 | YES |
| paradigm_break_class_instance_no_depends | dependency_injection | +4.2208 | YES |
| paradigm_break_aiohttp_no_context | downstream_http | +4.2208 | YES |
| paradigm_break_json_error_response | exception_handling | +2.6575 | YES |
| paradigm_break_traceback_in_response | exception_handling | +2.6575 | YES |
| paradigm_break_flask_errorhandler | exception_handling | +2.6575 | YES |
| paradigm_break_imperative_route_loop | routing | +3.2143 | YES |
| paradigm_break_manual_dict_response | serialization | +2.6575 | YES |
| paradigm_break_msgpack_response | serialization | +3.6147 | YES |
| paradigm_break_assert_validation | validation | +3.2777 | YES |

### Rich

| name | category | min_margin | always_flagged |
|---|---|---|---|
| break_ansi_raw_1 | ansi_raw | +2.9691 | YES |
| break_ansi_raw_2 | ansi_raw | +0.8692 | YES |
| break_colorama_1 | colorama | +1.9138 | YES |
| break_colorama_2 | colorama | +1.1504 | YES |
| break_termcolor_1 | termcolor | +1.3370 | YES |
| break_termcolor_2 | termcolor | +1.9138 | YES |
| break_curses_1 | curses | +1.4011 | YES |
| break_curses_2 | curses | +2.8525 | YES |
| break_print_manual_1 | print_manual | +0.8692 | YES |
| break_print_manual_2 | print_manual | +1.9138 | YES |

---

## 5. Faker Baseline (existing 159-hunk calibration)

| n_cal | threshold | breaks | flagged | recall | cal_fp |
|---|---|---|---|---|---|
| 159 | 7.3732 | 5 | 5 | 100% | 0 |

faker_hunk_0047: bpe=7.3732, threshold=7.3732, flagged=False

---

## 6. Faker Holdout Diagnostic

Removes faker_hunk_0047 from calibration to expose the construction artifact
(threshold = max(cal) is set by this single outlier).

| | value |
|---|---|
| Original n_cal | 159 |
| Holdout n_cal (−1) | 158 |
| Original threshold | 7.3732 |
| New threshold (without hunk_0047) | 6.0009 |
| hunk_0047 bpe_score | 7.3732 |
| hunk_0047 flagged at new threshold | True |

**Finding:** faker_hunk_0047 fires when removed from calibration.
The threshold=max(cal) construction is defined by this outlier.
Without it, the hunk becomes a false positive, confirming the construction artifact.

---

## 7. FP Rate Analysis

Note: stability bands (pre-registered) cover threshold CV and recall only.
This section documents FP rate as a separate correctness dimension.

| domain | FP rate (all seeds) | broken (>10%) |
|---|---|---|
| FastAPI | 100% (all seeds identical) | YES — critical |
| rich | 0% (all seeds identical) | no |

**FastAPI FP diagnosis:** All 20 synthetic control fixtures are flagged on every seed.
Cause: real FastAPI source code (496 files) has a narrow token distribution.
Calibrating on 100 random source hunks yields a low BPE threshold (~4.1),
which the synthetic fixture files exceed because they contain more diverse patterns.
This is a fixture-vs-source distribution mismatch, not a scorer error.
The 100% FP rate makes the pipeline unusable for FastAPI in its current form.

---

## 8. Stability Verdict

| domain | threshold CV | recall range (breaks) | FP rate | verdict |
|---|---|---|---|---|
| FastAPI | 2.1% | 0.0 breaks | 100% | **STABLE** (but FP=100%) |
| rich | 9.6% | 0.0 breaks | 0% | **FRAGILE** |

**Overall pipeline verdict: NEEDS_WORK**

Blocker: FastAPI FP=100% (fixture-vs-source mismatch); rich is FRAGILE (CV=9.6%, recall_range=0.0)

One or more domains failed the STABLE band or FP threshold.
The pipeline needs revision before V1 (see blocker above).

Faker holdout: hunk_0047 fires when removed from calibration — threshold=max(cal) is construction-artifact-dependent. This is expected by design but note the fragility.

