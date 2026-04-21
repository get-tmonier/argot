# Phase 13 Stage 1: Contrastive AST Treelet Scorer — 2026-04-21

## Setup

- **Entry:** `fastapi` (51 fixtures: 31 breaks, 20 controls)
- **Context mode:** `file_only`
- **Scorer:** `ContrastiveAstTreeletScorer` — log-ratio anomaly score over depth-1/2 AST treelets
- **Formula:** `mean_t[ log(model_B[t] + ε) - log(model_A[t] + ε) ]`
  - model_A = treelet counts from repo corpus (`corpus_file_only.jsonl`, 2000 records, ~14% parseable)
  - model_B = treelet counts from CPython stdlib (`generic_treelets.json`, 5 803 types, 3.2M treelets)
  - Higher score = hunk treelets are less like the repo and more like generic Python = more anomalous
- **ε variants:** 0.1 (`ast_contrastive_e01`), 1.0 (`ast_contrastive`), 10.0 (`ast_contrastive_e10`)
- **Baselines:** tfidf_anomaly = 0.6968 (phase 12 winner), JEPA = 0.6532 (phase 11 winner)

**Bug found and fixed during this run:** The scorer originally computed
`log(model_A) - log(model_B)` (idiomaticity score) and the fixture records
lacked valid source reconstruction.  Two fixes were applied:

1. `runner.py` `fixture_to_record` now includes `hunk_source` (original lines),
   `hunk_start_line`, `hunk_end_line`, and `node_type`/`start_line` per token.
2. `ast_contrastive.py` uses `hunk_source` when present (fixtures), falls back to
   `start_line`-grouped token reconstruction (corpus), and flips the log-ratio
   sign to produce an **anomaly** score consistent with the bakeoff AUC convention
   (higher score = more anomalous = more break-like).

## AUC Results

| variant | overall_AUC | beats_tfidf (0.6968)? |
|---|---|---|
| ast_contrastive (ε=1.0) | 0.5871 | No |
| ast_contrastive_e01 (ε=0.1) | 0.5710 | No |
| **ast_contrastive_e10 (ε=10.0)** | **0.6032** | No |
| tfidf_anomaly (reference) | 0.6968 | — |

Best variant: `ast_contrastive_e10` (ε=10.0).  Larger ε smooths away the
sparsity in model_A, which only has ~360 treelet types vs 5 803 in model_B.

## Per-Category AUC

| category | tfidf_anomaly | ast_contrastive_e10 |
|---|---|---|
| async_blocking | 0.8333 | 0.7500 |
| background_tasks | 0.3750 | **0.7500** |
| dependency_injection | 0.6667 | 0.5000 |
| downstream_http | 1.0000 | 0.7500 |
| exception_handling | 0.5000 | 0.7500 |
| framework_swap | 0.6667 | 0.6667 |
| routing | 0.6667 | 0.5833 |
| serialization | 1.0000 | 0.8333 |
| validation | 0.7500 | 0.5000 |

Notable: `ast_contrastive_e10` beats tfidf on `background_tasks` (0.75 vs 0.375),
`exception_handling` (0.75 vs 0.50), and matches on `framework_swap`.  It is
weaker on `serialization`, `downstream_http`, `validation`, and `routing`.

## Victory Gate Assessment

| condition | result | value |
|---|---|---|
| 1. AUC ≥ 0.75 | ✗ | 0.6032 (gap: 0.1468) |
| 2. background_tasks AUC ≥ 0.60 | ✓ | 0.7500 |

**Victory gate NOT cleared.**  Gate 1 fails by a large margin.

## Missed Fixtures Analysis

15 out of 31 break fixtures are misclassified (break score ≤ max control score
in the same category).  Nine are unparseable (score = 0.0); six have parseable
treelets but score below their category peers.

### Unparseable fixtures (score = 0.0)

These break fixtures begin mid-block (e.g. inside a dict literal, mid-function
body) so `ast.parse()` raises `SyntaxError` and the scorer falls back to 0.0.
The `hunk_start_line` in the manifest points into a partial expression context.

| fixture | category |
|---|---|
| paradigm_break_tornado_handler | framework_swap |
| paradigm_break_voluptuous_validation | validation |
| paradigm_break_sync_file_io_async | async_blocking |
| paradigm_break_multiprocessing_background | background_tasks |
| paradigm_break_queue_carryover | background_tasks |
| paradigm_break_aiohttp_no_context | downstream_http |
| paradigm_break_json_error_response | exception_handling |
| paradigm_break_traceback_in_response | exception_handling |
| paradigm_break_imperative_route_loop | routing |

### Parseable but wrong-scored fixtures

For each, the top-3 treelets driving the score are shown.  All share the same
dominant treelet `d1:arguments>arg` which has model_A count = 1 vs model_B
count = 42 999 — this is a corpus sparsity artefact, not a genuine signal.

**paradigm_break_starlette_mount** (score=4.47, max_ctrl_in_cat=4.58)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999) — sparsity artefact
- `d2:Expr>Call>Name` logr=7.28 (a=1, b=15893) — sparsity artefact
- `d1:Compare>Name` logr=7.23 (a=1, b=15170) — sparsity artefact

**paradigm_break_cerberus_validation** (score=4.33, max_ctrl_in_cat=4.45)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999)
- `d2:Assign>Dict>Constant` logr=7.60 (a=10, b=40028)
- `d2:Expr>Call>Name` logr=7.28 (a=1, b=15893)

**paradigm_break_manual_generator_drain** (score=4.43, max_ctrl_in_cat=4.74)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999)
- `d2:Expr>Call>Name` logr=7.28 (a=1, b=15893)
- `d1:Compare>Name` logr=7.23 (a=1, b=15170)

**paradigm_break_class_instance_no_depends** (score=4.52, max_ctrl_in_cat=4.74)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999)
- `d2:Expr>Call>Name` logr=7.28 (a=1, b=15893)
- `d1:Compare>Name` logr=7.23 (a=1, b=15170)

**paradigm_break_manual_dict_response** (score=4.24, max_ctrl_in_cat=4.32)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999)
- `d1:Compare>Name` logr=7.23 (a=1, b=15170)
- `d2:Compare>Name>Load` logr=7.23 (a=1, b=15170)

**paradigm_break_assert_validation** (score=4.24, max_ctrl_in_cat=4.45)
- `d1:arguments>arg` logr=8.27 (a=1, b=42999)
- `d1:If>Assign` logr=7.44 (a=0, b=17051)
- `d1:Compare>Name` logr=7.23 (a=1, b=15170)

**Diagnosis:** The top-3 treelets in every parseable miss are dominated by
sparsity artefacts — very common in generic Python (model_B counts in the
10 000–43 000 range) but almost absent in model_A (counts 0–10).  This means
`model_A` is too sparse to distinguish breaks from controls; the score is
essentially a monotone function of how many generic-Python treelets appear in
the hunk, not how repo-idiomatic they are.

## Root Cause: model_A Sparsity

The corpus contains 2 000 records, but only ~14% (≈280) yield parseable Python
after `ast.parse()`.  This gives model_A only 360 treelet types vs 5 803 in
model_B.  Most model_A counts are 0 or 1, so the log-ratio is dominated by
`log(model_B[t])` and the scorer degrades to a proxy for "how common is this
treelet in generic Python" — with no repo-specific contrast.

## Conclusion

**Victory gate NOT cleared** (overall AUC 0.6032, gate requires 0.75).

The `ast_contrastive_e10` variant beats the JEPA baseline (0.6532) on
`background_tasks` and `exception_handling` individually, but underperforms
on `serialization`, `downstream_http`, and `validation`.

**Primary failure mode:** model_A corpus sparsity (~14% parse rate on git hunks)
makes the contrast signal nearly flat across all fixtures.

**Recommended ablations for Stage 2:**

1. **Rebuild model_A from full file context** — instead of fitting on hunk
   tokens, fetch the full file for each corpus record and extract treelets from
   the entire file.  This would increase the parse rate from 14% to ~100%.

2. **Increase depth** — add depth-3 treelets to capture import-specific patterns
   (e.g. `d3:Module>ImportFrom>alias>identifier`).

3. **Scope-filtered model_A** — restrict corpus to the same directory/file scope
   as the fixture to get a more targeted contrast.

4. **ε sweep with rebuilt model_A** — once model_A is dense, re-sweep ε in
   {0.1, 1.0, 5.0, 10.0} to find the optimal smoothing.
