# Phase 13 — Contrastive-MLM Experiment (FastAPI, 2026-04-21)


## Summary


| scorer | corpus | approach | AUC |
|---|---|---|---|
| contrastive_tfidf (word) | FastAPI | marginal token freq | 0.9847 |
| bpe_contrastive_tfidf | FastAPI | marginal BPE freq | 1.0000 |
| contrastive_jepa | FastAPI | sentence embedding | 0.5532 |
| **contrastive_mlm** | **FastAPI** | **conditional MLM log-ratio** | **0.4645** |

## Per-Category AUC


*(break category vs all controls)*


| category | n_breaks | AUC |
|---|---|---|
| async_blocking | 3 | 0.8167 |
| background_tasks | 4 | 0.5750 |
| dependency_injection | 2 | 0.0500 |
| downstream_http | 3 | 0.5333 |
| exception_handling | 6 | 0.5417 |
| framework_swap | 3 | 0.1667 |
| routing | 3 | 0.5667 |
| serialization | 3 | 0.2167 |
| validation | 4 | 0.4625 |

## Fixture Scores


| fixture | category | is_break | max_score | mean_score | top5_mean |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | True | 4.2899 | -0.4367 | 2.4014 |
| paradigm_break_django_cbv | framework_swap | True | 1.5044 | -0.3351 | 1.2329 |
| paradigm_break_aiohttp_handler | framework_swap | True | 1.7421 | -0.4688 | 1.2903 |
| paradigm_break_manual_validation | validation | True | 2.0796 | -0.3824 | 0.9599 |
| paradigm_break_subtle_wrong_exception | exception_handling | True | 3.7979 | -0.6319 | 2.1958 |
| paradigm_break_subtle_manual_status_check | downstream_http | True | 4.8364 | -0.4341 | 1.5606 |
| paradigm_break_subtle_sync_endpoint | async_blocking | True | 4.6244 | -0.4395 | 2.3936 |
| paradigm_break_subtle_exception_swallow | exception_handling | True | 1.6305 | -0.7295 | 0.9546 |
| control_router_endpoint | routing | False | 2.7985 | -0.5210 | 1.9937 |
| control_dependency_injection | dependency_injection | False | 4.0253 | -0.6762 | 2.0049 |
| control_exception_handling | exception_handling | False | 3.5320 | -0.5446 | 2.2027 |
| paradigm_break_starlette_mount | routing | True | 2.8556 | -0.5001 | 1.6611 |
| paradigm_break_tornado_handler | framework_swap | True | 2.1055 | -0.4844 | 1.6889 |
| paradigm_break_voluptuous_validation | validation | True | 4.7582 | -0.6255 | 2.7413 |
| paradigm_break_cerberus_validation | validation | True | 2.3615 | -0.5424 | 1.4969 |
| paradigm_break_manual_json_response | serialization | True | 1.3575 | -0.5553 | 0.7832 |
| paradigm_break_bare_except | exception_handling | True | 4.0963 | -0.6688 | 1.7937 |
| paradigm_break_event_loop_blocking | async_blocking | True | 2.6957 | -0.4333 | 2.2660 |
| paradigm_break_sync_requests_in_async | downstream_http | True | 2.4827 | -0.6372 | 1.5823 |
| paradigm_break_concurrent_futures_background | background_tasks | True | 3.0077 | -0.4151 | 2.4236 |
| control_pydantic_validator | validation | False | 2.4101 | -0.6465 | 1.8731 |
| control_response_model | serialization | False | 2.0081 | -0.5991 | 1.1389 |
| control_async_streaming | async_blocking | False | 1.4611 | -0.4770 | 1.2609 |
| control_httpx_async | downstream_http | False | 2.3776 | -0.5885 | 1.6470 |
| control_annotated_depends | dependency_injection | False | 2.4901 | -0.6940 | 1.5148 |
| paradigm_break_sync_file_io_async | async_blocking | True | 3.7194 | -0.4324 | 2.1139 |
| control_anyio_thread_offload | async_blocking | False | 2.8425 | -0.5327 | 2.2226 |
| paradigm_break_multiprocessing_background | background_tasks | True | 1.8047 | -0.5130 | 1.4604 |
| paradigm_break_queue_carryover | background_tasks | True | 3.4380 | -0.5772 | 1.9045 |
| paradigm_break_atexit_background | background_tasks | True | 2.5420 | -0.6019 | 1.9899 |
| control_background_tasks_basic | background_tasks | False | 2.7394 | -0.5375 | 1.5954 |
| control_background_tasks_depends | background_tasks | False | 1.9839 | -0.6926 | 1.5474 |
| paradigm_break_manual_generator_drain | dependency_injection | True | 1.5629 | -0.6908 | 1.2078 |
| paradigm_break_class_instance_no_depends | dependency_injection | True | 1.1437 | -0.6624 | 0.6274 |
| control_nested_depends | dependency_injection | False | 2.0154 | -0.6073 | 0.8237 |
| paradigm_break_aiohttp_no_context | downstream_http | True | 1.7292 | -0.4129 | 1.1055 |
| control_httpx_depends | downstream_http | False | 1.7846 | -0.5815 | 1.2243 |
| paradigm_break_json_error_response | exception_handling | True | 4.2994 | -0.5499 | 1.9865 |
| paradigm_break_traceback_in_response | exception_handling | True | 1.2234 | -0.5499 | 0.9031 |
| paradigm_break_flask_errorhandler | exception_handling | True | 2.2879 | -0.5175 | 0.9090 |
| control_exception_handler_registration | exception_handling | False | 4.5479 | -0.5369 | 3.0596 |
| control_lifespan_context | framework_swap | False | 3.2922 | -0.5416 | 1.9204 |
| control_apirouter_composition | framework_swap | False | 2.9920 | -0.6029 | 1.4582 |
| control_mounted_subapp | framework_swap | False | 3.4060 | -0.7099 | 2.2800 |
| paradigm_break_imperative_route_loop | routing | True | 1.4481 | -0.4218 | 1.0846 |
| control_nested_router | routing | False | 2.3323 | -0.8251 | 1.7792 |
| paradigm_break_manual_dict_response | serialization | True | 1.4278 | -0.5173 | 1.1143 |
| paradigm_break_msgpack_response | serialization | True | 2.5831 | -0.5606 | 1.6810 |
| control_response_model_list | serialization | False | 1.2931 | -0.5364 | 0.8609 |
| paradigm_break_assert_validation | validation | True | 1.9527 | -0.5512 | 1.4267 |
| control_field_validator | validation | False | 2.2705 | -0.7265 | 1.7507 |

## Held-out vocabulary


N/A: FastAPI has no held-out vocabulary split


## Interpretation

**Gate: FAILED** — AUC 0.4645 < 0.85. Do not proceed to click.

Max-token saturation resolved: 31/31 unique break scores.

### Why 0.4645 — diagnosis from the fixture data

**All mean scores are negative** (breaks: −0.41 to −0.73; controls: −0.51 to −0.83).
Every single fixture — break or control — has model A (adapters ON) outperforming model B
(vanilla CodeBERT) on average. This confirms the root cause: 1 epoch of LoRA on 20 files
taught the adapter general Python fluency, making it marginally better at *all* Python tokens
including those in break fixtures. The contrastive formula `log P_B − log P_A` is therefore
negative for most positions everywhere, giving the wrong sign.

**The `max` aggregation picks up noise, not signal.** Controls like
`control_exception_handler_registration` (max=4.55) and `control_dependency_injection`
(max=4.03) outscore many break fixtures. The tokens driving the max are generic
(`dict`, `global`, whitespace, line numbers) — not framework-specific idioms.

**One category works: `async_blocking` AUC 0.8167.** The three async-blocking breaks score
2.70–4.62 while async controls score 1.46–2.84. Async/sync token patterns (`async`, `await`,
blocking I/O calls) may be distinctive enough that even a weakly trained adapter creates
directional signal. This is the exception, not the rule.

**`framework_swap` inverts hardest (AUC 0.1667).** Starlette, tornado, aiohttp code uses
standard Python patterns that the adapter learned; vanilla CodeBERT finds them slightly more
surprising. Exact opposite of what we want.

### Root cause

LoRA fine-tuning on 20 control files (~200 lines each) cannot shift a 125M-parameter model's
conditional distributions in a repo-specific direction. The resulting model A is not a
"FastAPI specialist" — it is a slightly better general Python predictor. The contrastive
signal requires a large gap between A and B; here the gap is dominated by noise.

### Recommendation

Abandon MLM fine-tuning. The click ceiling (BPE-tfidf AUC 0.60) is a *context* problem,
not a vocabulary problem. The promising next directions are:
- **Zero-shot LLM perplexity** (e.g. CodeLlama) — no training, full conditional distributions
- **AST node-type distribution diff** — no model, purely structural, no OOM risk
### Top-3 argmax tokens per break fixture

- **paradigm_break_flask_routing**: ` ` (pos 73, ratio 4.290), ` port` (pos 485, ratio 2.128), ` dict` (pos 160, ratio 2.012)
- **paradigm_break_django_cbv**: ` ` (pos 105, ratio 1.504), ` dict` (pos 228, ratio 1.231), ` global` (pos 68, ratio 1.230)
- **paradigm_break_aiohttp_handler**: ` dict` (pos 190, ratio 1.742), ` global` (pos 89, ratio 1.640), `user` (pos 254, ratio 1.181)
- **paradigm_break_manual_validation**: ` global` (pos 433, ratio 2.080), `def` (pos 1, ratio 1.603), `...` (pos 17, ratio 0.667)
- **paradigm_break_subtle_wrong_exception**: ` str` (pos 429, ratio 3.798), ` del` (pos 398, ratio 2.269), `Create` (pos 140, ratio 1.960)
- **paradigm_break_subtle_manual_status_check**: `search` (pos 493, ratio 4.836), `post` (pos 118, ratio 0.914), `put` (pos 239, ratio 0.757)
- **paradigm_break_subtle_sync_endpoint**: `no` (pos 337, ratio 4.624), `Create` (pos 368, ratio 3.272), `union` (pos 509, ratio 1.649)
- **paradigm_break_subtle_exception_swallow**: `put` (pos 377, ratio 1.630), ` user` (pos 135, ratio 1.209), `model` (pos 304, ratio 0.817)
- **paradigm_break_starlette_mount**: ` dict` (pos 174, ratio 2.856), ` global` (pos 163, ratio 1.412), `line` (pos 67, ratio 1.393)
- **paradigm_break_tornado_handler**: `:` (pos 18, ratio 2.106), ` 26` (pos 19, ratio 1.884), ` dict` (pos 335, ratio 1.710)
- **paradigm_break_voluptuous_validation**: ` ignore` (pos 60, ratio 4.758), ` ` (pos 56, ratio 3.725), `return` (pos 66, ratio 1.855)
- **paradigm_break_cerberus_validation**: ` 21` (pos 34, ratio 2.362), `line` (pos 32, ratio 1.600), `errors` (pos 390, ratio 1.267)
- **paradigm_break_manual_json_response**: `updated` (pos 508, ratio 1.357), ` global` (pos 142, ratio 1.101), `get` (pos 290, ratio 0.515)
- **paradigm_break_bare_except**: ` 29` (pos 79, ratio 4.096), `line` (pos 77, ratio 1.666), ` Pay` (pos 56, ratio 1.666)
- **paradigm_break_event_loop_blocking**: `orders` (pos 238, ratio 2.696), `line` (pos 138, ratio 2.612), `id` (pos 109, ratio 2.598)
- **paradigm_break_sync_requests_in_async**: ` 21` (pos 32, ratio 2.483), `:` (pos 31, ratio 1.539), `#` (pos 24, ratio 1.393)
- **paradigm_break_concurrent_futures_background**: ` ` (pos 114, ratio 3.008), `dict` (pos 203, ratio 2.762), ` bulk` (pos 405, ratio 2.486)
- **paradigm_break_sync_file_io_async**: ` ignore` (pos 193, ratio 3.719), ` ` (pos 189, ratio 2.709), ` 40` (pos 45, ratio 1.427)
- **paradigm_break_multiprocessing_background**: ` 45` (pos 11, ratio 1.805), `line` (pos 9, ratio 1.546), `:` (pos 10, ratio 1.521)
- **paradigm_break_queue_carryover**: ` 54` (pos 79, ratio 3.438), `status` (pos 385, ratio 2.106), `()` (pos 68, ratio 1.743)
- **paradigm_break_atexit_background**: `igm` (pos 86, ratio 2.542), `user` (pos 306, ratio 2.024), ` ` (pos 249, ratio 1.940)
- **paradigm_break_manual_generator_drain**: ` create` (pos 355, ratio 1.563), ` item` (pos 276, ratio 1.330), `get` (pos 126, ratio 1.216)
- **paradigm_break_class_instance_no_depends**: `key` (pos 327, ratio 1.144), `sent` (pos 162, ratio 0.751), `422` (pos 112, ratio 0.628)
- **paradigm_break_aiohttp_no_context**: ` connector` (pos 21, ratio 1.729), ` —` (pos 20, ratio 1.532), ` async` (pos 18, ratio 0.767)
- **paradigm_break_json_error_response**: `Create` (pos 447, ratio 4.299), `#` (pos 25, ratio 1.682), ` dict` (pos 320, ratio 1.473)
- **paradigm_break_traceback_in_response**: ` dict` (pos 264, ratio 1.223), `str` (pos 266, ratio 1.132), ` max` (pos 243, ratio 0.866)
- **paradigm_break_flask_errorhandler**: `Create` (pos 354, ratio 2.288), ` max` (pos 371, ratio 0.891), ` http` (pos 131, ratio 0.659)
- **paradigm_break_imperative_route_loop**: ` 77` (pos 83, ratio 1.448), `line` (pos 81, ratio 1.420), `t` (pos 150, ratio 0.890)
- **paradigm_break_manual_dict_response**: `values` (pos 250, ratio 1.428), ` 45` (pos 9, ratio 1.222), `post` (pos 505, ratio 1.041)
- **paradigm_break_msgpack_response**: `stock` (pos 312, ratio 2.583), `#` (pos 500, ratio 1.958), `[_` (pos 254, ratio 1.436)
- **paradigm_break_assert_validation**: `item` (pos 359, ratio 1.953), ` record` (pos 348, ratio 1.451), ` create` (pos 18, ratio 1.311)

