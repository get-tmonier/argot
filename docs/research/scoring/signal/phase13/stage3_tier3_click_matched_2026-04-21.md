# Phase 13 Stage 3 — Tier 3 Methodology-Controlled Rerun (click)

Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`

model_A: 13 click source files (click/*.py minus 3 held-out files: decorators.py, types.py, core.py)

Controls: 10 × ~20-line hunks drawn from the 3 held-out files (size-matched to break hunks).

**Overall AUC: 0.2500**

## Per-fixture scores

| fixture | category | is_break | score |
|---|---|---|---|
| control_decorators_pass_context | control_decorators | False | 5.8126 |
| control_decorators_command | control_decorators | False | 8.0561 |
| control_decorators_option | control_decorators | False | 8.0561 |
| control_types_datetime | control_types | False | 7.1494 |
| control_types_choice_convert | control_types | False | 7.1494 |
| control_types_path_convert | control_types | False | 7.1494 |
| control_types_paramtype_convert | control_types | False | 7.1494 |
| control_core_context_init_tail | control_core | False | 7.1494 |
| control_core_command_parse_args | control_core | False | 7.1494 |
| control_core_group_add_command | control_core | False | 7.1494 |
| break_argparse_class_based_1 | argparse_class | True | 5.1160 |
| break_argparse_class_based_2 | argparse_class | True | 5.1160 |
| break_argparse_class_based_3 | argparse_class | True | 6.0835 |
| break_optparse_deprecated_1 | optparse | True | 7.1610 |
| break_optparse_deprecated_2 | optparse | True | 7.1610 |
| break_raw_sys_argv_1 | raw_argv | True | 6.3423 |
| break_raw_sys_argv_2 | raw_argv | True | 6.3423 |
| break_docopt_style_1 | docopt | True | 5.8977 |

## Verdict

**FAIL** (AUC 0.2500 < 0.65). Method does not generalise to click even with corpus and hunk sizes matched; prior-run abandonment recommendation stands.

## Comparison to v1

Prior run (v1, 6-file model_A, whole-file controls): **AUC 0.1187**

This run (v2, 13-file model_A, size-matched controls): **AUC 0.2500** (Δ = +0.1313)

No meaningful recovery. Methodology fixes do not rescue the scorer — v1's 'FastAPI-tuned' verdict is supported by this rerun.

## Analysis

**Hypothesis supported:** "Genuinely FastAPI-tuned." AUC = 0.2500, delta = +0.1313. Both conditions for "genuinely FastAPI-tuned" are met: AUC < 0.35 and delta < 0.30. The v1 abandonment recommendation stands.

**Per-category breakdown:** The best-performing break category is `optparse` (7.1610), which barely edges the median control score of 7.1494 — essentially indistinguishable. `raw_argv` comes second (6.3423), `docopt` third (5.8977), and `argparse_class` is lowest (5.1160–6.0835). No category produces scores clearly above controls; in fact, most controls outscore most breaks. The ordering does not replicate FastAPI's break categories (which favoured `raw_argv` and `argparse_class`). The scorer's inability to rank any break category above the bulk of the controls is the core diagnostic: the AST treelet frequency ratios learned from click's own source are essentially flat across all break types, and the controls drawn from held-out click code score just as high as (or higher than) the breaks.

**Control outlier check:** `control_decorators_command` and `control_decorators_option` score 8.0561, 0.9067 above the median control of 7.1494 — the most notable spread in the control set. Inspection of those hunks (decorators.py lines 146–167 and 329–351) reveals no `argparse`, `optparse`, or `sys.argv` content. The elevated scores are structural: the `@t.overload` decorator stubs and `@argument`/`@option` docstring bodies contain AST treelets that are genuinely rare in the 13-file model_A because their source file (decorators.py) is held out. This is a mild corpus-gap artefact, not a fixture-design leak, and it actually works against the scorer (controls score higher, not breaks).

**Next action:** Abandon contrastive approach; move to CodeBERT MLM surprise baseline. Two independent runs on different corpora (FastAPI and click) now confirm that `ContrastiveAstTreeletScorer` does not generalise across CLI frameworks. The AUC on click remains below 0.35 even after removing both structural confounders from v1. Methodology was not the primary cause of failure.
