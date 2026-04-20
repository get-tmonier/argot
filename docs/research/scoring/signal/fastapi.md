# fastapi

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine | tfidf_anomaly | lof_embedding | lm_perplexity |
|---|---|---|---|---|---|---|---|
| paradigm_break_flask_routing | default | break | 1.0779 | 0.5718 | 0.7788 | 1.3568 | 1.7863 |
| paradigm_break_django_cbv | default | break | 1.1352 | 0.6030 | 0.7779 | 1.5218 | 1.1995 |
| paradigm_break_aiohttp_handler | default | break | 1.1976 | 0.5949 | 0.7132 | 1.3871 | 1.4096 |
| paradigm_break_manual_validation | default | break | 1.2836 | 0.5517 | 0.7020 | 1.4164 | 0.8254 |
| paradigm_break_raw_response | default | break | 1.2532 | 0.5756 | 0.7089 | 1.6280 | 1.3626 |
| paradigm_break_subtle_wrong_exception | default | break | 0.8942 | 0.5592 | 0.6611 | 1.2830 | 1.7090 |
| paradigm_break_subtle_manual_status_check | default | break | 1.2609 | 0.6013 | 0.6877 | 1.4209 | 0.6995 |
| paradigm_break_subtle_sync_endpoint | default | break | 0.7887 | 0.4762 | 0.7577 | 1.2312 | 1.5767 |
| paradigm_break_subtle_exception_swallow | default | break | 0.9181 | 0.5665 | 0.6794 | 1.3424 | 1.2163 |
| control_router_endpoint | default | control | 1.0757 | 0.5563 | 0.7153 | 1.2660 | 1.2363 |
| control_dependency_injection | default | control | 0.8117 | 0.5434 | 0.6642 | 1.2231 | 1.7646 |
| control_exception_handling | default | control | 0.8791 | 0.3838 | 0.5952 | 1.3928 | 1.6666 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank | tfidf_anomaly rank | lof_embedding rank | lm_perplexity rank |
|---|---|---|---|---|---|---|
| paradigm_break_flask_routing | break | 6/12 | 5/12 | 1/12 | 7/12 | 1/12 |
| paradigm_break_django_cbv | break | 5/12 | 1/12 | 2/12 | 2/12 | 10/12 |
| paradigm_break_aiohttp_handler | break | 4/12 | 3/12 | 5/12 | 6/12 | 6/12 |
| paradigm_break_manual_validation | break | 1/12 | 9/12 | 7/12 | 4/12 | 11/12 |
| paradigm_break_raw_response | break | 3/12 | 4/12 | 6/12 | 1/12 | 7/12 |
| paradigm_break_subtle_wrong_exception | break | 9/12 | 7/12 | 11/12 | 9/12 | 3/12 |
| paradigm_break_subtle_manual_status_check | break | 2/12 | 2/12 | 8/12 | 3/12 | 12/12 |
| paradigm_break_subtle_sync_endpoint | break | 12/12 | 11/12 | 3/12 | 11/12 | 5/12 |
| paradigm_break_subtle_exception_swallow | break | 8/12 | 6/12 | 9/12 | 8/12 | 9/12 |
| control_router_endpoint | control | 7/12 | 8/12 | 4/12 | 10/12 | 8/12 |
| control_dependency_injection | control | 11/12 | 10/12 | 10/12 | 12/12 | 2/12 |
| control_exception_handling | control | 10/12 | 12/12 | 12/12 | 5/12 | 4/12 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0899 | 0.9221 | 0.1678 | ✗ |
| knn_cosine | 0.5667 | 0.4945 | 0.0722 | ✗ |
| tfidf_anomaly | 0.7185 | 0.6582 | 0.0603 | ✗ |
| lof_embedding | 1.3986 | 1.2939 | 0.1047 | ✗ |
| lm_perplexity | 1.3094 | 1.5558 | -0.2464 | ✗ |
