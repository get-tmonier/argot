# Phase 13 Stage 2 — Contrastive AST Validation

Baseline AUC (FastAPI, model_A=control files): **0.9742**


## Tier 1.1 — Smoke Test (model_A = model_B = stdlib)


Expected: all scores ≈ 0.  **Result: PASS** (max |score| = 0.0000)


| fixture | is_break | score |
|---|---|---|
| paradigm_break_flask_routing | True | 0.0000 |
| paradigm_break_django_cbv | True | 0.0000 |
| paradigm_break_aiohttp_handler | True | 0.0000 |
| paradigm_break_manual_validation | True | 0.0000 |
| paradigm_break_subtle_wrong_exception | True | 0.0000 |
| paradigm_break_subtle_manual_status_check | True | 0.0000 |
| paradigm_break_subtle_sync_endpoint | True | 0.0000 |
| paradigm_break_subtle_exception_swallow | True | 0.0000 |
| control_router_endpoint | False | 0.0000 |
| control_dependency_injection | False | 0.0000 |
| control_exception_handling | False | 0.0000 |
| paradigm_break_starlette_mount | True | 0.0000 |
| paradigm_break_tornado_handler | True | 0.0000 |
| paradigm_break_voluptuous_validation | True | 0.0000 |
| paradigm_break_cerberus_validation | True | 0.0000 |
| paradigm_break_manual_json_response | True | 0.0000 |
| paradigm_break_bare_except | True | 0.0000 |
| paradigm_break_event_loop_blocking | True | 0.0000 |
| paradigm_break_sync_requests_in_async | True | 0.0000 |
| paradigm_break_concurrent_futures_background | True | 0.0000 |
| control_pydantic_validator | False | 0.0000 |
| control_response_model | False | 0.0000 |
| control_async_streaming | False | 0.0000 |
| control_httpx_async | False | 0.0000 |
| control_annotated_depends | False | 0.0000 |
| paradigm_break_sync_file_io_async | True | 0.0000 |
| control_anyio_thread_offload | False | 0.0000 |
| paradigm_break_multiprocessing_background | True | 0.0000 |
| paradigm_break_queue_carryover | True | 0.0000 |
| paradigm_break_atexit_background | True | 0.0000 |
| control_background_tasks_basic | False | 0.0000 |
| control_background_tasks_depends | False | 0.0000 |
| paradigm_break_manual_generator_drain | True | 0.0000 |
| paradigm_break_class_instance_no_depends | True | 0.0000 |
| control_nested_depends | False | 0.0000 |
| paradigm_break_aiohttp_no_context | True | 0.0000 |
| control_httpx_depends | False | 0.0000 |
| paradigm_break_json_error_response | True | 0.0000 |
| paradigm_break_traceback_in_response | True | 0.0000 |
| paradigm_break_flask_errorhandler | True | 0.0000 |
| control_exception_handler_registration | False | 0.0000 |
| control_lifespan_context | False | 0.0000 |
| control_apirouter_composition | False | 0.0000 |
| control_mounted_subapp | False | 0.0000 |
| paradigm_break_imperative_route_loop | True | 0.0000 |
| control_nested_router | False | 0.0000 |
| paradigm_break_manual_dict_response | True | 0.0000 |
| paradigm_break_msgpack_response | True | 0.0000 |
| control_response_model_list | False | 0.0000 |
| paradigm_break_assert_validation | True | 0.0000 |
| control_field_validator | False | 0.0000 |

## Tier 1.2 — Leave-One-Out over Control Files


| excluded file | AUC |
|---|---|
| control_annotated_depends.py | 0.9742 |
| control_anyio_thread_offload.py | 0.9339 |
| control_apirouter_composition.py | 0.9710 |
| control_async_streaming.py | 0.9379 |
| control_background_tasks_basic.py | 0.9726 |
| control_background_tasks_depends.py | 0.9315 |
| control_dependency_injection.py | 0.9331 |
| control_exception_handler_registration.py | 0.9742 |
| control_exception_handling.py | 0.9815 |
| control_field_validator.py | 0.9548 |
| control_httpx_async.py | 0.9742 |
| control_httpx_depends.py | 0.9742 |
| control_lifespan_context.py | 0.9742 |
| control_mounted_subapp.py | 0.9694 |
| control_nested_depends.py | 0.9532 |
| control_nested_router.py | 0.9710 |
| control_pydantic_validator.py | 0.9742 |
| control_response_model.py | 0.9742 |
| control_response_model_list.py | 0.9742 |
| control_router_endpoint.py | 0.9339 |

**min AUC:** 0.9315  **mean AUC:** 0.9619  **max AUC:** 0.9815


## Tier 2 — Wrong-Contrast (Django view files as model_A)


Django repo: `https://github.com/django/django` tag `4.2.16`


model_A files (11):
- `django/views/generic/__init__.py`
- `django/views/generic/base.py`
- `django/views/generic/dates.py`
- `django/views/generic/detail.py`
- `django/views/generic/edit.py`
- `django/views/generic/list.py`
- `django/contrib/auth/views.py`
- `django/contrib/admin/views/__init__.py`
- `django/contrib/admin/views/autocomplete.py`
- `django/contrib/admin/views/decorators.py`
- `django/contrib/admin/views/main.py`

**AUC with Django model_A:** 0.5323  (Δ vs baseline: -0.4419)


## Verdict


All three tiers confirm the scorer is genuinely contrastive. Tier 1.1: replacing model_A with the same stdlib corpus yields exactly zero signal on every fixture (max |score| = 0.0000), proving the formula is well-defined. Tier 1.2: LOO AUC stays between 0.93 and 0.98 (mean 0.96) across all 20 leave-one-out folds, showing stability with no single control file carrying the result. Tier 2 is the decisive test: swapping model_A to Django view files collapses AUC from 0.97 to 0.53 (Δ = −0.44), near-chance, confirming the scorer measures repo-voice contrast rather than generic stdlib-relative novelty. **Promote to Stage 3.**
