# fastapi

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine |
|---|---|---|---|---|
| paradigm_break_flask_routing | default | break | 1.0654 | 0.5718 |
| paradigm_break_django_cbv | default | break | 1.1175 | 0.6030 |
| paradigm_break_aiohttp_handler | default | break | 1.2022 | 0.5949 |
| paradigm_break_manual_validation | default | break | 1.2731 | 0.5517 |
| paradigm_break_raw_response | default | break | 1.2391 | 0.5756 |
| paradigm_break_subtle_wrong_exception | default | break | 0.8921 | 0.5592 |
| paradigm_break_subtle_manual_status_check | default | break | 1.3252 | 0.6013 |
| paradigm_break_subtle_sync_endpoint | default | break | 0.7916 | 0.4762 |
| paradigm_break_subtle_exception_swallow | default | break | 0.9074 | 0.5665 |
| control_router_endpoint | default | control | 1.0566 | 0.5563 |
| control_dependency_injection | default | control | 0.8377 | 0.5434 |
| control_exception_handling | default | control | 0.8646 | 0.3838 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank |
|---|---|---|---|
| paradigm_break_flask_routing | break | 6/12 | 5/12 |
| paradigm_break_django_cbv | break | 5/12 | 1/12 |
| paradigm_break_aiohttp_handler | break | 4/12 | 3/12 |
| paradigm_break_manual_validation | break | 2/12 | 9/12 |
| paradigm_break_raw_response | break | 3/12 | 4/12 |
| paradigm_break_subtle_wrong_exception | break | 9/12 | 7/12 |
| paradigm_break_subtle_manual_status_check | break | 1/12 | 2/12 |
| paradigm_break_subtle_sync_endpoint | break | 12/12 | 11/12 |
| paradigm_break_subtle_exception_swallow | break | 8/12 | 6/12 |
| control_router_endpoint | control | 7/12 | 8/12 |
| control_dependency_injection | control | 11/12 | 10/12 |
| control_exception_handling | control | 10/12 | 12/12 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0904 | 0.9196 | 0.1708 | ✗ |
| knn_cosine | 0.5667 | 0.4945 | 0.0722 | ✗ |
