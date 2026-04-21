# fastapi stage 5 sweep — context_mode=file_only

## Raw

| config | seed | auc | delta_v2 |
|---|---|---|---|
| mean_z | 0 | 0.6532 | 0.3177 |

## Summary

| config | mean_auc | std_auc | mean_delta_v2 |
|---|---|---|---|
| mean_z | 0.6532 | 0.0000 | 0.3177 |

## Per-category AUC

| category | mean_auc |
|---|---|
| async_blocking | 0.8333 |
| background_tasks | 0.3750 |
| dependency_injection | 0.3333 |
| downstream_http | 0.8333 |
| exception_handling | 0.9167 |
| framework_swap | 1.0000 |
| routing | 0.6667 |
| serialization | 0.6667 |
| validation | 0.2500 |
