# fastapi stage 5 sweep — context_mode=combined

## Raw

| config | seed | auc | delta_v2 |
|---|---|---|---|
| mean_z | 0 | 0.6452 | 0.3783 |

## Summary

| config | mean_auc | std_auc | mean_delta_v2 |
|---|---|---|---|
| mean_z | 0.6452 | 0.0000 | 0.3783 |

## Per-category AUC

| category | mean_auc |
|---|---|
| async_blocking | 0.6667 |
| background_tasks | 0.0000 |
| dependency_injection | 0.3333 |
| downstream_http | 0.8333 |
| exception_handling | 1.0000 |
| framework_swap | 1.0000 |
| routing | 0.6667 |
| serialization | 0.6667 |
| validation | 0.6250 |
