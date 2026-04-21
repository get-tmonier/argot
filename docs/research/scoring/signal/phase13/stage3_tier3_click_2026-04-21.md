# Phase 13 Stage 3 — Tier 3 Cross-Domain Validation (click)

Scorer: `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')`

model_A: 6 click source files (click/*.py minus 10 held-out controls)

**Overall AUC: 0.1187**

## Per-category scores

| fixture | category | is_break | score |
|---|---|---|---|
| control_click_decorators | control | False | 8.2346 |
| control_click_types | control | False | 8.8888 |
| control_click_core | control | False | 8.8888 |
| control_click_termui | control | False | 11.3507 |
| control_click_utils | control | False | 7.8741 |
| control_click_parser | control | False | 8.4249 |
| control_click_formatting | control | False | 7.9600 |
| control_click_exceptions | control | False | 8.8888 |
| control_click_globals | control | False | 5.9035 |
| control_click_shell_completion | control | False | 8.8888 |
| break_argparse_class_based_1 | argparse_class | True | 6.2359 |
| break_argparse_class_based_2 | argparse_class | True | 6.2359 |
| break_argparse_class_based_3 | argparse_class | True | 6.2359 |
| break_optparse_deprecated_1 | optparse | True | 7.1610 |
| break_optparse_deprecated_2 | optparse | True | 7.1610 |
| break_raw_sys_argv_1 | raw_argv | True | 7.8741 |
| break_raw_sys_argv_2 | raw_argv | True | 7.8741 |
| break_docopt_style_1 | docopt | True | 7.8741 |

## Verdict

**FAIL** (AUC 0.1187 < 0.65). Method is FastAPI-tuned; do not promote.

## Analysis

### 1. Top-scoring breaks — what drove the scores?

The three highest-scoring break categories are `raw_sys_argv` and `docopt_style` (both 7.8741) and `optparse` (7.1610).

- `raw_sys_argv` and `docopt_style` reach 7.8741 — the same score as `control_click_utils`. Their treelets (simple list-index accesses, string comparisons, bare loops over `sys.argv`) happen to appear in the generic-Python reference corpus at a similar rate to how they appear in the click source, producing a mid-range log-ratio. No structural click-specific constructs are displaced.
- `optparse` scores 7.1610, slightly below raw_argv. `optparse.OptionParser` and its methods share an object-oriented calling convention with click's own internals, so their treelets are not strongly anomalous relative to the click corpus.
- `argparse_class` is the lowest break category (6.2359), below most controls. Class-based argparse code uses `add_argument` / `parse_args` call patterns that happen to be rarer in the click source than in generic Python, which should push the score up — but `max` aggregation selects the single highest per-treelet ratio, and for argparse the maximum treelet is not distinctive enough to beat click-decorator-heavy controls.

### 2. Misclassification diagnosis

The ranking is almost completely inverted: 9 of 10 controls outscore the break median (7.16). This is not a fixture-design leak — the controls genuinely contain click-idiomatic code — but an architectural mismatch:

- **Model_A is too small and too generic.** Only 6 click source files were available after holding out the 10 control files. Click's own internals use many generic Python patterns (class hierarchies, decorators, string formatting), so the model_A treelet distribution closely resembles the generic reference corpus (model_B). The log-ratio `log(model_B[t] / tb) - log(model_A[t] / ta)` is therefore positive and large for most treelets regardless of whether the fixture is a control or a break.
- **`max` aggregation amplifies the noise.** Taking the maximum per-treelet ratio means a single common treelet shared between model_B and the fixture can dominate the score, overriding any genuine discriminative signal.
- The result: every fixture, control or break, receives a high "anomaly" score because the model cannot distinguish click-idiomatic code from anything else. Controls in fact score slightly higher because they contain more of the same generic treelets that appear frequently in the reference corpus.

### 3. Gate assessment

The pre-registered PASS gate is AUC ≥ 0.80. The pre-registered FAIL threshold is AUC < 0.65.

**The gate was missed. AUC = 0.1187 is far below the FAIL threshold of 0.65.** An AUC below 0.5 means the scorer actively ranks controls above breaks, i.e. it performs worse than random.

### 4. Recommendation

**Abandon `ast_contrastive` for cross-domain promotion.** The method is fragile to corpus size: it worked on FastAPI (large, idiomatic corpus) but collapses when model_A is small and generic. Do not proceed to Stage 4. Instead, design a successor scorer that either (a) requires no fitting (zero-training baseline such as CodeBERT MLM surprise, per the feedback on trying simple literature baselines first) or (b) derives discriminative signal from import-graph topology rather than raw treelet frequencies, which is domain-agnostic and robust to small reference corpora.
