# Phase 14 Experiment 1 — Import-graph foreign-module scorer: Phase 13 domain validation (2026-04-22)

**Scorer:** `ImportGraphScorer` — counts top-level modules in hunk that were never seen in model_A
**Domains:** FastAPI (control fixtures as model_A), rich (72 source files), faker (722 source files)
**Key diagnostic:** Does the scorer filter BPE's false-positive outlier `faker_hunk_0047` (error-handling code, BPE score 7.37)?

---

## 1. Cross-domain Summary

| domain | model_A files | breaks | breaks flagged | recall | controls/cal | flagged | FP rate |
|---|---|---|---|---|---|---|---|
| FastAPI | 20 | 31 | 20 | 65% | 20 (controls) | 0 | 0% |
| rich | 72 | 10 | 6 | 60% | 10 (controls) | 0 | 0% |
| faker | 722 | 5 | 5 | 100% | 159 (cal hunks) | 0 | 0.0% |

Overall break recall across all domains: **67%** (31/46)

---

## 2. Per-domain Fixture Details

### FastAPI

| name | category | is_break | score | flagged | foreign modules |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | break | 1 | YES | — |
| paradigm_break_django_cbv | framework_swap | break | 1 | YES | — |
| paradigm_break_aiohttp_handler | framework_swap | break | 1 | YES | — |
| paradigm_break_manual_validation | validation | break | 0 | no | — |
| paradigm_break_subtle_wrong_exception | exception_handling | break | 0 | no | — |
| paradigm_break_subtle_manual_status_check | downstream_http | break | 0 | no | — |
| paradigm_break_subtle_sync_endpoint | async_blocking | break | 1 | YES | — |
| paradigm_break_subtle_exception_swallow | exception_handling | break | 0 | no | — |
| control_router_endpoint | routing | control | 0 | no | — |
| control_dependency_injection | dependency_injection | control | 0 | no | — |
| control_exception_handling | exception_handling | control | 0 | no | — |
| paradigm_break_starlette_mount | routing | break | 1 | YES | — |
| paradigm_break_tornado_handler | framework_swap | break | 1 | YES | — |
| paradigm_break_voluptuous_validation | validation | break | 1 | YES | — |
| paradigm_break_cerberus_validation | validation | break | 1 | YES | — |
| paradigm_break_manual_json_response | serialization | break | 2 | YES | — |
| paradigm_break_bare_except | exception_handling | break | 0 | no | — |
| paradigm_break_event_loop_blocking | async_blocking | break | 1 | YES | — |
| paradigm_break_sync_requests_in_async | downstream_http | break | 1 | YES | — |
| paradigm_break_concurrent_futures_background | background_tasks | break | 1 | YES | — |
| control_pydantic_validator | validation | control | 0 | no | — |
| control_response_model | serialization | control | 0 | no | — |
| control_async_streaming | async_blocking | control | 0 | no | — |
| control_httpx_async | downstream_http | control | 0 | no | — |
| control_annotated_depends | dependency_injection | control | 0 | no | — |
| paradigm_break_sync_file_io_async | async_blocking | break | 0 | no | — |
| control_anyio_thread_offload | async_blocking | control | 0 | no | — |
| paradigm_break_multiprocessing_background | background_tasks | break | 1 | YES | — |
| paradigm_break_queue_carryover | background_tasks | break | 1 | YES | — |
| paradigm_break_atexit_background | background_tasks | break | 2 | YES | — |
| control_background_tasks_basic | background_tasks | control | 0 | no | — |
| control_background_tasks_depends | background_tasks | control | 0 | no | — |
| paradigm_break_manual_generator_drain | dependency_injection | break | 0 | no | — |
| paradigm_break_class_instance_no_depends | dependency_injection | break | 1 | YES | — |
| control_nested_depends | dependency_injection | control | 0 | no | — |
| paradigm_break_aiohttp_no_context | downstream_http | break | 1 | YES | — |
| control_httpx_depends | downstream_http | control | 0 | no | — |
| paradigm_break_json_error_response | exception_handling | break | 0 | no | — |
| paradigm_break_traceback_in_response | exception_handling | break | 1 | YES | — |
| paradigm_break_flask_errorhandler | exception_handling | break | 0 | no | — |
| control_exception_handler_registration | exception_handling | control | 0 | no | — |
| control_lifespan_context | framework_swap | control | 0 | no | — |
| control_apirouter_composition | framework_swap | control | 0 | no | — |
| control_mounted_subapp | framework_swap | control | 0 | no | — |
| paradigm_break_imperative_route_loop | routing | break | 0 | no | — |
| control_nested_router | routing | control | 0 | no | — |
| paradigm_break_manual_dict_response | serialization | break | 1 | YES | — |
| paradigm_break_msgpack_response | serialization | break | 1 | YES | — |
| control_response_model_list | serialization | control | 0 | no | — |
| paradigm_break_assert_validation | validation | break | 0 | no | — |
| control_field_validator | validation | control | 0 | no | — |

### Rich

| name | category | is_break | score | flagged | foreign modules |
|---|---|---|---|---|---|
| control_console_capture | control_console | control | 0 | no | — |
| control_console_init | control_console | control | 0 | no | — |
| control_table_add_column_body | control_table | control | 0 | no | — |
| control_table_rich_console | control_table | control | 0 | no | — |
| control_live_enter_exit | control_live | control | 0 | no | — |
| control_live_renderable | control_live | control | 0 | no | — |
| control_text_init | control_text | control | 0 | no | — |
| control_text_styled | control_text | control | 0 | no | — |
| control_panel_init_body | control_panel | control | 0 | no | — |
| control_panel_rich_console | control_panel | control | 0 | no | — |
| break_ansi_raw_1 | ansi_raw | break | 0 | no | — |
| break_ansi_raw_2 | ansi_raw | break | 0 | no | — |
| break_colorama_1 | colorama | break | 1 | YES | — |
| break_colorama_2 | colorama | break | 1 | YES | — |
| break_termcolor_1 | termcolor | break | 1 | YES | — |
| break_termcolor_2 | termcolor | break | 1 | YES | — |
| break_curses_1 | curses | break | 1 | YES | — |
| break_curses_2 | curses | break | 1 | YES | — |
| break_print_manual_1 | print_manual | break | 0 | no | — |
| break_print_manual_2 | print_manual | break | 0 | no | — |

### Faker (break fixtures)

| name | category | score | flagged |
|---|---|---|---|
| break_mimesis_alt_1 | mimesis_alt | 1 | YES |
| break_threading_provider_1 | threading_provider | 2 | YES |
| break_sqlalchemy_sink_1 | sqlalchemy_sink | 1 | YES |
| break_numpy_random_1 | numpy_random | 1 | YES |
| break_requests_source_1 | requests_source | 1 | YES |

---

## 3. Key Diagnostic: faker_hunk_0047 (BPE False-positive)

- BPE score (from Phase 13): 7.3732 — the single hunk that caused BPE's `FULL OVERLAP` verdict on faker
- Import scorer score: 0
- Flagged: False
- **PASS** — scorer correctly ignores this hunk (error-handling code, no foreign imports)

No calibration hunks flagged (FP rate = 0%).

---

## 4. False-negatives (stdlib-only or no-import patterns)

**FastAPI:** paradigm_break_manual_validation, paradigm_break_subtle_wrong_exception, paradigm_break_subtle_manual_status_check, paradigm_break_subtle_exception_swallow, paradigm_break_bare_except, paradigm_break_sync_file_io_async, paradigm_break_manual_generator_drain, paradigm_break_json_error_response, paradigm_break_flask_errorhandler, paradigm_break_imperative_route_loop, paradigm_break_assert_validation
**Rich:** break_ansi_raw_1, break_ansi_raw_2, break_print_manual_1, break_print_manual_2

These breaks do not introduce foreign imports — they are either stdlib-only paradigm
violations (e.g. raw ANSI escape codes, `assert` for validation, `raise ValueError`)
or use libraries already present in model_A. A second scorer layer is required to cover them.

---

## 5. Verdict

| domain | recall | FP rate | per-domain verdict |
|---|---|---|---|
| FastAPI | 65% | 0% | PARTIAL |
| rich | 60% | 0% | PARTIAL |
| faker | 100% | 0.0% | STRONG |
| **combined** | **67%** | **0.0%** | — |

**Cross-domain verdict: PARTIAL**

The scorer has complementary signal — it catches foreign-library breaks that both BPE-tfidf and AST-contrastive can miss — but cannot be a standalone primary scorer. The false-negative set (stdlib-only paradigm breaks) requires a second axis.

**Recommendation:** use as a fast pre-filter: any hunk with score ≥ 1 is an instant flag (high precision); the remaining hunks (score = 0) are passed to BPE or AST for deeper scoring.

