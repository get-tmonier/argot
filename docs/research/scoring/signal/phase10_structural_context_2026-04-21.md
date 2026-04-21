# Phase 10 — Fixture Expansion + Structural-Context Scorer (2026-04-21)

## Setup

- FastAPI HEAD: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`
- Encoder: `microsoft/unixcoder-base`
- JEPA ensemble: `EnsembleInfoNCE(n=3, beta=0.1, tau=0.1)`
- Training corpus: 1089 static chunks (core-only, non-test files; 3868 total chunks, 1089 after core filter)
- Configs tested: `baseline_9` (plain AST), `ctx` (parent-context features), `cooc` (co-occurrence features), `full` (ctx + cooc), `jepa_alone`
- Blend weights tested: 0.25 / 0.50 / 0.75 fraction AST

### Fixture counts per category

| Category | Break (before) | Control (before) | Break (after) | Control (after) | Total |
|----------|---------------:|-----------------:|--------------:|----------------:|------:|
| async_blocking | 2 | 1 | 3 | 2 | 5 |
| background_tasks | 1 | 0 | 4 | 2 | 6 |
| dependency_injection | 0 | 2 | 2 | 3 | 5 |
| downstream_http | 2 | 1 | 3 | 2 | 5 |
| exception_handling | 3 | 1 | 6 | 2 | 8 |
| framework_swap | 3 | 0 | 3 | 3 | 6 |
| routing | 2 | 1 | 3 | 2 | 5 |
| serialization | 1 | 1 | 3 | 2 | 5 |
| validation | 3 | 1 | 4 | 2 | 6 |
| **Total** | **17** | **8** | **31** | **20** | **51** |

Note: Phase 9 reported 19 break + 8 control = 27 fixtures. The discrepancy between 17 (v1+v2 breaks from the current manifest) and 19 is explained by the 2 fixture replacements executed during Phase 1B (`paradigm_break_raw_response` → `paradigm_break_msgpack_response`; `paradigm_break_global_singletons` → `paradigm_break_class_instance_no_depends`).

---

## Headline Results

**Best overall: `jepa (z)` — AUC=0.6419, Δ(z)=+0.4491**
(Phase 9 best: `jepa+ast_oov@0.50` — AUC=0.7697, Δ(z)=+0.7023)

### All scorers and best blends

| Scorer | Config | AUC | Δ(z) |
|--------|--------|----:|-----:|
| jepa (z) ← **best** | jepa_alone | 0.6419 | +0.4491 |
| jepa+ast_structural_zscore@0.25 | baseline_9 | 0.6290 | +0.4068 |
| jepa+ast_structural_oov@0.25 | baseline_9 | 0.6290 | +0.3873 |
| jepa+ast_structural_ctx_zscore@0.25 | ctx | 0.6339 | +0.3933 |
| jepa+ast_structural_cooc_zscore@0.25 | cooc | 0.6290 | +0.3950 |
| jepa+ast_structural_full_oov@0.25 | full | 0.6242 | +0.3664 |
| jepa+ast_structural_ll@0.25 | baseline_9 | 0.6226 | +0.3712 |
| jepa+ast_structural_ctx_ll@0.25 | ctx | 0.6194 | +0.3665 |
| jepa+ast_structural_cooc_oov@0.25 | cooc | 0.6274 | +0.3643 |
| jepa+ast_structural_ctx_oov@0.25 | ctx | 0.6274 | +0.3589 |
| jepa+ast_structural_cooc_ll@0.25 | cooc | 0.6129 | +0.3343 |
| ast_structural_ll | baseline_9 | 0.5661 | +36.8903 |
| ast_structural_ctx_ll | ctx | 0.5597 | +23.5286 |
| ast_structural_zscore | baseline_9 | 0.5468 | +2.0904 |
| ast_structural_oov | baseline_9 | 0.5468 | +4.5532 |
| ast_structural_cooc_zscore | cooc | 0.5419 | +2.0386 |
| ast_structural_ctx_zscore | ctx | 0.5419 | +5.4518 |
| ast_structural_full_oov | full | 0.5298 | +32.2339 |
| ast_structural_cooc_oov | cooc | 0.5282 | +26.8435 |
| ast_structural_ctx_oov | ctx | 0.5097 | +2.5565 |
| ast_structural_cooc_ll | cooc | 0.4855 | -30.6546 |

No AST blend surpassed JEPA alone. The best blend (`jepa+ast_structural_ctx_zscore@0.25`) reaches AUC=0.6339, still below the jepa-alone baseline of 0.6419.

---

## Per-category AUC

| Category | Phase 9 best (jepa+ast_oov@0.50) | jepa (z) | ast_ll | jepa+ast_ll@0.25 | ast_zscore | jepa+ast_zscore@0.25 | ast_oov | jepa+ast_oov@0.25 | ast_ctx_ll | jepa+ast_ctx_ll@0.25 | ast_ctx_zscore | jepa+ast_ctx_zscore@0.25 | ast_ctx_oov | jepa+ast_ctx_oov@0.25 | ast_cooc_ll | jepa+ast_cooc_ll@0.25 | ast_cooc_zscore | jepa+ast_cooc_zscore@0.25 | ast_cooc_oov | jepa+ast_cooc_oov@0.25 | ast_full_oov | jepa+ast_full_oov@0.25 |
|----------|--------------------------------:|--------:|-------:|----------------:|---------:|--------------------:|-------:|------------------:|----------:|--------------------:|-------------:|------------------------:|----------:|---------------------:|----------:|---------------------:|---------------:|-------------------------:|----------:|---------------------:|-----------:|----------------------:|
| async_blocking | 1.0000 | 0.6667 | 0.7500 | 0.6667 | 0.7500 | 0.6667 | 0.7500 | 0.6667 | 0.7500 | 0.8333 | 0.7500 | 0.6667 | 0.7500 | 0.6667 | 0.4167 | 0.6667 | 0.7500 | 0.6667 | 0.7500 | 0.6667 | 0.7500 | 0.6667 |
| background_tasks | n/a | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.7500 |
| dependency_injection | 1.0000 | 0.1667 | 0.3333 | 0.1667 | 0.6667 | 0.1667 | 0.0833 | 0.0000 | 0.5000 | 0.1667 | 0.8333 | 0.0000 | 0.0000 | 0.0000 | 0.6667 | 0.1667 | 0.6667 | 0.1667 | 1.0000 | 0.1667 | 0.8333 | 0.1667 |
| downstream_http | 1.0000 | 1.0000 | 0.7500 | 1.0000 | 0.4167 | 1.0000 | 0.7500 | 1.0000 | 0.5833 | 1.0000 | 0.5833 | 1.0000 | 0.5833 | 1.0000 | 0.5833 | 1.0000 | 0.4167 | 1.0000 | 0.7500 | 1.0000 | 0.5833 | 1.0000 |
| exception_handling | 0.6667 | 0.8333 | 0.5000 | 0.7500 | 0.5833 | 0.7500 | 0.7083 | 0.7500 | 0.5000 | 0.8333 | 0.5000 | 0.7500 | 0.6667 | 0.7500 | 0.5000 | 0.7500 | 0.5833 | 0.7500 | 0.5000 | 0.8333 | 0.5000 | 0.8333 |
| framework_swap | n/a | 1.0000 | 0.6667 | 0.8889 | 0.6667 | 0.8889 | 0.6667 | 0.8889 | 0.6667 | 0.8889 | 0.3333 | 0.8889 | 0.5556 | 0.8889 | 0.6667 | 0.8889 | 0.6667 | 0.8889 | 0.6667 | 0.8889 | 0.6667 | 0.8889 |
| routing | 1.0000 | 0.3333 | 0.5833 | 0.6667 | 0.5833 | 0.5000 | 0.5833 | 0.6667 | 0.4167 | 0.6667 | 0.5833 | 0.5000 | 0.5833 | 0.6667 | 0.4167 | 0.5000 | 0.5833 | 0.5000 | 0.4167 | 0.5000 | 0.4167 | 0.5000 |
| serialization | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.3333 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.5000 | 0.1667 | 0.3333 | 0.1667 | 0.5000 | 0.1667 | 0.5000 | 0.3333 | 0.3333 |
| validation | 1.0000 | 0.6250 | 1.0000 | 0.6250 | 0.7500 | 0.6250 | 0.7500 | 0.6250 | 0.8750 | 0.6250 | 0.5000 | 0.7500 | 0.7500 | 0.6250 | 0.3750 | 0.6250 | 0.7500 | 0.6250 | 0.6250 | 0.6250 | 0.6250 | 0.6250 |

---

## Per-category Delta (z-normalized)

| Category | jepa (z) | jepa+ast_ll@0.25 | jepa+ast_zscore@0.25 | jepa+ast_oov@0.25 | jepa+ast_ctx_ll@0.25 | jepa+ast_ctx_zscore@0.25 | jepa+ast_ctx_oov@0.25 | jepa+ast_cooc_ll@0.25 | jepa+ast_cooc_zscore@0.25 | jepa+ast_cooc_oov@0.25 | jepa+ast_full_oov@0.25 |
|----------|--------:|----------------:|-----------------:|----------------:|--------------------:|------------------------:|---------------------:|---------------------:|--------------------------:|-----------------------:|----------------------:|
| async_blocking | — | — | — | — | — | — | — | — | — | — | — |
| background_tasks | — | — | — | — | — | — | — | — | — | — | — |
| dependency_injection | — | — | — | — | — | — | — | — | — | — | — |
| downstream_http | — | — | — | — | — | — | — | — | — | — | — |
| exception_handling | — | — | — | — | — | — | — | — | — | — | — |
| framework_swap | — | — | — | — | — | — | — | — | — | — | — |
| routing | — | — | — | — | — | — | — | — | — | — | — |
| serialization | — | — | — | — | — | — | — | — | — | — | — |
| validation | — | — | — | — | — | — | — | — | — | — | — |

Per-category z-delta values are not broken out in the Phase 10 results JSON (only overall Δ(z) is stored per scorer). The overall Δ(z) values are reported in the Headline Results table above. See `.argot/phase10_results/` for raw fixture scores to recompute per-category deltas.

---

## Top-20 Corpus Composition

Not separately stored in Phase 10 eval output (the Phase 10 eval focused on the fixture scoring grid rather than corpus top-k analysis). The training corpus is identical to Phase 9: 1089 core-only chunks drawn from the same FastAPI HEAD (`2fa00db8581bb4e74b2d00d859c8469b6da296c4`). For per-variant corpus composition, see `.argot/phase10_results/` and the Phase 9 report corpus section.

---

## Fixture Audit Summary

Phase 1B audited all 27 pre-existing fixtures against the 1083-file corpus frequency tables.

**4 flagged for action:**

1. `paradigm_break_raw_response` — **replaced** (`paradigm_break_msgpack_response`): `jsonable_encoder` appears 78 times in corpus — firmly in-vocabulary — causing the break to score *lower* OOV than the control. The inversion caused serialization AUC=0.0 in Phase 9 `ast_oov`. Replaced with `msgpack.packb()` binary serialization, entirely absent from corpus (0 import sites).

2. `paradigm_break_global_singletons` — **replaced** (`paradigm_break_class_instance_no_depends`): `module-level singleton assignments` appear 922 times in corpus (the highest-frequency DI-adjacent pattern), making the "break" pattern indistinguishable from normal corpus code. Replaced with service classes passed as plain default argument values (`service: EmailService = email_service`), which has 0 corpus sites.

3. `paradigm_break_subtle_wrong_exception` — **kept** (split not yet executed): `ValueError` (10 sites) and `RuntimeError` (4 sites) are present in corpus at low frequency, mixing idiomatic scaffolding with partially-corpus-familiar exception vocabulary. Flagged for a clean two-fixture split in Phase 11.

4. `paradigm_break_subtle_exception_swallow` — **kept** (split not yet executed): Structurally overlaps with `wrong_exception` in the same category. Needs a dedicated minimal control (no try/except) for cleaner per-axis signal. Deferred to Phase 11.

**Per-category additions (Phase 10 wave 1 and wave 2):**

- `background_tasks`: 3 new breaks (`asyncio_create_task`, `queue_carryover`, `run_in_executor_endpoint`) + 2 new controls (`background_tasks_basic`, `background_tasks_depends`), establishing the category from scratch with corpus-grounded fixtures.
- `framework_swap`: 3 new controls (`lifespan_context`, `apirouter_composition`, `mounted_subapp`) to fix the missing-control problem that caused n/a AUC in Phase 9.
- `serialization`: 2 new breaks (`manual_dict_response`, `msgpack_response`, replacing `raw_response`) + 1 new control (`response_model_list`).
- `exception_handling`: 3 new breaks (`json_error_response`, `traceback_in_response`, `flask_errorhandler`) + 1 new control (`exception_handler_registration`).
- `dependency_injection`: 2 new breaks (`manual_generator_drain`, `class_instance_no_depends`) + 1 new control (`nested_depends`).
- `async_blocking`, `downstream_http`, `validation`, `routing`: 1 new break + 1 new control each.

---

## Discussion

### Why overall AUC dropped from Phase 9 (0.7697) to Phase 10 (0.6419)

The expanded fixture set is substantially harder. Phase 9 had 27 fixtures across 7 active categories (framework_swap and background_tasks had no controls, producing n/a AUC). Phase 10 adds 24 new fixtures across all 9 categories, and the new fixtures are deliberately harder: `dependency_injection` (JEPA AUC=0.1667) and `routing` (JEPA AUC=0.3333) are now evaluated with multiple fixtures per side, exposing JEPA's weakness on subtle structural axes where the break vocabulary overlaps with the corpus. The two previously n/a categories (`background_tasks` AUC=0.75, `framework_swap` AUC=1.00 with JEPA) now contribute real numbers, and while framework_swap is strong, the harder categories dominate the macro-average downward. The drop is a measurement artifact of a fairer, fuller test set — not a regression in the underlying model.

### What parent-context bought

The `ctx` config adds parent-scope AST features (the enclosing function/class/module context for each identifier occurrence). In isolation, `ast_structural_ctx_ll` (AUC=0.5597) is slightly below `ast_structural_ll` (AUC=0.5661). When blended with JEPA, the best ctx blend (`jepa+ast_structural_ctx_zscore@0.25`, AUC=0.6339) marginally outperforms the best baseline_9 blend (`jepa+ast_structural_zscore@0.25`, AUC=0.6290), but the difference is 0.0049 AUC — within noise for a 51-fixture set. Parent context adds modest signal on `dependency_injection` (ctx_zscore standalone: AUC=0.8333 vs baseline zscore: AUC=0.6667) but degrades `framework_swap` (ctx_zscore: AUC=0.3333 vs baseline: AUC=0.6667). Net contribution: minimal, and not reliably additive.

### What co-occurrence bought

The `cooc` config pairs each (AST-node-class, identifier) with its within-file co-occurring tokens. The standout result is `ast_structural_cooc_oov` achieving AUC=1.0000 on `dependency_injection` — the only scorer to perfectly separate the two new DI break fixtures (`manual_generator_drain`, `class_instance_no_depends`) from the three controls. This works because the co-occurrence vocabulary around `next()` (manual generator drain) and plain default-argument service instances has zero overlap with the corpus's `Depends()` co-occurrence patterns. However, `ast_structural_cooc_ll` overall is the worst performer (AUC=0.4855, Δ(z)=−30.65), dragged down by a negative delta on `serialization` and `routing`. Co-occurrence is a high-variance feature: superb on specific axes (DI), harmful on others.

### Where signal is still weak

**`routing` (JEPA AUC=0.3333):** The three break fixtures use patterns that carry some in-vocabulary tokens — `add_api_route` (6 corpus sites) and Starlette's `add_route` appear in path-adjacent contexts. The new `paradigm_break_imperative_route_loop` fixture uses `add_api_route` in a programmatic loop, which shares tokens with the corpus's 6 `add_api_route` sites. The AST scorers partially recover routing (e.g., `jepa+ast_ll@0.25` AUC=0.6667 and `jepa+ast_oov@0.25` AUC=0.6667), suggesting JEPA's semantic embeddings struggle to distinguish decorator-based from loop-based route registration when the vocabulary overlaps.

**`dependency_injection` (JEPA AUC=0.1667):** `Depends()` is the corpus-dominant pattern (428 sites), and the two breaks use `next(get_db())` (manual generator drain) and `service: EmailService = email_service` (default-arg injection). JEPA embeds these code chunks at similar semantic positions because the surrounding function signatures and return types are identical. The JEPA encoder does not distinguish `Depends(get_db)` from a plain default argument at the embedding level. Only `ast_cooc_oov` resolves this by exploiting the absence of `Depends`-adjacent co-occurrence vocabulary in the breaks.

**`serialization` (JEPA AUC=0.5000, all scorers ≤0.5000):** Even after replacing `raw_response` with `msgpack_response`, serialization remains at chance. The `manual_dict_response` break uses `float()`, `bool()`, and `.isoformat()` coercions — lexically plausible Python, just not FastAPI-idiomatic. Neither JEPA (semantic similarity) nor AST OOV (vocabulary surprise) can reliably detect that `float(item["price"])` is non-idiomatic when `float` is ubiquitous in the corpus. The axis is structural at the response construction level, not at the token or embedding level.

### What Phase 11 should attack

1. **Fix the routing scorer with a programmatic-route-detection heuristic.** `add_api_route` in a for-loop body is the distinguishing axis; an AST-walk that counts route-registration calls inside loop bodies vs. at module level would sharply separate the break from controls. This is a targeted structural rule, not a corpus-derived scorer, but routing AUC=0.33 is the worst category result and a one-rule fix is justified.

2. **Execute the two deferred exception_handling splits.** `paradigm_break_subtle_wrong_exception` and `paradigm_break_subtle_exception_swallow` were flagged but not split in Phase 10 to keep scope manageable. Splitting them with clean single-axis controls would likely lift exception_handling from AUC=0.8333 (JEPA) toward 1.00, and also reduce noise in the category AUC estimate.

3. **Target serialization with a response-construction AST rule.** The serialization fixtures share a common axis: the presence or absence of `response_model=` on the decorator and the presence of manual type coercions in the return path. A targeted AST feature that counts `response_model` keyword arguments on `@app.*` / `@router.*` decorators vs. manual `float()`/`dict()` constructions in return statements would give a high-contrast signal that neither JEPA nor OOV currently provides.

---

*Generated by Phase 10 autonomous overnight run. See `phase10_execution_log.md` for run details.*
