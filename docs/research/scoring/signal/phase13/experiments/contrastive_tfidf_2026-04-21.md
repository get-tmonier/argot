# Phase 13 — Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)


## Summary


| scorer | AUC |
|---|---|
| tfidf_anomaly (one-sided, existing) | 0.6968 |
| ast_contrastive_max (AST + contrast) | 0.9742 |
| **contrastive_tfidf (tokens + contrast)** | **0.9847** |

## Per-Category AUC


*(break category vs all controls)*


| category | n_breaks | AUC |
|---|---|---|
| async_blocking | 3 | 1.0000 |
| background_tasks | 4 | 1.0000 |
| dependency_injection | 2 | 1.0000 |
| downstream_http | 3 | 1.0000 |
| exception_handling | 6 | 0.9917 |
| framework_swap | 3 | 1.0000 |
| routing | 3 | 1.0000 |
| serialization | 3 | 0.8583 |
| validation | 4 | 1.0000 |

## Fixture Scores


| fixture | category | is_break | score |
|---|---|---|---|
| paradigm_break_flask_routing | routing | True | 9.5652 |
| paradigm_break_django_cbv | framework_swap | True | 8.3952 |
| paradigm_break_aiohttp_handler | framework_swap | True | 7.7745 |
| paradigm_break_manual_validation | validation | True | 9.3100 |
| paradigm_break_subtle_wrong_exception | exception_handling | True | 7.5478 |
| paradigm_break_subtle_manual_status_check | downstream_http | True | 9.4383 |
| paradigm_break_subtle_sync_endpoint | async_blocking | True | 7.9162 |
| paradigm_break_subtle_exception_swallow | exception_handling | True | 8.7160 |
| control_router_endpoint | routing | False | 2.4447 |
| control_dependency_injection | dependency_injection | False | 1.4467 |
| control_exception_handling | exception_handling | False | 2.8331 |
| paradigm_break_starlette_mount | routing | True | 8.5906 |
| paradigm_break_tornado_handler | framework_swap | True | 8.3952 |
| paradigm_break_voluptuous_validation | validation | True | 8.1389 |
| paradigm_break_cerberus_validation | validation | True | 8.8464 |
| paradigm_break_manual_json_response | serialization | True | 9.3100 |
| paradigm_break_bare_except | exception_handling | True | 6.7161 |
| paradigm_break_event_loop_blocking | async_blocking | True | 7.5100 |
| paradigm_break_sync_requests_in_async | downstream_http | True | 7.9162 |
| paradigm_break_concurrent_futures_background | background_tasks | True | 8.7160 |
| control_pydantic_validator | validation | False | 1.0781 |
| control_response_model | serialization | False | 1.5886 |
| control_async_streaming | async_blocking | False | 3.3154 |
| control_httpx_async | downstream_http | False | 1.1633 |
| control_annotated_depends | dependency_injection | False | 1.5886 |
| paradigm_break_sync_file_io_async | async_blocking | True | 7.7954 |
| control_anyio_thread_offload | async_blocking | False | 1.5901 |
| paradigm_break_multiprocessing_background | background_tasks | True | 9.5652 |
| paradigm_break_queue_carryover | background_tasks | True | 7.0095 |
| paradigm_break_atexit_background | background_tasks | True | 8.8464 |
| control_background_tasks_basic | background_tasks | False | 0.4987 |
| control_background_tasks_depends | background_tasks | False | 0.4987 |
| paradigm_break_manual_generator_drain | dependency_injection | True | 7.4025 |
| paradigm_break_class_instance_no_depends | dependency_injection | True | 8.1735 |
| control_nested_depends | dependency_injection | False | 1.7347 |
| paradigm_break_aiohttp_no_context | downstream_http | True | 9.4383 |
| control_httpx_depends | downstream_http | False | 1.1633 |
| paradigm_break_json_error_response | exception_handling | True | 8.7160 |
| paradigm_break_traceback_in_response | exception_handling | True | 7.9592 |
| paradigm_break_flask_errorhandler | exception_handling | True | 5.2040 |
| control_exception_handler_registration | exception_handling | False | 2.8331 |
| control_lifespan_context | framework_swap | False | 1.7347 |
| control_apirouter_composition | framework_swap | False | 1.2072 |
| control_mounted_subapp | framework_swap | False | 6.4949 |
| paradigm_break_imperative_route_loop | routing | True | 8.2233 |
| control_nested_router | routing | False | 0.9520 |
| paradigm_break_manual_dict_response | serialization | True | 7.1687 |
| paradigm_break_msgpack_response | serialization | True | 1.5901 |
| control_response_model_list | serialization | False | 1.4920 |
| paradigm_break_assert_validation | validation | True | 9.3100 |
| control_field_validator | validation | False | 2.4327 |

## Interpretation


AUC 0.9847 ≥ 0.85: the contrastive log-ratio formulation alone — applied to raw tokens rather than AST treelets — recovers most of the lift seen in ast_contrastive_max. The key innovation was the contrastive signal structure, not the AST treelet vocabulary. **Next step: pursue a contrastive MLM baseline** (e.g. CodeBERT log P_B(t) − log P_A(t)) to test whether a pre-trained token distribution outperforms the stdlib corpus.

