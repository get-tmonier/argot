# ky

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine |
|---|---|---|---|---|
| paradigm_break_xhr | default | break | 1.0722 | 0.4878 |
| paradigm_break_callback | default | break | 0.8889 | 0.5328 |
| paradigm_break_class_client | default | break | 0.9498 | 0.4788 |
| paradigm_break_explicit_promise | default | break | 1.0694 | 0.4244 |
| paradigm_break_interceptors | default | break | 0.8620 | 0.5192 |
| control_options_normalization | default | control | 0.7477 | 0.4310 |
| control_response_handling | default | control | 0.7527 | 0.4259 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank |
|---|---|---|---|
| paradigm_break_xhr | break | 1/7 | 3/7 |
| paradigm_break_callback | break | 4/7 | 1/7 |
| paradigm_break_class_client | break | 3/7 | 4/7 |
| paradigm_break_explicit_promise | break | 2/7 | 7/7 |
| paradigm_break_interceptors | break | 5/7 | 2/7 |
| control_options_normalization | control | 7/7 | 5/7 |
| control_response_handling | control | 6/7 | 6/7 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 0.9685 | 0.7502 | 0.2183 | ✓ |
| knn_cosine | 0.4886 | 0.4285 | 0.0601 | ✗ |
