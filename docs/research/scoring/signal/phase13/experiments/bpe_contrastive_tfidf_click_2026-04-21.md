# Phase 13 — BPE Contrastive TF-IDF on Click (Tier 3 Matched, 2026-04-21)


## Summary


| scorer | corpus | tokenizer | AUC |
|---|---|---|---|
| contrastive_tfidf (word baseline) | click (v2 matched) | argot tokenize_lines | 0.7000 |
| **bpe_contrastive_tfidf** | **click (v2 matched)** | **UnixCoder BPE** | **0.6000** |

## Per-Category AUC


*(break category vs all 10 controls)*


| category | n_breaks | AUC |
|---|---|---|
| argparse_class | 3 | 0.4333 |
| docopt | 1 | 0.8000 |
| optparse | 2 | 0.8500 |
| raw_argv | 2 | 0.5000 |

## Fixture Scores


| fixture | category | is_break | score |
|---|---|---|---|
| control_decorators_pass_context | control_decorators | False | 7.8407 |
| control_decorators_command | control_decorators | False | 6.4772 |
| control_decorators_option | control_decorators | False | 6.4772 |
| control_types_datetime | control_types | False | 9.5915 |
| control_types_choice_convert | control_types | False | 6.7350 |
| control_types_path_convert | control_types | False | 8.4267 |
| control_types_paramtype_convert | control_types | False | 7.0783 |
| control_core_context_init_tail | control_core | False | 6.9354 |
| control_core_command_parse_args | control_core | False | 7.6106 |
| control_core_group_add_command | control_core | False | 6.4353 |
| break_argparse_class_based_1 | argparse_class | True | 7.7616 |
| break_argparse_class_based_2 | argparse_class | True | 7.2781 |
| break_argparse_class_based_3 | argparse_class | True | 6.3728 |
| break_optparse_deprecated_1 | optparse | True | 8.4267 |
| break_optparse_deprecated_2 | optparse | True | 8.4267 |
| break_raw_sys_argv_1 | raw_argv | True | 6.9267 |
| break_raw_sys_argv_2 | raw_argv | True | 7.2485 |
| break_docopt_style_1 | docopt | True | 8.1309 |

## Interpretation


AUC 0.6000 < 0.70: regression from word baseline (0.7000). BPE tokens made things worse — vocabulary holes are not the bottleneck. Recommend a context-aware approach (conditional distributions over context windows).

Max-token saturation resolved: 7/8 unique break scores (word baseline had 1/8 unique scores — all 8 identical at 8.4418).

