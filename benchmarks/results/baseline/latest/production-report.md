# argot-bench production-path report

| Corpus | Recall (production) | Recall (catalog) | Gap | FP rate | FP hits/hunks (commits) | Uncaught |
|:---|---:|---:|---:|---:|---:|:---|
| fastapi | 32/32 (100.0%) | 32/32 (100.0%) | +0.0pp | 0.00% | 0/0 (30) | — |
| rich | 16/16 (100.0%) | 16/16 (100.0%) | +0.0pp | 0.00% | 0/30 (30) | — |
| faker | 16/16 (100.0%) | 14/16 (87.5%) | +12.5pp | 0.00% | 0/27 (30) | — |
| hono | 16/17 (94.1%) | 13/17 (76.5%) | +17.6pp | 0.00% | 0/53 (30) | hono_middleware_3 |
| ink | 17/17 (100.0%) | 16/17 (94.1%) | +5.9pp | 1.22% | 1/82 (30) | — |
| faker-js | 14/17 (82.4%) | 14/17 (82.4%) | +0.0pp | 0.00% | 0/172 (30) | faker_js_runtime_fetch_1, faker_js_error_flip_1, faker_js_error_flip_3 |
| saleor | 14/14 (100.0%) | 11/14 (78.6%) | +21.4pp | 2.07% | 3/145 (30) | — |
| wagtail | 14/14 (100.0%) | 14/14 (100.0%) | +0.0pp | 0.00% | 0/29 (30) | — |
| excalidraw | 9/14 (64.3%) | 8/14 (57.1%) | +7.1pp | 3.17% | 8/252 (30) | excalidraw_xhr_network_1, excalidraw_legacy_lifecycle_1, excalidraw_callback_pyramid_1, excalidraw_callback_pyramid_2, excalidraw_vue_idioms_2 |
| outline | 12/14 (85.7%) | 10/14 (71.4%) | +14.3pp | 0.48% | 1/209 (30) | outline_foreign_http_2, outline_class_components_1 |

**Total production recall: 160/171 (93.6%)**
