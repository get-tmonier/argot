# fastapi stage 5 sweep — context_mode=siblings_only

## Raw

| config | seed | auc | delta_v2 |
|---|---|---|---|
| mean_z | 0 | 0.6242 | 0.2673 |

## Summary

| config | mean_auc | std_auc | mean_delta_v2 |
|---|---|---|---|
| mean_z | 0.6242 | 0.0000 | 0.2673 |

## Per-category AUC

| category | mean_auc |
|---|---|
| async_blocking | 0.6667 |
| background_tasks | 0.3750 |
| dependency_injection | 0.5000 |
| downstream_http | 0.8333 |
| exception_handling | 0.9167 |
| framework_swap | 1.0000 |
| routing | 0.6667 |
| serialization | 0.5000 |
| validation | 0.2500 |
