# argot-bench report

| Corpus | Recall | FP rate | AUC | Threshold | CV | rare | Uncaught |
|:---|---:|---:|---:|---:|---:|---:|:---|
| fastapi | 32/32 (100.0%) | 1.66% (510/30685) | 0.995 | 5.2585 | 10.01% | 0 | — |
| rich | 16/16 (100.0%) | 0.93% (489/52307) | 0.996 | 3.8424 | 8.92% | 0 | — |
| faker | 14/16 (87.5%) | 2.13% (775/36403) | 0.957 | 5.0663 | 3.50% | 0 | mimesis_alt_3, synthetic_formula_1 |
| hono | 13/17 (76.5%) | 3.16% (1004/31745) | 0.833 | 4.2707 | 2.52% | 0 | hono_validation_2, hono_middleware_2, hono_middleware_3, hono_routing_2 |
| ink | 16/17 (94.1%) | 1.45% (104/7185) | 0.991 | 4.9932 | 6.01% | 0 | ink_dom_access_2 |
| faker-js | 14/17 (82.4%) | 6.94% (2814/40540) | 0.966 | 4.8607 | 3.20% | 2 | faker_js_foreign_rng_2, faker_js_error_flip_2, faker_js_error_flip_3 |
| saleor | 11/14 (78.6%) | 0.61% (77/12712) | 0.993 | 5.4387 | 14.18% | 0 | raw_sql_2, print_debug_1, sleep_polling_2 |
| wagtail | 14/14 (100.0%) | 1.42% (172/12077) | 0.999 | 4.6714 | 8.36% | 0 | — |
| excalidraw | 8/14 (57.1%) | 1.65% (319/19316) | 0.958 | 5.7597 | 13.40% | 0 | excalidraw_legacy_lifecycle_1, excalidraw_redux_store_2, excalidraw_callback_pyramid_1, excalidraw_callback_pyramid_2, excalidraw_vue_idioms_1, excalidraw_vue_idioms_2 |
| outline | 10/14 (71.4%) | 0.37% (89/24375) | 0.880 | 5.0035 | 11.21% | 0 | outline_redux_boilerplate_1, outline_express_idioms_2, outline_foreign_http_2, outline_class_components_1 |

**Total recall: 148/171 (86.5%)**
