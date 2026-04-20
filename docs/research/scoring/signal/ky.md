# ky

## Raw Scores

| fixture | scope | type | jepa_pretrained |
|---|---|---|---|
| paradigm_break_xhr | default | break | 1.2009 |
| paradigm_break_callback | default | break | 0.9788 |
| paradigm_break_class_client | default | break | 1.1925 |
| paradigm_break_explicit_promise | default | break | 1.0849 |
| paradigm_break_interceptors | default | break | 0.9857 |
| control_options_normalization | default | control | 0.8623 |
| control_response_handling | default | control | 0.8744 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank |
|---|---|---|
| paradigm_break_xhr | break | 1/7 |
| paradigm_break_callback | break | 5/7 |
| paradigm_break_class_client | break | 2/7 |
| paradigm_break_explicit_promise | break | 3/7 |
| paradigm_break_interceptors | break | 4/7 |
| control_options_normalization | control | 7/7 |
| control_response_handling | control | 6/7 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 1.0886 | 0.8684 | 0.2202 | ✓ |
