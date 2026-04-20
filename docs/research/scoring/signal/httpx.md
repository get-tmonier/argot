# httpx

## Raw Scores

| fixture | scope | type | jepa_pretrained |
|---|---|---|---|
| paradigm_break_requests_session_mount | default | break | 1.0900 |
| paradigm_break_urllib3_pool | default | break | 1.2174 |
| paradigm_break_aiohttp_session | default | break | 1.2314 |
| paradigm_break_sync_in_async | default | break | 1.3623 |
| paradigm_break_raw_socket | default | break | 1.2722 |
| control_client_context_manager | default | control | 1.1421 |
| control_async_client_transport | default | control | 0.7644 |
| paradigm_break_subtle_wrong_exception | default | break | 0.7089 |
| paradigm_break_subtle_status_check | default | break | 0.9323 |
| paradigm_break_subtle_sync_in_async_context | default | break | 1.1518 |
| paradigm_break_subtle_exception_swallow | default | break | 0.8736 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank |
|---|---|---|
| paradigm_break_requests_session_mount | break | 7/11 |
| paradigm_break_urllib3_pool | break | 4/11 |
| paradigm_break_aiohttp_session | break | 3/11 |
| paradigm_break_sync_in_async | break | 1/11 |
| paradigm_break_raw_socket | break | 2/11 |
| control_client_context_manager | control | 6/11 |
| control_async_client_transport | control | 10/11 |
| paradigm_break_subtle_wrong_exception | break | 11/11 |
| paradigm_break_subtle_status_check | break | 8/11 |
| paradigm_break_subtle_sync_in_async_context | break | 5/11 |
| paradigm_break_subtle_exception_swallow | break | 9/11 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0933 | 0.9533 | 0.1401 | ✗ |
