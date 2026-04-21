# Phase 12 S4 — MLM Surprise Bakeoff

Date: 2026-04-21  Entry: `fastapi`  Context mode: `file_only`

> Phase 11 production winner: **EnsembleJepa mean_z @ file_only** AUC=0.6532

> Pairwise CI computed vs `tfidf_anomaly` (paired bootstrap, n=1000, α=0.05).


## Summary: Overall AUC

| scorer | overall_auc | delta_ci_lo | delta_ci_hi |
|---|---|---|---|
| ast_contrastive | 0.5871 | -0.2903 | +0.0758 |
| ast_contrastive_e01 | 0.5710 | -0.3048 | +0.0661 |
| ast_contrastive_e10 | 0.6032 | -0.2766 | +0.0935 |
| tfidf_anomaly | 0.6968 | +0.0000 | +0.0000 |

## Per-Category AUC

| scorer | async_blocking | background_tasks | dependency_injection | downstream_http | exception_handling | framework_swap | routing | serialization | validation |
|---|---|---|---|---|---|---|---|---|---|
| ast_contrastive | 0.7500 | 0.7500 | 0.5000 | 0.7500 | 0.6667 | 0.6667 | 0.5833 | 0.8333 | 0.3750 |
| ast_contrastive_e01 | 0.7500 | 0.7500 | 0.5000 | 0.5833 | 0.6667 | 0.6667 | 0.5833 | 0.6667 | 0.3750 |
| ast_contrastive_e10 | 0.7500 | 0.7500 | 0.5000 | 0.7500 | 0.7500 | 0.6667 | 0.5833 | 0.8333 | 0.5000 |
| tfidf_anomaly | 0.8333 | 0.3750 | 0.6667 | 1.0000 | 0.5000 | 0.6667 | 0.6667 | 1.0000 | 0.7500 |

## Per-Fixture Scores

| fixture | category | type | ast_contrastive | ast_contrastive_e01 | ast_contrastive_e10 | tfidf_anomaly |
|---|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | break | 6.2964 | 7.2436 | 5.2173 | 0.7788 |
| paradigm_break_django_cbv | framework_swap | break | 6.5133 | 7.4463 | 5.4100 | 0.7779 |
| paradigm_break_aiohttp_handler | framework_swap | break | 5.6556 | 6.3901 | 4.7939 | 0.7132 |
| paradigm_break_manual_validation | validation | break | 6.1733 | 7.1307 | 5.0595 | 0.7020 |
| paradigm_break_subtle_wrong_exception | exception_handling | break | 5.0127 | 5.7033 | 4.2484 | 0.6611 |
| paradigm_break_subtle_manual_status_check | downstream_http | break | 5.4759 | 6.0806 | 4.7137 | 0.6877 |
| paradigm_break_subtle_sync_endpoint | async_blocking | break | 6.3753 | 7.1662 | 5.4263 | 0.7577 |
| paradigm_break_subtle_exception_swallow | exception_handling | break | 5.2999 | 6.0240 | 4.4673 | 0.6794 |
| control_router_endpoint | routing | control | 5.3675 | 6.0918 | 4.5781 | 0.7153 |
| control_dependency_injection | dependency_injection | control | 5.5609 | 6.2617 | 4.7378 | 0.6642 |
| control_exception_handling | exception_handling | control | 5.0456 | 5.7493 | 4.2423 | 0.5952 |
| paradigm_break_starlette_mount | routing | break | 5.2419 | 5.8931 | 4.4721 | 0.7051 |
| paradigm_break_tornado_handler | framework_swap | break | 0.0000 | 0.0000 | 0.0000 | 0.5308 |
| paradigm_break_voluptuous_validation | validation | break | 0.0000 | 0.0000 | 0.0000 | 0.6327 |
| paradigm_break_cerberus_validation | validation | break | 4.9252 | 5.4424 | 4.3265 | 0.7223 |
| paradigm_break_manual_json_response | serialization | break | 5.3405 | 6.0901 | 4.5322 | 0.7118 |
| paradigm_break_bare_except | exception_handling | break | 5.1733 | 5.9225 | 4.3346 | 0.7647 |
| paradigm_break_event_loop_blocking | async_blocking | break | 5.2392 | 5.9768 | 4.4698 | 0.6417 |
| paradigm_break_sync_requests_in_async | downstream_http | break | 5.3949 | 5.9030 | 4.7388 | 0.7326 |
| paradigm_break_concurrent_futures_background | background_tasks | break | 4.5018 | 5.0003 | 3.9829 | 0.7804 |
| control_pydantic_validator | validation | control | 4.9514 | 5.6701 | 4.1576 | 0.5991 |
| control_response_model | serialization | control | 4.9987 | 5.7428 | 4.1698 | 0.6126 |
| control_async_streaming | async_blocking | control | 5.2103 | 5.9363 | 4.4150 | 0.5963 |
| control_httpx_async | downstream_http | control | 5.2854 | 6.0601 | 4.4385 | 0.6538 |
| control_annotated_depends | dependency_injection | control | 5.0212 | 5.6192 | 4.3216 | 0.5505 |
| paradigm_break_sync_file_io_async | async_blocking | break | 0.0000 | 0.0000 | 0.0000 | 0.7484 |
| control_anyio_thread_offload | async_blocking | control | 0.0000 | 0.0000 | 0.0000 | 0.7181 |
| paradigm_break_multiprocessing_background | background_tasks | break | 0.0000 | 0.0000 | 0.0000 | 0.7084 |
| paradigm_break_queue_carryover | background_tasks | break | 0.0000 | 0.0000 | 0.0000 | 0.7333 |
| paradigm_break_atexit_background | background_tasks | break | 4.8928 | 5.5644 | 4.1457 | 0.7439 |
| control_background_tasks_basic | background_tasks | control | 0.0000 | 0.0000 | 0.0000 | 0.7886 |
| control_background_tasks_depends | background_tasks | control | 0.0000 | 0.0000 | 0.0000 | 0.7204 |
| paradigm_break_manual_generator_drain | dependency_injection | break | 5.3398 | 6.1896 | 4.4331 | 0.7271 |
| paradigm_break_class_instance_no_depends | dependency_injection | break | 5.3461 | 6.1020 | 4.5218 | 0.5975 |
| control_nested_depends | dependency_injection | control | 5.3435 | 6.1125 | 4.4590 | 0.7215 |
| paradigm_break_aiohttp_no_context | downstream_http | break | 0.0000 | 0.0000 | 0.0000 | 0.7242 |
| control_httpx_depends | downstream_http | control | 0.0000 | 0.0000 | 0.0000 | 0.6468 |
| paradigm_break_json_error_response | exception_handling | break | 0.0000 | 0.0000 | 0.0000 | 0.5862 |
| paradigm_break_traceback_in_response | exception_handling | break | 0.0000 | 0.0000 | 0.0000 | 0.5791 |
| paradigm_break_flask_errorhandler | exception_handling | break | 5.1398 | 5.8971 | 4.2868 | 0.5519 |
| control_exception_handler_registration | exception_handling | control | 0.0000 | 0.0000 | 0.0000 | 0.6056 |
| control_lifespan_context | framework_swap | control | 4.7357 | 5.3109 | 4.0931 | 0.6637 |
| control_apirouter_composition | framework_swap | control | 4.6186 | 5.1327 | 4.0295 | 0.6095 |
| control_mounted_subapp | framework_swap | control | 4.7775 | 5.4558 | 4.0381 | 0.5846 |
| paradigm_break_imperative_route_loop | routing | break | 0.0000 | 0.0000 | 0.0000 | 0.6156 |
| control_nested_router | routing | control | 0.0000 | 0.0000 | 0.0000 | 0.4954 |
| paradigm_break_manual_dict_response | serialization | break | 5.0056 | 5.6897 | 4.2358 | 0.6636 |
| paradigm_break_msgpack_response | serialization | break | 5.2498 | 6.0134 | 4.4130 | 0.7574 |
| control_response_model_list | serialization | control | 5.1786 | 5.9704 | 4.3192 | 0.5384 |
| paradigm_break_assert_validation | validation | break | 5.1215 | 5.9030 | 4.2365 | 0.6490 |
| control_field_validator | validation | control | 5.2751 | 6.0202 | 4.4514 | 0.6968 |
