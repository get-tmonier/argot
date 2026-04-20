# httpx

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine |
|---|---|---|---|---|
| paradigm_break_requests_session_mount | default | break | 1.1115 | 0.5660 |
| paradigm_break_urllib3_pool | default | break | 1.2157 | 0.6832 |
| paradigm_break_aiohttp_session | default | break | 1.1782 | 0.6692 |
| paradigm_break_sync_in_async | default | break | 1.2617 | 0.6383 |
| paradigm_break_raw_socket | default | break | 1.2607 | 0.6489 |
| control_client_context_manager | default | control | 0.9995 | 0.4634 |
| control_async_client_transport | default | control | 0.8593 | 0.4377 |
| paradigm_break_subtle_wrong_exception | default | break | 0.7080 | 0.5987 |
| paradigm_break_subtle_status_check | default | break | 0.9018 | 0.6374 |
| paradigm_break_subtle_sync_in_async_context | default | break | 1.1366 | 0.6714 |
| paradigm_break_subtle_exception_swallow | default | break | 0.8540 | 0.6279 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank |
|---|---|---|---|
| paradigm_break_requests_session_mount | break | 6/11 | 9/11 |
| paradigm_break_urllib3_pool | break | 3/11 | 1/11 |
| paradigm_break_aiohttp_session | break | 4/11 | 3/11 |
| paradigm_break_sync_in_async | break | 1/11 | 5/11 |
| paradigm_break_raw_socket | break | 2/11 | 4/11 |
| control_client_context_manager | control | 7/11 | 10/11 |
| control_async_client_transport | control | 9/11 | 11/11 |
| paradigm_break_subtle_wrong_exception | break | 11/11 | 8/11 |
| paradigm_break_subtle_status_check | break | 8/11 | 6/11 |
| paradigm_break_subtle_sync_in_async_context | break | 5/11 | 2/11 |
| paradigm_break_subtle_exception_swallow | break | 10/11 | 7/11 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0698 | 0.9294 | 0.1404 | ✗ |
| knn_cosine | 0.6379 | 0.4506 | 0.1873 | ✗ |
