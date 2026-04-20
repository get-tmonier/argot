# fastapi stage 6 sweep

## Raw

| config | seed | delta_v2 | delta_v1 | gate |
|---|---|---|---|---|
| b01_t01_w0 | 42 | 0.1087 | 0.2291 | ✗ |
| b01_t01_w0 | 43 | 0.1087 | 0.2291 | ✗ |
| b01_t01_w0 | 44 | 0.1087 | 0.2291 | ✗ |
| b01_t01_w0 | 45 | 0.1087 | 0.2291 | ✗ |
| b01_t01_w0 | 46 | 0.1087 | 0.2291 | ✗ |

## Summary

| config | mean_delta_v1 | mean_delta_v2 | std_delta | gate |
|---|---|---|---|---|
| b01_t01_w0 | 0.2291 | 0.1087 | 0.0000 | ✗ |

## Per-category deltas (v2)

> Categories with no controls share the global v2 control mean.

| category | mean_delta |
|---|---|
| async_blocking | 0.1007 |
| background_tasks | 0.0511 |
| dependency_injection | 0.1923 |
| downstream_http | 0.1185 |
| exception_handling | 0.2541 |
| framework_swap | 0.1304 |
| routing | 0.2411 |
| serialization | 0.2232 |
| validation | -0.1512 |
