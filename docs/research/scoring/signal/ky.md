# ky

## Raw Scores

| fixture | scope | type | jepa_pretrained | knn_cosine | tfidf_anomaly | lof_embedding | lm_perplexity |
|---|---|---|---|---|---|---|---|
| paradigm_break_xhr | default | break | 1.0792 | 0.4878 | 0.7439 | 1.1003 | 1.4822 |
| paradigm_break_callback | default | break | 0.9180 | 0.5328 | 0.7405 | 1.1165 | 1.9619 |
| paradigm_break_class_client | default | break | 0.9576 | 0.4788 | 0.7989 | 1.1288 | 1.6251 |
| paradigm_break_explicit_promise | default | break | 1.0532 | 0.4244 | 0.7552 | 1.0828 | 1.5531 |
| paradigm_break_interceptors | default | break | 0.8520 | 0.5192 | 0.7997 | 1.1901 | 2.1724 |
| control_options_normalization | default | control | 0.7374 | 0.4310 | 0.7161 | 1.0311 | 2.0910 |
| control_response_handling | default | control | 0.7808 | 0.4259 | 0.7062 | 1.0350 | 2.2204 |

## Ranks (1 = most anomalous)

| fixture | type | jepa_pretrained rank | knn_cosine rank | tfidf_anomaly rank | lof_embedding rank | lm_perplexity rank |
|---|---|---|---|---|---|---|
| paradigm_break_xhr | break | 1/7 | 3/7 | 4/7 | 4/7 | 7/7 |
| paradigm_break_callback | break | 4/7 | 1/7 | 5/7 | 3/7 | 4/7 |
| paradigm_break_class_client | break | 3/7 | 4/7 | 2/7 | 2/7 | 5/7 |
| paradigm_break_explicit_promise | break | 2/7 | 7/7 | 3/7 | 5/7 | 6/7 |
| paradigm_break_interceptors | break | 5/7 | 2/7 | 1/7 | 1/7 | 2/7 |
| control_options_normalization | control | 7/7 | 5/7 | 6/7 | 7/7 | 3/7 |
| control_response_handling | control | 6/7 | 6/7 | 7/7 | 6/7 | 1/7 |

## Summary

| scorer | break_mean | ctrl_mean | delta | gate |
|---|---|---|---|---|
| jepa_pretrained | 0.9720 | 0.7591 | 0.2129 | ✓ |
| knn_cosine | 0.4886 | 0.4285 | 0.0601 | ✗ |
| tfidf_anomaly | 0.7676 | 0.7112 | 0.0565 | ✗ |
| lof_embedding | 1.1237 | 1.0330 | 0.0907 | ✗ |
| lm_perplexity | 1.7589 | 2.1557 | -0.3968 | ✗ |
