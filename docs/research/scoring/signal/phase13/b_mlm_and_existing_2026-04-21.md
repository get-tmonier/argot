# Phase 12 S4 — MLM Surprise Bakeoff

Date: 2026-04-21  Entry: `fastapi`  Context mode: `file_only`

> Phase 11 production winner: **EnsembleJepa mean_z @ file_only** AUC=0.6532

> Pairwise CI computed vs `tfidf_anomaly` (paired bootstrap, n=1000, α=0.05).


## Summary: Overall AUC

| scorer | overall_auc | delta_ci_lo | delta_ci_hi |
|---|---|---|---|
| ast_contrastive_max | 0.9742 | +0.1371 | +0.4242 |
| ast_contrastive_e10 | 0.8823 | +0.0419 | +0.3452 |
| tfidf_anomaly | 0.6968 | +0.0000 | +0.0000 |

## Per-Category AUC

| scorer | async_blocking | background_tasks | dependency_injection | downstream_http | exception_handling | framework_swap | routing | serialization | validation |
|---|---|---|---|---|---|---|---|---|---|
| ast_contrastive_max | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 0.8333 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| ast_contrastive_e10 | 0.8333 | 0.6250 | 0.8333 | 1.0000 | 0.9167 | 1.0000 | 1.0000 | 1.0000 | 1.0000 |
| tfidf_anomaly | 0.8333 | 0.3750 | 0.6667 | 1.0000 | 0.5000 | 0.6667 | 0.6667 | 1.0000 | 0.7500 |

## Per-Fixture Scores

| fixture | category | type | ast_contrastive_max | ast_contrastive_e10 | tfidf_anomaly |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | break | 9.7809 | 1.5107 | 0.7788 |
| paradigm_break_django_cbv | framework_swap | break | 8.3459 | 1.2643 | 0.7779 |
| paradigm_break_aiohttp_handler | framework_swap | break | 8.0635 | -0.9925 | 0.7132 |
| paradigm_break_manual_validation | validation | break | 9.7809 | 0.7738 | 0.7020 |
| paradigm_break_subtle_wrong_exception | exception_handling | break | 9.2976 | -2.5930 | 0.6611 |
| paradigm_break_subtle_manual_status_check | downstream_http | break | 7.1308 | -2.1386 | 0.6877 |
| paradigm_break_subtle_sync_endpoint | async_blocking | break | 8.1819 | -0.3631 | 0.7577 |
| paradigm_break_subtle_exception_swallow | exception_handling | break | 8.5739 | -1.1803 | 0.6794 |
| control_router_endpoint | routing | control | 3.2955 | -2.8760 | 0.7153 |
| control_dependency_injection | dependency_injection | control | 3.2611 | -1.9435 | 0.6642 |
| control_exception_handling | exception_handling | control | 1.8049 | -3.2160 | 0.5952 |
| paradigm_break_starlette_mount | routing | break | 8.3459 | -0.8495 | 0.7051 |
| paradigm_break_tornado_handler | framework_swap | break | 10.9322 | 1.2105 | 0.5308 |
| paradigm_break_voluptuous_validation | validation | break | 8.2607 | -1.8197 | 0.6327 |
| paradigm_break_cerberus_validation | validation | break | 11.2740 | -1.6031 | 0.7223 |
| paradigm_break_manual_json_response | serialization | break | 8.2289 | -1.2883 | 0.7118 |
| paradigm_break_bare_except | exception_handling | break | 9.1690 | -0.7363 | 0.7647 |
| paradigm_break_event_loop_blocking | async_blocking | break | 8.0622 | -1.5806 | 0.6417 |
| paradigm_break_sync_requests_in_async | downstream_http | break | 9.2514 | -1.7483 | 0.7326 |
| paradigm_break_concurrent_futures_background | background_tasks | break | 6.4587 | -2.9082 | 0.7804 |
| control_pydantic_validator | validation | control | 1.2425 | -3.0095 | 0.5991 |
| control_response_model | serialization | control | 3.8239 | -2.8427 | 0.6126 |
| control_async_streaming | async_blocking | control | 3.9848 | -2.1599 | 0.5963 |
| control_httpx_async | downstream_http | control | 3.2611 | -2.4404 | 0.6538 |
| control_annotated_depends | dependency_injection | control | 3.2611 | -2.5277 | 0.5505 |
| paradigm_break_sync_file_io_async | async_blocking | break | 9.1690 | -2.0126 | 0.7484 |
| control_anyio_thread_offload | async_blocking | control | 3.8736 | -1.8552 | 0.7181 |
| paradigm_break_multiprocessing_background | background_tasks | break | 9.2514 | -1.4052 | 0.7084 |
| paradigm_break_queue_carryover | background_tasks | break | 9.6704 | -1.3931 | 0.7333 |
| paradigm_break_atexit_background | background_tasks | break | 9.2514 | -1.6304 | 0.7439 |
| control_background_tasks_basic | background_tasks | control | 2.5424 | -2.2473 | 0.7886 |
| control_background_tasks_depends | background_tasks | control | 3.4920 | -1.5113 | 0.7204 |
| paradigm_break_manual_generator_drain | dependency_injection | break | 8.2700 | -1.1567 | 0.7271 |
| paradigm_break_class_instance_no_depends | dependency_injection | break | 8.7103 | -2.5156 | 0.5975 |
| control_nested_depends | dependency_injection | control | 2.6088 | -2.5335 | 0.7215 |
| paradigm_break_aiohttp_no_context | downstream_http | break | 7.1308 | -1.6016 | 0.7242 |
| control_httpx_depends | downstream_http | control | 3.2611 | -2.3618 | 0.6468 |
| paradigm_break_json_error_response | exception_handling | break | 8.2700 | -1.2929 | 0.5862 |
| paradigm_break_traceback_in_response | exception_handling | break | 8.2700 | -1.0906 | 0.5791 |
| paradigm_break_flask_errorhandler | exception_handling | break | 1.6966 | -3.2142 | 0.5519 |
| control_exception_handler_registration | exception_handling | control | 1.8049 | -2.8983 | 0.6056 |
| control_lifespan_context | framework_swap | control | 3.9848 | -2.8962 | 0.6637 |
| control_apirouter_composition | framework_swap | control | 1.5092 | -2.9874 | 0.6095 |
| control_mounted_subapp | framework_swap | control | 1.9644 | -2.8740 | 0.5846 |
| paradigm_break_imperative_route_loop | routing | break | 10.9322 | -1.8984 | 0.6156 |
| control_nested_router | routing | control | 1.5092 | -2.9118 | 0.4954 |
| paradigm_break_manual_dict_response | serialization | break | 8.0635 | -1.5330 | 0.6636 |
| paradigm_break_msgpack_response | serialization | break | 8.2289 | -2.6906 | 0.7574 |
| control_response_model_list | serialization | control | 3.8239 | -3.0972 | 0.5384 |
| paradigm_break_assert_validation | validation | break | 9.5388 | -0.4885 | 0.6490 |
| control_field_validator | validation | control | 1.5401 | -2.9781 | 0.6968 |
