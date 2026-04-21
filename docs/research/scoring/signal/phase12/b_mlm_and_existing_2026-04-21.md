# Phase 12 S4 — MLM Surprise Bakeoff

Date: 2026-04-21  Entry: `fastapi`  Context mode: `file_only`

> Phase 11 production winner: **EnsembleJepa mean_z @ file_only** AUC=0.6532

> Pairwise CI computed vs `tfidf_anomaly` (paired bootstrap, n=1000, α=0.05).


## Summary: Overall AUC

| scorer | overall_auc | delta_ci_lo | delta_ci_hi |
|---|---|---|---|
| mlm_surprise_mean | 0.4290 | -0.4452 | -0.1000 |
| mlm_surprise_min | 0.4129 | -0.4871 | -0.0839 |
| mlm_surprise_p05 | 0.4129 | -0.5065 | -0.0645 |
| tfidf_anomaly | 0.6968 | +0.0000 | +0.0000 |
| knn_cosine | 0.5306 | -0.3355 | +0.0113 |
| lof_embedding | 0.3919 | -0.4790 | -0.1274 |
| lm_perplexity | 0.2839 | -0.5984 | -0.2048 |
| ast_structural_ll | 0.4387 | -0.4290 | -0.0823 |
| ast_structural_zscore | 0.4887 | -0.3742 | -0.0355 |
| ast_structural_oov | 0.4387 | -0.4290 | -0.0823 |

## Per-Category AUC

| scorer | async_blocking | background_tasks | dependency_injection | downstream_http | exception_handling | framework_swap | routing | serialization | validation |
|---|---|---|---|---|---|---|---|---|---|
| mlm_surprise_mean | 0.1667 | 0.0000 | 0.3333 | 0.3333 | 0.1667 | 0.4444 | 0.6667 | 0.6667 | 0.5000 |
| mlm_surprise_min | 0.5000 | 0.5000 | 0.3333 | 0.3333 | 0.2500 | 0.1111 | 0.6667 | 0.5000 | 0.7500 |
| mlm_surprise_p05 | 0.0000 | 0.7500 | 0.3333 | 0.3333 | 0.2500 | 0.3333 | 0.6667 | 0.8333 | 0.8750 |
| tfidf_anomaly | 0.8333 | 0.3750 | 0.6667 | 1.0000 | 0.5000 | 0.6667 | 0.6667 | 1.0000 | 0.7500 |
| knn_cosine | 0.5000 | 0.7500 | 0.5000 | 0.6667 | 0.5833 | 0.7778 | 0.5000 | 0.6667 | 0.3750 |
| lof_embedding | 0.3333 | 0.0000 | 0.5000 | 0.0000 | 0.1667 | 0.3333 | 1.0000 | 0.6667 | 0.3750 |
| lm_perplexity | 0.0000 | 0.7500 | 0.0000 | 0.1667 | 0.0833 | 0.2222 | 0.6667 | 0.3333 | 0.0000 |
| ast_structural_ll | 0.2500 | 0.6250 | 0.5000 | 0.5000 | 0.2500 | 0.3333 | 0.5000 | 0.5000 | 0.5000 |
| ast_structural_zscore | 0.2500 | 0.6250 | 0.5000 | 0.5000 | 0.7500 | 0.3333 | 0.5000 | 0.5000 | 0.5000 |
| ast_structural_oov | 0.2500 | 0.6250 | 0.5000 | 0.5000 | 0.2500 | 0.3333 | 0.5000 | 0.5000 | 0.5000 |

## Per-Fixture Scores

| fixture | category | type | mlm_surprise_mean | mlm_surprise_min | mlm_surprise_p05 | tfidf_anomaly | knn_cosine | lof_embedding | lm_perplexity | ast_structural_ll | ast_structural_zscore | ast_structural_oov |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | break | 11.2769 | 14.2380 | 12.8207 | 0.7788 | 0.5718 | 1.3568 | 1.5128 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_django_cbv | framework_swap | break | 10.9353 | 14.8640 | 13.1876 | 0.7779 | 0.6030 | 1.5218 | 0.9718 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_aiohttp_handler | framework_swap | break | 11.1096 | 14.8384 | 12.7842 | 0.7132 | 0.5949 | 1.3871 | 1.1564 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_manual_validation | validation | break | 11.3587 | 15.1400 | 13.3231 | 0.7020 | 0.5517 | 1.4164 | 0.6593 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_subtle_wrong_exception | exception_handling | break | 10.8711 | 15.3562 | 12.8714 | 0.6611 | 0.5592 | 1.2830 | 1.4854 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_subtle_manual_status_check | downstream_http | break | 10.9389 | 13.7485 | 12.7087 | 0.6877 | 0.6013 | 1.4209 | 0.5464 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_subtle_sync_endpoint | async_blocking | break | 11.1501 | 15.3583 | 13.3143 | 0.7577 | 0.4762 | 1.2312 | 1.3503 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_subtle_exception_swallow | exception_handling | break | 10.9181 | 14.7236 | 12.9789 | 0.6794 | 0.5665 | 1.3424 | 1.0162 | 0.0000 | 0.0000 | 0.0000 |
| control_router_endpoint | routing | control | 11.4370 | 15.4429 | 13.3366 | 0.7153 | 0.5563 | 1.2660 | 1.1462 | 0.0000 | 0.0000 | 0.0000 |
| control_dependency_injection | dependency_injection | control | 11.3165 | 17.0559 | 13.6310 | 0.6642 | 0.5434 | 1.2231 | 1.5434 | 0.0000 | 0.0000 | 0.0000 |
| control_exception_handling | exception_handling | control | 11.3184 | 15.8499 | 13.3727 | 0.5952 | 0.3838 | 1.3928 | 1.3716 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_starlette_mount | routing | break | 11.4226 | 16.1279 | 13.6719 | 0.7051 | 0.4003 | 1.4444 | 1.0683 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_tornado_handler | framework_swap | break | 11.1402 | 15.4292 | 13.3311 | 0.5308 | 0.6106 | 1.5044 | 1.1902 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_voluptuous_validation | validation | break | 11.1524 | 15.1220 | 13.4550 | 0.6327 | 0.5873 | 1.7289 | 1.2928 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_cerberus_validation | validation | break | 11.2408 | 15.6847 | 13.4553 | 0.7223 | 0.4373 | 1.5367 | 1.1702 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_manual_json_response | serialization | break | 11.0784 | 15.2380 | 13.5924 | 0.7118 | 0.6172 | 1.7235 | 1.1191 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_bare_except | exception_handling | break | 11.4346 | 15.9319 | 13.2321 | 0.7647 | 0.3773 | 1.5095 | 1.2934 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_event_loop_blocking | async_blocking | break | 11.0023 | 15.7364 | 13.3377 | 0.6417 | 0.3880 | 1.3152 | 1.0090 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_sync_requests_in_async | downstream_http | break | 11.2297 | 14.5783 | 13.1853 | 0.7326 | 0.3763 | 1.7728 | 1.1950 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_concurrent_futures_background | background_tasks | break | 11.4562 | 15.5779 | 13.7982 | 0.7804 | 0.6149 | 1.4373 | 1.2986 | 0.0000 | 0.0000 | 0.0000 |
| control_pydantic_validator | validation | control | 11.0238 | 14.1883 | 13.2188 | 0.5991 | 0.3265 | 1.1254 | 1.7797 | 0.0000 | 0.0000 | 0.0000 |
| control_response_model | serialization | control | 10.9639 | 14.6416 | 13.1208 | 0.6126 | 0.5254 | 1.6003 | 1.2757 | 0.0000 | 0.0000 | 0.0000 |
| control_async_streaming | async_blocking | control | 11.2215 | 14.7093 | 13.6552 | 0.5963 | 0.4737 | 1.3704 | 1.5656 | 0.0000 | 0.0000 | 0.0000 |
| control_httpx_async | downstream_http | control | 11.2832 | 15.7755 | 13.4017 | 0.6538 | 0.5585 | 2.0794 | 1.4208 | 0.0000 | 0.0000 | 0.0000 |
| control_annotated_depends | dependency_injection | control | 11.0247 | 14.5021 | 13.0235 | 0.5505 | 0.5767 | 1.6903 | 1.4626 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_sync_file_io_async | async_blocking | break | 11.3255 | 14.6196 | 13.4345 | 0.7484 | 0.6646 | 1.7387 | 1.3708 | 0.0000 | 0.0000 | 0.0000 |
| control_anyio_thread_offload | async_blocking | control | 11.4287 | 15.5442 | 14.0046 | 0.7181 | 0.6608 | 1.5665 | 1.4245 | 26.8505 | 5.3214 | 5.0000 |
| paradigm_break_multiprocessing_background | background_tasks | break | 11.3281 | 15.1097 | 13.5499 | 0.7084 | 0.6516 | 1.5304 | 1.2383 | 4.7958 | 1.9858 | 2.0000 |
| paradigm_break_queue_carryover | background_tasks | break | 11.6285 | 14.4107 | 13.1674 | 0.7333 | 0.7821 | 1.4493 | 1.9495 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_atexit_background | background_tasks | break | 11.5725 | 15.3776 | 13.1773 | 0.7439 | 0.6225 | 1.6507 | 1.9947 | 0.0000 | 0.0000 | 0.0000 |
| control_background_tasks_basic | background_tasks | control | 11.9291 | 14.4464 | 13.5323 | 0.7886 | 0.5758 | 1.7541 | 1.1864 | 0.0000 | 0.0000 | 0.0000 |
| control_background_tasks_depends | background_tasks | control | 11.7087 | 15.4398 | 13.0593 | 0.7204 | 0.6346 | 1.6888 | 1.6594 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_manual_generator_drain | dependency_injection | break | 11.2140 | 15.1117 | 13.3297 | 0.7271 | 0.4724 | 1.8160 | 1.4460 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_class_instance_no_depends | dependency_injection | break | 11.1870 | 15.1760 | 13.6112 | 0.5975 | 0.7096 | 1.6653 | 0.9587 | 0.0000 | 0.0000 | 0.0000 |
| control_nested_depends | dependency_injection | control | 11.4441 | 15.4317 | 13.8829 | 0.7215 | 0.6593 | 2.1332 | 1.5591 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_aiohttp_no_context | downstream_http | break | 11.1474 | 16.4528 | 13.2447 | 0.7242 | 0.6291 | 1.5875 | 1.2208 | 0.0000 | 0.0000 | 0.0000 |
| control_httpx_depends | downstream_http | control | 11.0722 | 15.9417 | 13.0398 | 0.6468 | 0.5767 | 1.8004 | 1.2018 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_json_error_response | exception_handling | break | 11.3793 | 14.7056 | 13.5377 | 0.5862 | 0.3994 | 1.3113 | 1.0527 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_traceback_in_response | exception_handling | break | 11.2166 | 14.1067 | 13.5055 | 0.5791 | 0.3635 | 1.2845 | 1.0484 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_flask_errorhandler | exception_handling | break | 11.1635 | 14.3732 | 13.4148 | 0.5519 | 0.4246 | 1.2974 | 1.3534 | 0.0000 | 0.0000 | 0.0000 |
| control_exception_handler_registration | exception_handling | control | 11.4996 | 15.1027 | 13.9383 | 0.6056 | 0.4111 | 1.3882 | 1.8059 | 16.7282 | -1.9195 | 8.0000 |
| control_lifespan_context | framework_swap | control | 11.1349 | 15.8920 | 13.5779 | 0.6637 | 0.5029 | 2.2025 | 1.7729 | 0.0000 | 0.0000 | 0.0000 |
| control_apirouter_composition | framework_swap | control | 11.5317 | 16.2175 | 13.8383 | 0.6095 | 0.3728 | 1.3280 | 1.0400 | 0.0000 | 0.0000 | 0.0000 |
| control_mounted_subapp | framework_swap | control | 10.9028 | 14.9006 | 12.7772 | 0.5846 | 0.6069 | 1.9796 | 1.5294 | 25.9254 | 3.3497 | 4.0000 |
| paradigm_break_imperative_route_loop | routing | break | 11.7803 | 16.9384 | 13.9507 | 0.6156 | 0.4223 | 1.5076 | 2.4680 | 0.0000 | 0.0000 | 0.0000 |
| control_nested_router | routing | control | 11.1549 | 14.9766 | 13.4457 | 0.4954 | 0.4005 | 1.3395 | 1.0890 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_manual_dict_response | serialization | break | 11.1357 | 14.4857 | 13.2818 | 0.6636 | 0.3859 | 1.5322 | 0.9416 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_msgpack_response | serialization | break | 11.2106 | 14.8292 | 13.5424 | 0.7574 | 0.6017 | 1.8012 | 1.3284 | 0.0000 | 0.0000 | 0.0000 |
| control_response_model_list | serialization | control | 11.2022 | 14.8738 | 13.4615 | 0.5384 | 0.4261 | 1.6135 | 1.1232 | 0.0000 | 0.0000 | 0.0000 |
| paradigm_break_assert_validation | validation | break | 11.3062 | 15.3869 | 13.4859 | 0.6490 | 0.2737 | 1.0648 | 0.9921 | 0.0000 | 0.0000 | 0.0000 |
| control_field_validator | validation | control | 11.4136 | 15.1599 | 13.4158 | 0.6968 | 0.6142 | 1.8395 | 1.6398 | 0.0000 | 0.0000 | 0.0000 |
