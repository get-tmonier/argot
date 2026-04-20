# fastapi

## Raw Scores

| fixture | scope | type | jepa_pretrained |
|---|---|---|---|
| paradigm_break_flask_routing | default | break | 1.2514 |
| paradigm_break_django_cbv | default | break | 1.0843 |
| paradigm_break_aiohttp_handler | default | break | 1.2653 |
| paradigm_break_manual_validation | default | break | 1.4981 |
| paradigm_break_raw_response | default | break | 1.3258 |
| paradigm_break_subtle_wrong_exception | default | break | 0.9133 |
| paradigm_break_subtle_manual_status_check | default | break | 1.3566 |
| paradigm_break_subtle_sync_endpoint | default | break | 0.9238 |
| paradigm_break_subtle_exception_swallow | default | break | 0.9194 |
| control_router_endpoint | default | control | 1.0774 |
| control_dependency_injection | default | control | 0.9708 |
| control_exception_handling | default | control | 0.7999 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank |
|---|---|---|
| paradigm_break_flask_routing | break | 5/12 |
| paradigm_break_django_cbv | break | 6/12 |
| paradigm_break_aiohttp_handler | break | 4/12 |
| paradigm_break_manual_validation | break | 1/12 |
| paradigm_break_raw_response | break | 3/12 |
| paradigm_break_subtle_wrong_exception | break | 11/12 |
| paradigm_break_subtle_manual_status_check | break | 2/12 |
| paradigm_break_subtle_sync_endpoint | break | 9/12 |
| paradigm_break_subtle_exception_swallow | break | 10/12 |
| control_router_endpoint | control | 7/12 |
| control_dependency_injection | control | 8/12 |
| control_exception_handling | control | 12/12 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.1709 | 0.9494 | 0.2215 | ✓ |
