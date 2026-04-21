# Phase 13 — BPE Contrastive TF-IDF Experiment (FastAPI, 2026-04-21)


## Summary


| scorer | tokenizer | AUC |
|---|---|---|
| contrastive_tfidf (word baseline) | argot tokenize_lines | 0.9847 |
| **bpe_contrastive_tfidf** | **UnixCoder BPE** | **1.0000** |

## Per-Category AUC


*(break category vs all controls)*


| category | n_breaks | AUC |
|---|---|---|
| async_blocking | 3 | 1.0000 |
| background_tasks | 4 | 1.0000 |
| dependency_injection | 2 | 1.0000 |
| downstream_http | 3 | 1.0000 |
| exception_handling | 6 | 1.0000 |
| framework_swap | 3 | 1.0000 |
| routing | 3 | 1.0000 |
| serialization | 3 | 1.0000 |
| validation | 4 | 1.0000 |

## Fixture Scores


| fixture | category | is_break | score |
|---|---|---|---|
| paradigm_break_flask_routing | routing | True | 10.2874 |
| paradigm_break_django_cbv | framework_swap | True | 8.7239 |
| paradigm_break_aiohttp_handler | framework_swap | True | 10.2874 |
| paradigm_break_manual_validation | validation | True | 11.0824 |
| paradigm_break_subtle_wrong_exception | exception_handling | True | 10.2874 |
| paradigm_break_subtle_manual_status_check | downstream_http | True | 7.3760 |
| paradigm_break_subtle_sync_endpoint | async_blocking | True | 8.7239 |
| paradigm_break_subtle_exception_swallow | exception_handling | True | 6.5902 |
| control_router_endpoint | routing | False | 3.6377 |
| control_dependency_injection | dependency_injection | False | 4.5175 |
| control_exception_handling | exception_handling | False | 2.6033 |
| paradigm_break_starlette_mount | routing | True | 8.1289 |
| paradigm_break_tornado_handler | framework_swap | True | 10.2874 |
| paradigm_break_voluptuous_validation | validation | True | 6.8912 |
| paradigm_break_cerberus_validation | validation | True | 8.7239 |
| paradigm_break_manual_json_response | serialization | True | 8.4862 |
| paradigm_break_bare_except | exception_handling | True | 8.7653 |
| paradigm_break_event_loop_blocking | async_blocking | True | 7.7453 |
| paradigm_break_sync_requests_in_async | downstream_http | True | 6.3690 |
| paradigm_break_concurrent_futures_background | background_tasks | True | 9.2954 |
| control_pydantic_validator | validation | False | 1.5506 |
| control_response_model | serialization | False | 1.6502 |
| control_async_streaming | async_blocking | False | 3.2149 |
| control_httpx_async | downstream_http | False | 1.9131 |
| control_annotated_depends | dependency_injection | False | 1.8759 |
| paradigm_break_sync_file_io_async | async_blocking | True | 7.4222 |
| control_anyio_thread_offload | async_blocking | False | 1.8759 |
| paradigm_break_multiprocessing_background | background_tasks | True | 8.1197 |
| paradigm_break_queue_carryover | background_tasks | True | 7.8284 |
| paradigm_break_atexit_background | background_tasks | True | 8.7239 |
| control_background_tasks_basic | background_tasks | False | 0.8303 |
| control_background_tasks_depends | background_tasks | False | 1.8195 |
| paradigm_break_manual_generator_drain | dependency_injection | True | 7.7011 |
| paradigm_break_class_instance_no_depends | dependency_injection | True | 11.0824 |
| control_nested_depends | dependency_injection | False | 1.8675 |
| paradigm_break_aiohttp_no_context | downstream_http | True | 7.4433 |
| control_httpx_depends | downstream_http | False | 1.9131 |
| paradigm_break_json_error_response | exception_handling | True | 9.2954 |
| paradigm_break_traceback_in_response | exception_handling | True | 8.1510 |
| paradigm_break_flask_errorhandler | exception_handling | True | 5.8893 |
| control_exception_handler_registration | exception_handling | False | 2.6033 |
| control_lifespan_context | framework_swap | False | 1.9131 |
| control_apirouter_composition | framework_swap | False | 2.6608 |
| control_mounted_subapp | framework_swap | False | 1.5490 |
| paradigm_break_imperative_route_loop | routing | True | 7.5039 |
| control_nested_router | routing | False | 1.6609 |
| paradigm_break_manual_dict_response | serialization | True | 8.7239 |
| paradigm_break_msgpack_response | serialization | True | 7.6516 |
| control_response_model_list | serialization | False | 2.0981 |
| paradigm_break_assert_validation | validation | True | 8.6327 |
| control_field_validator | validation | False | 1.6395 |

## Interpretation


AUC 1.0000 ≥ 0.90: FastAPI gate passed. BPE tokenization preserves the word-token baseline signal. Proceed to click.

Max-token saturation resolved: 22/31 unique break scores (word baseline had 1/8).

