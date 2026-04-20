# httpx

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine | tfidf_anomaly | lof_embedding | lm_perplexity |
|---|---|---|---|---|---|---|---|
| paradigm_break_requests_session_mount | default | break | 1.1192 | 0.5660 | 0.7427 | 1.1572 | 1.5140 |
| paradigm_break_urllib3_pool | default | break | 1.2088 | 0.6832 | 0.7918 | 1.3918 | 1.8794 |
| paradigm_break_aiohttp_session | default | break | 1.1940 | 0.6692 | 0.7933 | 1.3422 | 1.8039 |
| paradigm_break_sync_in_async | default | break | 1.2789 | 0.6383 | 0.7928 | 1.2797 | 1.8515 |
| paradigm_break_raw_socket | default | break | 1.2137 | 0.6489 | 0.7684 | 1.3909 | 1.4178 |
| control_client_context_manager | default | control | 1.0267 | 0.4634 | 0.5111 | 1.0687 | 1.6462 |
| control_async_client_transport | default | control | 0.8630 | 0.4377 | 0.5101 | 0.9969 | 1.3216 |
| paradigm_break_subtle_wrong_exception | default | break | 0.6867 | 0.5987 | 0.8026 | 1.2186 | 1.6751 |
| paradigm_break_subtle_status_check | default | break | 0.9329 | 0.6374 | 0.7470 | 1.4480 | 0.8440 |
| paradigm_break_subtle_sync_in_async_context | default | break | 1.1337 | 0.6714 | 0.7568 | 1.3440 | 1.5936 |
| paradigm_break_subtle_exception_swallow | default | break | 0.8520 | 0.6279 | 0.7716 | 1.2386 | 1.0714 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank | tfidf_anomaly rank | lof_embedding rank | lm_perplexity rank |
|---|---|---|---|---|---|---|
| paradigm_break_requests_session_mount | break | 6/11 | 9/11 | 9/11 | 9/11 | 7/11 |
| paradigm_break_urllib3_pool | break | 3/11 | 1/11 | 4/11 | 2/11 | 1/11 |
| paradigm_break_aiohttp_session | break | 4/11 | 3/11 | 2/11 | 5/11 | 3/11 |
| paradigm_break_sync_in_async | break | 1/11 | 5/11 | 3/11 | 6/11 | 2/11 |
| paradigm_break_raw_socket | break | 2/11 | 4/11 | 6/11 | 3/11 | 8/11 |
| control_client_context_manager | control | 7/11 | 10/11 | 10/11 | 10/11 | 5/11 |
| control_async_client_transport | control | 9/11 | 11/11 | 11/11 | 11/11 | 9/11 |
| paradigm_break_subtle_wrong_exception | break | 11/11 | 8/11 | 1/11 | 8/11 | 4/11 |
| paradigm_break_subtle_status_check | break | 8/11 | 6/11 | 8/11 | 1/11 | 11/11 |
| paradigm_break_subtle_sync_in_async_context | break | 5/11 | 2/11 | 7/11 | 4/11 | 6/11 |
| paradigm_break_subtle_exception_swallow | break | 10/11 | 7/11 | 5/11 | 7/11 | 10/11 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0689 | 0.9448 | 0.1241 | ✗ |
| knn_cosine | 0.6379 | 0.4506 | 0.1873 | ✗ |
| tfidf_anomaly | 0.7741 | 0.5106 | 0.2635 | ✓ |
| lof_embedding | 1.3123 | 1.0328 | 0.2795 | ✓ |
| lm_perplexity | 1.5167 | 1.4839 | 0.0328 | ✗ |
