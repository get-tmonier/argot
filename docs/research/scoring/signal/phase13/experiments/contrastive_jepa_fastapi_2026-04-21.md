# Phase 13 — Contrastive-JEPA Experiment (FastAPI, 2026-04-21)


## Summary


| scorer | corpus | AUC |
|---|---|---|
| tfidf_anomaly (production) | FastAPI | 0.6968 |
| ast_contrastive_max (FastAPI) | FastAPI | 0.9742 |
| **contrastive_jepa (this run)** | **FastAPI** | **0.5532** |

## Per-Category AUC


*(break category vs all controls)*


| category | n_breaks | AUC |
|---|---|---|
| async_blocking | 3 | 0.6000 |
| background_tasks | 4 | 0.7125 |
| dependency_injection | 2 | 0.6500 |
| downstream_http | 3 | 0.3000 |
| exception_handling | 6 | 0.4167 |
| framework_swap | 3 | 0.7000 |
| routing | 3 | 0.6000 |
| serialization | 3 | 0.8000 |
| validation | 4 | 0.3750 |

## Fixture Scores


| fixture | category | is_break | score |
|---|---|---|---|
| paradigm_break_flask_routing | routing | True | 5.3947 |
| paradigm_break_django_cbv | framework_swap | True | 8.9103 |
| paradigm_break_aiohttp_handler | framework_swap | True | 7.1657 |
| paradigm_break_manual_validation | validation | True | 7.3148 |
| paradigm_break_subtle_wrong_exception | exception_handling | True | 6.2147 |
| paradigm_break_subtle_manual_status_check | downstream_http | True | 5.8160 |
| paradigm_break_subtle_sync_endpoint | async_blocking | True | 6.3045 |
| paradigm_break_subtle_exception_swallow | exception_handling | True | 6.8928 |
| control_router_endpoint | routing | False | 6.0100 |
| control_dependency_injection | dependency_injection | False | 4.7779 |
| control_exception_handling | exception_handling | False | 4.3607 |
| paradigm_break_starlette_mount | routing | True | 12.3110 |
| paradigm_break_tornado_handler | framework_swap | True | 8.0073 |
| paradigm_break_voluptuous_validation | validation | True | 5.9102 |
| paradigm_break_cerberus_validation | validation | True | 6.8923 |
| paradigm_break_manual_json_response | serialization | True | 9.3317 |
| paradigm_break_bare_except | exception_handling | True | 5.8652 |
| paradigm_break_event_loop_blocking | async_blocking | True | 6.4557 |
| paradigm_break_sync_requests_in_async | downstream_http | True | 5.2176 |
| paradigm_break_concurrent_futures_background | background_tasks | True | 9.0014 |
| control_pydantic_validator | validation | False | 4.3599 |
| control_response_model | serialization | False | 7.3780 |
| control_async_streaming | async_blocking | False | 6.4057 |
| control_httpx_async | downstream_http | False | 6.3116 |
| control_annotated_depends | dependency_injection | False | 8.0567 |
| paradigm_break_sync_file_io_async | async_blocking | True | 18.1147 |
| control_anyio_thread_offload | async_blocking | False | 10.2072 |
| paradigm_break_multiprocessing_background | background_tasks | True | 7.9145 |
| paradigm_break_queue_carryover | background_tasks | True | 8.2802 |
| paradigm_break_atexit_background | background_tasks | True | 7.3528 |
| control_background_tasks_basic | background_tasks | False | 7.9460 |
| control_background_tasks_depends | background_tasks | False | 14.5940 |
| paradigm_break_manual_generator_drain | dependency_injection | True | 7.5418 |
| paradigm_break_class_instance_no_depends | dependency_injection | True | 7.8177 |
| control_nested_depends | dependency_injection | False | 8.9413 |
| paradigm_break_aiohttp_no_context | downstream_http | True | 6.4412 |
| control_httpx_depends | downstream_http | False | 6.6022 |
| paradigm_break_json_error_response | exception_handling | True | 8.2785 |
| paradigm_break_traceback_in_response | exception_handling | True | 6.3657 |
| paradigm_break_flask_errorhandler | exception_handling | True | 5.9248 |
| control_exception_handler_registration | exception_handling | False | 5.0596 |
| control_lifespan_context | framework_swap | False | 6.9414 |
| control_apirouter_composition | framework_swap | False | 6.6037 |
| control_mounted_subapp | framework_swap | False | 8.4900 |
| paradigm_break_imperative_route_loop | routing | True | 7.5682 |
| control_nested_router | routing | False | 5.5671 |
| paradigm_break_manual_dict_response | serialization | True | 7.3289 |
| paradigm_break_msgpack_response | serialization | True | 13.4844 |
| control_response_model_list | serialization | False | 6.2612 |
| paradigm_break_assert_validation | validation | True | 4.6567 |
| control_field_validator | validation | False | 10.2415 |

## Interpretation

AUC 0.5532 < 0.90 — FastAPI gate not cleared. **Decision: abandon contrastive-JEPA.**

### Failure mode

Two overlapping problems cancel the contrastive signal:

**1. JEPA_A is poorly calibrated on high-scoring controls.**
`control_background_tasks_depends` (14.59), `control_field_validator` (10.24), and
`control_anyio_thread_offload` (10.21) score above most breaks. JEPA_A was trained with
`corpus_cap=2000` using linear (first-N) sampling, so it sees only the earliest 2000 records
in `corpus_file_only.jsonl`. FastAPI patterns from background_tasks and async validation
appear late in commit history and are underrepresented — the predictor treats them as
out-of-distribution even though they are idiomatic FastAPI.

**2. err_B (raw embedding distance) co-varies with err_A for real breaks.**
For paradigm breaks, the hunk embedding is semantically distant from the context embedding
(argparse/tornado/flask code vs FastAPI context). This pushes err_B high for exactly the
same breaks that push err_A high. The subtraction `max(err_A[d] − err_B[d])` cancels most
of the signal. `paradigm_break_assert_validation` (4.66) and `paradigm_break_flask_routing`
(5.39) score below several controls, confirming the cancellation.

### Root cause

`err_B = (ctx_embed − hunk_embed)²` with a frozen pretrained encoder is not a valid
"generic Python baseline." It is a raw embedding distance that increases for *any* transition
where context and hunk are semantically dissimilar — which includes both genuine paradigm
breaks AND idiomatic FastAPI patterns that happen to be stylistically distinct from their
context window. The contrastive formulation requires a JEPA_B that predicts well on general
Python code (low err_B for Python-normal hunks) but does not specialise for this repo.
A frozen-encoder identity predictor cannot fulfil that role.

**Next step:** contrastive-JEPA with identity JEPA_B is not the answer. The contrastive
log-ratio structure that worked for contrastive_tfidf requires a meaningful cross-entropy
reference (P_B = generic token distribution), not a raw embedding distance. Consider
contrastive-MLM (CodeBERT log P_B(t)) as the context-aware successor to contrastive_tfidf,
or return to optimising the existing ast_contrastive_max scorer directly.

