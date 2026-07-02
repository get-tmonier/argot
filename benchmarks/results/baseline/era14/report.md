# argot-bench report

| Corpus | Recall | FP rate | AUC | Threshold | CV | rare | Uncaught |
|:---|---:|---:|---:|---:|---:|---:|:---|
| fastapi | 30/32 (93.8%) | 0.53% (164/30758) | 0.995 | 5.2585 | 10.01% | 0 | validation_2, exception_handling_4 |
| rich | 16/16 (100.0%) | 1.23% (638/52070) | 0.996 | 3.8424 | 8.92% | 0 | — |
| faker | 15/16 (93.8%) | 1.92% (649/33883) | 0.954 | 5.0663 | 3.50% | 0 | synthetic_formula_1 |
| hono | 15/17 (88.2%) | 0.51% (163/31775) | 0.833 | 4.2707 | 2.52% | 0 | hono_validation_2, hono_middleware_3 |
| ink | 16/17 (94.1%) | 0.39% (28/7204) | 0.991 | 4.9932 | 6.01% | 0 | ink_dom_access_2 |
| faker-js | 16/17 (94.1%) | 1.70% (761/44680) | 0.948 | 4.8607 | 3.20% | 2 | faker_js_error_flip_2 |
| saleor | 12/14 (85.7%) | 0.24% (31/12777) | 0.993 | 5.4387 | 14.18% | 0 | raw_sql_2, print_debug_1 |
| wagtail | 14/14 (100.0%) | 0.34% (41/12155) | 0.999 | 4.6714 | 8.36% | 0 | — |
| excalidraw | 9/14 (64.3%) | 0.43% (84/19408) | 0.957 | 5.7597 | 13.40% | 0 | excalidraw_legacy_lifecycle_1, excalidraw_legacy_lifecycle_2, excalidraw_redux_store_2, excalidraw_callback_pyramid_2, excalidraw_vue_idioms_2 |
| outline | 10/14 (71.4%) | 0.46% (113/24482) | 0.879 | 5.0035 | 11.21% | 0 | outline_jquery_1, outline_foreign_http_2, outline_class_components_1, outline_class_components_2 |

**Total recall: 153/171 (89.5%)**
