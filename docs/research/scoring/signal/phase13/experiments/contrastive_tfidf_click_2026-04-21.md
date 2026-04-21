# Phase 13 — Contrastive TF-IDF on Click (Tier 3 Matched, 2026-04-21)


## Summary


| scorer | corpus | AUC |
|---|---|---|
| ast_contrastive_max | click (v2 matched) | 0.2500 |
| contrastive_tfidf | FastAPI | 0.9847 |
| **contrastive_tfidf** | **click (v2 matched)** | **0.7000** |

## Per-Category AUC


*(break category vs all 10 controls)*


| category | n_breaks | AUC |
|---|---|---|
| argparse_class | 3 | 0.7000 |
| docopt | 1 | 0.7000 |
| optparse | 2 | 0.7000 |
| raw_argv | 2 | 0.7000 |

## Fixture Scores


| fixture | category | is_break | score |
|---|---|---|---|
| control_decorators_pass_context | control_decorators | False | 3.0498 |
| control_decorators_command | control_decorators | False | 7.1048 |
| control_decorators_option | control_decorators | False | 7.1048 |
| control_types_datetime | control_types | False | 7.1687 |
| control_types_choice_convert | control_types | False | 5.6789 |
| control_types_path_convert | control_types | False | 6.7282 |
| control_types_paramtype_convert | control_types | False | 9.0899 |
| control_core_context_init_tail | control_core | False | 9.0899 |
| control_core_command_parse_args | control_core | False | 8.7998 |
| control_core_group_add_command | control_core | False | 4.1951 |
| break_argparse_class_based_1 | argparse_class | True | 8.4418 |
| break_argparse_class_based_2 | argparse_class | True | 8.4418 |
| break_argparse_class_based_3 | argparse_class | True | 8.4418 |
| break_optparse_deprecated_1 | optparse | True | 8.4418 |
| break_optparse_deprecated_2 | optparse | True | 8.4418 |
| break_raw_sys_argv_1 | raw_argv | True | 8.4418 |
| break_raw_sys_argv_2 | raw_argv | True | 8.4418 |
| break_docopt_style_1 | docopt | True | 8.4418 |

## Interpretation


AUC 0.7000 falls between 0.55 and 0.80: partial generalisation. The contrast formulation carries real cross-repo signal but is not sufficient alone. The contrastive-MLM experiment is now justified as the next step — a pre-trained token distribution may close the gap.

