# Phase 11 — AST Context-Variant Sweep: Full Results (2026-04-21)

## Setup

- **Scorer:** `EnsembleJepaScorer(n=3, aggregation=mean, zscore_vs_corpus=True, topk_k=64)` — Stage-5 winner (`mean_z`)
- **Encoder:** `microsoft/unixcoder-base` (frozen, Phase 7 decision)
- **Training corpus:** 2000 FastAPI commit records
- **Fixtures:** 51 total — 31 break, 20 control, 9 categories
- **Seeds:** `[0]` (single seed; n=3 ensemble reduces within-run variance)

### Context modes

| Mode | Description |
|---|---|
| `baseline` | 20 lines immediately preceding the hunk (original behaviour) |
| `parent_only` | Smallest enclosing AST scope (function/class → Module fallback), hunk lines spliced out |
| `file_only` | Full source file, hunk lines spliced out, 2000-char budget centred on hunk |
| `siblings_only` | Sibling nodes of the hunk's direct AST parent (fallback → parent_only) |
| `combined` | parent_only + `---` + file_only + `---` + siblings_only, 2000-char budget |

Corpus variants (`corpus_{mode}.jsonl`) were built with `build_variant_corpus.py` using `git show <sha>:<path>` on the local fastapi clone. Drop rate: 0/2000 for all modes.

---

## 1. Overall results

| Mode | AUC | Delta (break−ctrl) | Δ AUC vs baseline |
|---|---:|---:|---:|
| `baseline` | 0.4871 | −0.0012 | — |
| `siblings_only` | 0.6242 | +0.2673 | +0.1371 |
| `parent_only` | 0.6274 | +0.2531 | +0.1403 |
| `combined` | 0.6452 | +0.3783 | +0.1581 |
| **`file_only`** | **0.6532** | **+0.3177** | **+0.1661** |

**Key finding:** the 20-line lexical window is below chance (AUC < 0.5). Every AST variant flips it into positive signal. `file_only` wins on AUC; `combined` wins on raw delta.

### Filtered results (excluding 3 structurally-inverted categories)

| Mode | AUC | Delta | n |
|---|---:|---:|---|
| `baseline` | 0.6154 | +0.3171 | 21b / 13c |
| **`file_only`** | **0.8095** | **+0.6469** | 21b / 13c |

On the 6 categories where FastAPI idioms are visible in file context, `file_only` reaches 0.81 AUC.

---

## 2. Per-category AUC × mode

| Category | baseline | parent_only | file_only | siblings_only | combined | Best |
|---|---:|---:|---:|---:|---:|---|
| framework_swap | 1.0000 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | all tied |
| exception_handling | 0.5833 | 0.9167 | 0.9167 | 0.9167 | **1.0000** | combined |
| async_blocking | 0.8333 | 0.8333 | 0.8333 | 0.6667 | 0.6667 | baseline / file_only / parent_only |
| downstream_http | 0.6667 | 0.8333 | 0.8333 | 0.8333 | 0.8333 | all AST variants |
| routing | 0.3333 | 0.6667 | 0.6667 | 0.6667 | 0.6667 | all AST variants |
| serialization | 0.6667 | 0.5000 | 0.6667 | 0.5000 | 0.6667 | baseline / file_only / combined |
| background_tasks | 0.0000 | 0.5000 | 0.3750 | 0.3750 | 0.0000 | parent_only |
| dependency_injection | 0.3333 | 0.5000 | 0.3333 | 0.5000 | 0.3333 | parent_only / siblings_only |
| validation | 0.2500 | 0.0000 | 0.2500 | 0.2500 | **0.6250** | combined |

### Per-category delta (break_mean − ctrl_mean): baseline vs file_only

| Category | baseline delta | file_only delta | lift |
|---|---:|---:|---:|
| framework_swap | +0.9531 | +1.0738 | +0.1207 |
| downstream_http | +0.5790 | +0.7302 | +0.1512 |
| async_blocking | +0.4756 | +0.5231 | +0.0475 |
| serialization | +0.3493 | +0.1731 | −0.1762 |
| exception_handling | +0.3404 | +0.7137 | +0.3733 |
| routing | −0.2428 | +0.8300 | +1.0728 |
| dependency_injection | −0.2763 | −0.2198 | +0.0565 |
| background_tasks | −0.7850 | −0.1326 | +0.6524 |
| validation | −0.6484 | −0.5402 | +0.1082 |

Largest gains from `file_only`: `routing` (+1.07 delta), `exception_handling` (+0.37), `background_tasks` (+0.65). Serialization slightly regresses on delta (−0.18) despite AUC staying flat.

---

## 3. Fallback and truncation diagnostics (file_only)

| Category | Fixtures | fallback | truncated | Notes |
|---|---|---|---|---|
| framework_swap | 6 | 0 | 0 | Full file extracted cleanly |
| downstream_http | 5 | 0 | 0 | Full file extracted cleanly |
| exception_handling | 8 | 0 | 1 | 1 truncated, still 0.9167 AUC |
| async_blocking | 5 | 0 | 1 | — |
| routing | 5 | 0 | 2 | 2 truncated; still 0.6667 |
| serialization | 5 | 0 | 0 | — |
| background_tasks | 6 | 0 | 3 | 3/6 truncated |
| dependency_injection | 5 | 0 | 2 | 2/5 truncated |
| validation | 6 | 0 | 1 | — |

**No fallbacks anywhere** — file_only always extracted real context. Truncation is present in the 3 weak categories but is not the root cause (see §4).

---

## 4. Root cause of the 3 weak categories

All 3 have `fallback=0` — context was extracted correctly. The failure is in the **training distribution**.

### Pattern across all 3: FastAPI idiom is the rare pattern

| Category | Break pattern | Control pattern | Why model fails |
|---|---|---|---|
| `validation` | marshmallow, cerberus, `assert` | Pydantic `@field_validator` | Breaks are generic Python (common in corpus). Controls use Pydantic validators (rare in commit diffs). |
| `background_tasks` | `threading.Thread`, `asyncio.create_task`, queue | `BackgroundTasks.add_task()` | Threading/asyncio ubiquitous. `BackgroundTasks` is FastAPI-specific and rare. |
| `dependency_injection` | Class instantiation, manual generator drain | `Depends()` chains, `Annotated[..., Depends()]` | Class patterns everywhere. Advanced `Depends()` usage rare even in FastAPI commits. |

The JEPA predictor scores low-reconstruction-error = "normal" = control. In these 3 categories, the break pattern IS normal (it's everyday Python), and the control pattern is unusual (FastAPI-idiomatic). The full file context shows the same problem at file level — the corpus files don't flip the model's prior.

### Inverted fixtures (file_only: break scored below best control in same category)

| Fixture | Category | Score | ctrl_max | Truncated |
|---|---|---:|---:|---|
| paradigm_break_event_loop_blocking | async_blocking | 1.1896 | 1.3130 | No |
| paradigm_break_sync_requests_in_async | async_blocking | 0.7760 | 1.3721 | No |
| paradigm_break_threading_background | background_tasks | 0.8208 | 1.4153 | No |
| paradigm_break_asyncio_create_task | background_tasks | 0.8103 | 1.4153 | Yes |
| paradigm_break_run_in_executor_endpoint | background_tasks | 1.4146 | 1.4153 | No |
| paradigm_break_manual_generator_drain | dependency_injection | 1.4103 | 2.4812 | No |
| paradigm_break_class_instance_no_depends | dependency_injection | 1.5990 | 2.4812 | Yes |
| paradigm_break_flask_errorhandler | exception_handling | 0.8962 | 1.4070 | No |
| paradigm_break_flask_routing | routing | 2.0590 | 2.1617 | No |
| paradigm_break_imperative_route_loop | routing | 1.4184 | 2.1617 | Yes |
| paradigm_break_orjson_serialization | serialization | 0.9033 | 1.3853 | No |
| paradigm_break_marshmallow_schema | validation | 0.8057 | 2.1721 | No |
| paradigm_break_cerberus_validation | validation | 1.7311 | 2.1721 | No |
| paradigm_break_assert_validation | validation | 1.7225 | 2.1721 | No |

Notable: `paradigm_break_run_in_executor_endpoint` misses its control by **0.0007** — nearly correct. Several routing breaks (flask_routing, imperative_route_loop) also come close. These are candidates for targeted fixture score investigation.

---

## 5. Mode comparison observations

- **`parent_only`** beats `file_only` on `background_tasks` (0.50 vs 0.375) and `dependency_injection` (0.50 vs 0.333). The enclosing function scope isolates the paradigm signal better than the full file for these.
- **`combined`** uniquely fixes `validation` (0.625) — the best of any mode. The parent + sibling context together likely surfaces Pydantic import/usage patterns visible elsewhere in the file.
- **`siblings_only`** rarely beats `file_only`; it degrades `async_blocking` (0.667 vs 0.833). High fallback rate in corpus (90.5%) limits its effectiveness.
- **`file_only`** has the best overall AUC and is the most consistent: no category degresses vs baseline by more than 0.125, and it recovers `routing` from 0.333 to 0.667.

---

## 6. Decision

`file_only` is now the default context mode in `fixture_to_record` and `sweep.py`. The 3 weak categories require corpus-side intervention, not architecture changes.

---

## 7. Open questions for Phase 12

1. **Corpus filtering**: restrict training corpus to files that use FastAPI idioms (`Depends`, `BackgroundTasks`, Pydantic validators). Would shift the model's "normal" distribution toward FastAPI-idiomatic code.
2. **`combined` for validation**: combined uniquely reaches 0.625 on validation. Worth investigating whether a selective mode (file_only except for validation-adjacent fixtures) is practical.
3. **`parent_only` for DI / background_tasks**: parent scope isolates the function signature where `Depends()` and `BackgroundTasks` appear. A hybrid mode (parent_only for those 2 categories, file_only otherwise) could be explored.
4. **Fixture quality**: `paradigm_break_run_in_executor_endpoint` misses its control by 0.0007 — essentially a coin flip. Consider replacing with a stronger break example.
5. **Multi-seed CI**: current results are single-seed. Re-run `file_only` vs `baseline` with seeds `[0,1,2]` to get bootstrap CI before any production promotion.
