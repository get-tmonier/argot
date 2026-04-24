# Evidence — Benchmark Fairness (Era 7)

Era 7 run: `benchmarks/results/baseline/latest/` (20260424T113422Z).
5 seeds, full control sets (no sampling), call_receiver_alpha=0.0.

## Headline metrics

| Corpus | AUC | Recall | FP rate | N fixtures | N controls |
|:---|---:|---:|---:|---:|---:|
| fastapi | 0.9880 | 71.3% | 0.8% | 32 | 79,623 |
| rich | 0.9935 | 93.3% | 0.1% | 15 | 68,598 |
| faker | 0.9530 | 73.3% | 0.7% | 15 | 75,996 |
| hono | 0.8107 | 60.0% | 0.4% | 15 | 54,717 |
| ink | 0.9888 | 86.7% | 0.4% | 15 | 16,678 |
| faker-js | 0.9408 | 20.0% | 0.8% | 15 | 255,760 |

## Recall by difficulty band

| Corpus | easy | medium | hard | uncaught |
|:---|:---|:---|:---|:---|
| fastapi | 1/1 | 22/22 | 0/6 | 0/3 |
| rich | 8/8 | 5/5 | 1/2 | — |
| faker | 5/5 | 4/5 | 2/5 | — |
| hono | — | 9/9 | — | 0/6 |
| ink | — | 13/14 | 0/1 | — |
| faker-js | — | 3/15 | — | — |

easy and medium bands are caught reliably (100% easy, 80–100% medium on
five of six corpora). hard/uncaught fixtures are correct misses at
call_receiver_alpha=0.0; they are the target for Stage 1.5 development.

## Gate 1 parity check

Script: `benchmarks/scripts/verify_parity.py`.
Compared era-6 baseline (91 fixtures) vs era-7 run (107 fixtures, 91 shared):

```
Matching:           80 / 91
Config changes:     10 / 91  (call_receiver alpha 1.0→0.0, expected)
Threshold variance: 1 / 91   (ink_dom_access_2, CV=10.6%, BPE within ±15% of thr)
MISMATCHES:         0 / 91
```

Gate 1: PASS.

## Structural gates (era 7)

| Gate | Check | Result |
|:---|:---|:---|
| G-3a | Each corpus ≥15 fixtures | PASS (32/15/15/15/15/15) |
| G-3b | Each corpus ≥5 categories | PASS (9/5/5/5/5/5) |
| G-3c | Each category ≥3 fixtures | PASS (verified by test_era7_structural_requirements) |
| G-4 | Each corpus exactly 5 real PR entries | PASS |
| G-5 | All 107 fixtures have non-None difficulty | PASS |
| G-6 | recall_by_difficulty in every report | PASS |

## Per-corpus difficulty distribution

### fastapi (32 fixtures)
- easy: 1 (dependency_injection_3)
- medium: 22 (background_tasks ×4, framework_swap ×3, serialization ×3, ...)
- hard: 6 (routing ×3, exception_handling ×2, dependency_injection_2)
- uncaught: 3 (async_blocking_1, downstream_http_1, validation_2)

### rich (15 fixtures)
- easy: 8 (colorama ×2, curses ×2, termcolor ×2, print_manual_3, ansi_raw_3)
- medium: 5 (ansi_raw ×2, colorama_3, curses_3, termcolor_3)
- hard: 2 (print_manual_1, print_manual_2)

### faker (15 fixtures)
- easy: 5 (mimesis_alt_1, numpy_random_1, requests_source_1, sqlalchemy_sink_1, threading_provider_1)
- medium: 5 (mimesis_alt_2, numpy_random_2, requests_source_2, sqlalchemy_sink_2, threading_provider_2)
- hard: 5 (mimesis_alt_3, numpy_random_3, requests_source_3, sqlalchemy_sink_3, threading_provider_3)

### hono (15 fixtures)
- medium: 9 (async_blocking ×3, framework_swap ×2, middleware_1, routing_1, validation ×2)
- uncaught: 6 (framework_swap_1, middleware_2, middleware_3, routing_2, routing_3, validation_2)

### ink (15 fixtures)
- medium: 14 (class_components ×3, dom_access_3, error_flip ×3, jquery ×3, lifecycle ×3)
- hard: 1 (dom_access_1)

### faker-js (15 fixtures)
- medium: 15 (all)

## Calibration stability

| Corpus | Threshold CV | Jaccard |
|:---|---:|---:|
| fastapi | 0.4% | 0.069 |
| rich | 9.5% | 0.038 |
| faker | 3.7% | 0.067 |
| hono | 3.0% | 0.016 |
| ink | 6.9% | 0.094 |
| faker-js | 3.7% | 0.142 |

fastapi calibration is very stable (0.4% CV). rich, ink, and faker have
moderate variance — these are the corpora where borderline fixture verdicts
(within the noise band) may flip between runs. The ink threshold-variance
case in Gate 1 is consistent with its 6.9% CV.
