# Acceptance Test: httpx

| fixture | scope | score | type |
|---|---|---|---|
| paradigm_break_requests_session_mount | default | 1.0766 | break |
| paradigm_break_urllib3_pool | default | 1.2123 | break |
| paradigm_break_aiohttp_session | default | 1.1732 | break |
| paradigm_break_sync_in_async | default | 1.2296 | break |
| paradigm_break_raw_socket | default | 1.1857 | break |
| control_client_context_manager | default | 0.9817 | control |
| control_async_client_transport | default | 0.8594 | control |
| paradigm_break_subtle_wrong_exception | default | 0.6773 | break |
| paradigm_break_subtle_status_check | default | 0.9332 | break |
| paradigm_break_subtle_sync_in_async_context | default | 1.1245 | break |
| paradigm_break_subtle_exception_swallow | default | 0.8461 | break |

**[default]** control=0.9206  break=1.0509  delta=0.1304  NO-GO ✗

**Overall:** NO-GO ✗
