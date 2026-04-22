# Phase 14 Recall Probe: fix6 on Paradigm-Break Fixtures (2026-04-22)

**Purpose:** Verify that four rounds of precision tuning (fix3→fix6) have not
silently crushed recall on genuine paradigm breaks.  Pre-fix3 catch rate: ~100%
(per Phase 14 Exp #2c Postfix V2 validation, obs 4395).

**Scorer:** fix6 (`SequentialImportBpeScorer` with per-PR recalibration + prose masking)

**Gate:** catch rate ≥90% = PASS | <90% = REGRESSION (blocks V0)

---

## §0 Summary

| metric | value |
|---|---|
| break fixtures | 31 |
| host PRs | 4 (note: only 4 met criteria; spec suggested 5) |
| total (fixture × host) pairs | 124 |
| pairs flagged | 124 |
| overall catch rate | 100.0% |
| pre-fix3 baseline | ~100% |
| verdict | **PASS** |

### Per-fixture catch rate

| fixture | category | flagged (of 4 hosts) | catch rate |
|---|---|---|---|
| paradigm_break_flask_routing | routing | 4/4 | 100% |
| paradigm_break_django_cbv | framework_swap | 4/4 | 100% |
| paradigm_break_aiohttp_handler | framework_swap | 4/4 | 100% |
| paradigm_break_manual_validation | validation | 4/4 | 100% |
| paradigm_break_subtle_wrong_exception | exception_handling | 4/4 | 100% |
| paradigm_break_subtle_manual_status_check | downstream_http | 4/4 | 100% |
| paradigm_break_subtle_sync_endpoint | async_blocking | 4/4 | 100% |
| paradigm_break_subtle_exception_swallow | exception_handling | 4/4 | 100% |
| paradigm_break_starlette_mount | routing | 4/4 | 100% |
| paradigm_break_tornado_handler | framework_swap | 4/4 | 100% |
| paradigm_break_voluptuous_validation | validation | 4/4 | 100% |
| paradigm_break_cerberus_validation | validation | 4/4 | 100% |
| paradigm_break_manual_json_response | serialization | 4/4 | 100% |
| paradigm_break_bare_except | exception_handling | 4/4 | 100% |
| paradigm_break_event_loop_blocking | async_blocking | 4/4 | 100% |
| paradigm_break_sync_requests_in_async | downstream_http | 4/4 | 100% |
| paradigm_break_concurrent_futures_background | background_tasks | 4/4 | 100% |
| paradigm_break_sync_file_io_async | async_blocking | 4/4 | 100% |
| paradigm_break_multiprocessing_background | background_tasks | 4/4 | 100% |
| paradigm_break_queue_carryover | background_tasks | 4/4 | 100% |
| paradigm_break_atexit_background | background_tasks | 4/4 | 100% |
| paradigm_break_manual_generator_drain | dependency_injection | 4/4 | 100% |
| paradigm_break_class_instance_no_depends | dependency_injection | 4/4 | 100% |
| paradigm_break_aiohttp_no_context | downstream_http | 4/4 | 100% |
| paradigm_break_json_error_response | exception_handling | 4/4 | 100% |
| paradigm_break_traceback_in_response | exception_handling | 4/4 | 100% |
| paradigm_break_flask_errorhandler | exception_handling | 4/4 | 100% |
| paradigm_break_imperative_route_loop | routing | 4/4 | 100% |
| paradigm_break_manual_dict_response | serialization | 4/4 | 100% |
| paradigm_break_msgpack_response | serialization | 4/4 | 100% |
| paradigm_break_assert_validation | validation | 4/4 | 100% |

### Per-host catch rate

| host PR | n_breaks_flagged | catch rate |
|---|---|---|
| #14862 (threshold=4.1047) | 31/31 | 100.0% |
| #14944 (threshold=4.0155) | 31/31 | 100.0% |
| #14856 (threshold=4.1115) | 31/31 | 100.0% |
| #14806 (threshold=3.2696) | 31/31 | 100.0% |

---

## §1 Per-fixture × Per-host Score Table

Cell format: `flagged/not | bpe_score vs threshold`
Cells showing `IMPORT` mean Stage 1 fired (foreign module detected).

| fixture | category | PR #14862 | PR #14944 | PR #14856 | PR #14806 |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_django_cbv | framework_swap | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_aiohttp_handler | framework_swap | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_manual_validation | validation | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_subtle_wrong_exception | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_subtle_manual_status_check | downstream_http | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_subtle_sync_endpoint | async_blocking | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_subtle_exception_swallow | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_starlette_mount | routing | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_tornado_handler | framework_swap | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_voluptuous_validation | validation | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_cerberus_validation | validation | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_manual_json_response | serialization | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_bare_except | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_event_loop_blocking | async_blocking | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_sync_requests_in_async | downstream_http | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_concurrent_futures_background | background_tasks | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_sync_file_io_async | async_blocking | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_multiprocessing_background | background_tasks | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_queue_carryover | background_tasks | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_atexit_background | background_tasks | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_manual_generator_drain | dependency_injection | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_class_instance_no_depends | dependency_injection | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_aiohttp_no_context | downstream_http | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_json_error_response | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_traceback_in_response | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_flask_errorhandler | exception_handling | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_imperative_route_loop | routing | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_manual_dict_response | serialization | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_msgpack_response | serialization | IMPORT | IMPORT | IMPORT | IMPORT |
| paradigm_break_assert_validation | validation | IMPORT | IMPORT | IMPORT | IMPORT |

---

## §2 Failures Analysis

No failures — all fixtures flagged on all host PRs.

---

## §3 Verdict

| metric | value |
|---|---|
| overall catch rate | 100.0% |
| pre-fix3 baseline | ~100% |
| delta | +0.0% |
| gate (≥90%) | **PASS** |

Recall is preserved from pre-fix3 baseline.  Four rounds of precision tuning
(fix3→fix6) did not materially reduce sensitivity to genuine paradigm breaks.
fix6 is cleared on the recall dimension.

