# Phase 14 Stage-2 Recall Isolation Probe (2026-04-22)

**Purpose:** Measure Stage 2 (BPE-tfidf) recall independently of Stage 1
(import detection).  Step L proved 124/124 recall but all fixtures fired via
Stage 1.  This probe forces Stage 2 by using `Stage2OnlyScorer` (import_score
permanently 0) across two phases.

**Scorer:** `Stage2OnlyScorer` (experiment subclass, Stage 1 disabled)
**Per-PR recalibration:** identical to fix6 (git archive → sample_hunks seed=0 N=100)

---

## §0 Summary

| phase | fixtures | host PRs | total pairs | flagged | catch rate | gate | result |
|---|---|---|---|---|---|---|---|
| Phase 1 — catalog breaks | 31 | 4 | 124 | 118 | 95.2% | ≥50% | PASS |
| Phase 2 — stage2-only fixtures | 8 | 4 | 32 | 32 | 100.0% | ≥70% | PASS |

**Verdict: STAGE-2 FUNCTIONAL — V0 unblocked on this dimension.**

---

## §1 Phase 1 — Catalog Break Fixtures Through Stage 2 Only

All 31 break fixtures from the acceptance catalog, scored via Stage2OnlyScorer.
Import-flagged fixtures from Step L are now tested on BPE merit alone.

### Per-fixture catch rate

| fixture | category | flagged (of 4 hosts) | catch rate |
|---|---|---|---|
| paradigm_break_flask_routing | routing | 4/4 | 100% |
| paradigm_break_django_cbv | framework_swap | 4/4 | 100% |
| paradigm_break_aiohttp_handler | framework_swap | 4/4 | 100% |
| paradigm_break_manual_validation | validation | 4/4 | 100% |
| paradigm_break_subtle_wrong_exception | exception_handling | 4/4 | 100% |
| paradigm_break_subtle_manual_status_check | downstream_http | 4/4 | 100% |
| paradigm_break_subtle_sync_endpoint | async_blocking | 4/4 | 100% |
| paradigm_break_subtle_exception_swallow | exception_handling | 4/4 | 100% |
| paradigm_break_starlette_mount | routing | 4/4 | 100% |
| paradigm_break_tornado_handler | framework_swap | 4/4 | 100% |
| paradigm_break_voluptuous_validation | validation | 1/4 | 25% ⚠ |
| paradigm_break_cerberus_validation | validation | 4/4 | 100% |
| paradigm_break_manual_json_response | serialization | 4/4 | 100% |
| paradigm_break_bare_except | exception_handling | 4/4 | 100% |
| paradigm_break_event_loop_blocking | async_blocking | 4/4 | 100% |
| paradigm_break_sync_requests_in_async | downstream_http | 4/4 | 100% |
| paradigm_break_concurrent_futures_background | background_tasks | 4/4 | 100% |
| paradigm_break_sync_file_io_async | async_blocking | 4/4 | 100% |
| paradigm_break_multiprocessing_background | background_tasks | 4/4 | 100% |
| paradigm_break_queue_carryover | background_tasks | 4/4 | 100% |
| paradigm_break_atexit_background | background_tasks | 4/4 | 100% |
| paradigm_break_manual_generator_drain | dependency_injection | 4/4 | 100% |
| paradigm_break_class_instance_no_depends | dependency_injection | 4/4 | 100% |
| paradigm_break_aiohttp_no_context | downstream_http | 4/4 | 100% |
| paradigm_break_json_error_response | exception_handling | 4/4 | 100% |
| paradigm_break_traceback_in_response | exception_handling | 4/4 | 100% |
| paradigm_break_flask_errorhandler | exception_handling | 4/4 | 100% |
| paradigm_break_imperative_route_loop | routing | 1/4 | 25% ⚠ |
| paradigm_break_manual_dict_response | serialization | 4/4 | 100% |
| paradigm_break_msgpack_response | serialization | 4/4 | 100% |
| paradigm_break_assert_validation | validation | 4/4 | 100% |

### Per-host catch rate (Phase 1)

| host PR | threshold | flagged | catch rate |
|---|---|---|---|
| #14862 | 4.1047 | 29/31 | 93.5% |
| #14944 | 4.0155 | 29/31 | 93.5% |
| #14856 | 4.1115 | 29/31 | 93.5% |
| #14806 | 3.2696 | 31/31 | 100.0% |

### Phase 1 — per-fixture × per-host score table

Cell format: `YES bpe>thr` / `no bpe<thr`.

| fixture | category | #14862 | #14944 | #14856 | #14806 |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | routing | **YES** 5.772>4.105 | **YES** 5.772>4.015 | **YES** 5.772>4.112 | **YES** 5.772>3.270 |
| paradigm_break_django_cbv | framework_swap | **YES** 6.365>4.105 | **YES** 6.365>4.015 | **YES** 6.365>4.112 | **YES** 6.365>3.270 |
| paradigm_break_aiohttp_handler | framework_swap | **YES** 7.307>4.105 | **YES** 7.307>4.015 | **YES** 7.307>4.112 | **YES** 7.307>3.270 |
| paradigm_break_manual_validation | validation | **YES** 6.226>4.105 | **YES** 6.226>4.015 | **YES** 6.226>4.112 | **YES** 6.226>3.270 |
| paradigm_break_subtle_wrong_exception | exception_handling | **YES** 6.852>4.105 | **YES** 6.852>4.015 | **YES** 6.852>4.112 | **YES** 6.852>3.270 |
| paradigm_break_subtle_manual_status_check | downstream_http | **YES** 4.499>4.105 | **YES** 4.499>4.015 | **YES** 4.499>4.112 | **YES** 4.499>3.270 |
| paradigm_break_subtle_sync_endpoint | async_blocking | **YES** 7.329>4.105 | **YES** 6.508>4.015 | **YES** 7.329>4.112 | **YES** 7.329>3.270 |
| paradigm_break_subtle_exception_swallow | exception_handling | **YES** 5.832>4.105 | **YES** 5.832>4.015 | **YES** 5.832>4.112 | **YES** 5.832>3.270 |
| paradigm_break_starlette_mount | routing | **YES** 5.623>4.105 | **YES** 5.623>4.015 | **YES** 5.623>4.112 | **YES** 5.623>3.270 |
| paradigm_break_tornado_handler | framework_swap | **YES** 6.845>4.105 | **YES** 6.845>4.015 | **YES** 6.845>4.112 | **YES** 6.845>3.270 |
| paradigm_break_voluptuous_validation | validation | no 3.996<4.105 | no 3.996<4.015 | no 3.996<4.112 | **YES** 3.996>3.270 |
| paradigm_break_cerberus_validation | validation | **YES** 5.347>4.105 | **YES** 5.347>4.015 | **YES** 5.347>4.112 | **YES** 5.347>3.270 |
| paradigm_break_manual_json_response | serialization | **YES** 5.926>4.105 | **YES** 5.926>4.015 | **YES** 5.926>4.112 | **YES** 5.926>3.270 |
| paradigm_break_bare_except | exception_handling | **YES** 6.781>4.105 | **YES** 6.781>4.015 | **YES** 6.781>4.112 | **YES** 6.781>3.270 |
| paradigm_break_event_loop_blocking | async_blocking | **YES** 7.388>4.105 | **YES** 7.388>4.015 | **YES** 7.388>4.112 | **YES** 7.388>3.270 |
| paradigm_break_sync_requests_in_async | downstream_http | **YES** 7.329>4.105 | **YES** 6.508>4.015 | **YES** 7.329>4.112 | **YES** 7.329>3.270 |
| paradigm_break_concurrent_futures_background | background_tasks | **YES** 7.119>4.105 | **YES** 7.119>4.015 | **YES** 7.119>4.112 | **YES** 7.119>3.270 |
| paradigm_break_sync_file_io_async | async_blocking | **YES** 7.804>4.105 | **YES** 7.804>4.015 | **YES** 7.804>4.112 | **YES** 7.804>3.270 |
| paradigm_break_multiprocessing_background | background_tasks | **YES** 6.553>4.105 | **YES** 6.553>4.015 | **YES** 6.553>4.112 | **YES** 6.553>3.270 |
| paradigm_break_queue_carryover | background_tasks | **YES** 6.553>4.105 | **YES** 6.553>4.015 | **YES** 6.553>4.112 | **YES** 6.553>3.270 |
| paradigm_break_atexit_background | background_tasks | **YES** 7.117>4.105 | **YES** 7.117>4.015 | **YES** 7.117>4.112 | **YES** 7.117>3.270 |
| paradigm_break_manual_generator_drain | dependency_injection | **YES** 6.243>4.105 | **YES** 5.467>4.015 | **YES** 6.243>4.112 | **YES** 6.243>3.270 |
| paradigm_break_class_instance_no_depends | dependency_injection | **YES** 5.476>4.105 | **YES** 5.025>4.015 | **YES** 5.476>4.112 | **YES** 5.476>3.270 |
| paradigm_break_aiohttp_no_context | downstream_http | **YES** 6.226>4.105 | **YES** 6.226>4.015 | **YES** 6.226>4.112 | **YES** 6.226>3.270 |
| paradigm_break_json_error_response | exception_handling | **YES** 5.832>4.105 | **YES** 5.832>4.015 | **YES** 5.832>4.112 | **YES** 5.832>3.270 |
| paradigm_break_traceback_in_response | exception_handling | **YES** 6.852>4.105 | **YES** 6.852>4.015 | **YES** 6.852>4.112 | **YES** 6.852>3.270 |
| paradigm_break_flask_errorhandler | exception_handling | **YES** 5.832>4.105 | **YES** 5.832>4.015 | **YES** 5.832>4.112 | **YES** 5.832>3.270 |
| paradigm_break_imperative_route_loop | routing | no 3.398<4.105 | no 3.281<4.015 | no 3.405<4.112 | **YES** 3.389>3.270 |
| paradigm_break_manual_dict_response | serialization | **YES** 6.504>4.105 | **YES** 6.504>4.015 | **YES** 6.504>4.112 | **YES** 6.504>3.270 |
| paradigm_break_msgpack_response | serialization | **YES** 7.128>4.105 | **YES** 7.128>4.015 | **YES** 7.128>4.112 | **YES** 7.128>3.270 |
| paradigm_break_assert_validation | validation | **YES** 7.482>4.105 | **YES** 7.482>4.015 | **YES** 7.482>4.112 | **YES** 7.482>3.270 |

### Phase 1 — failures (fixtures that never fire via Stage 2)

None — all fixtures fired via Stage 2 on at least one host PR.

### Phase 1 — partial misses (fire in some hosts only)

| fixture | category | hosts hit/total | scores vs threshold |
|---|---|---|---|
| paradigm_break_voluptuous_validation | validation | 1/4 | 3.996>3.270 / 3.996<4.112 / 3.996<4.105 / 3.996<4.015 |
| paradigm_break_imperative_route_loop | routing | 1/4 | 3.389>3.270 / 3.405<4.112 / 3.398<4.105 / 3.281<4.015 |

---

## §2 Phase 2 — Stage-2-Only Fixture Pack

8 fixtures using only stdlib / in-corpus imports.  Stage 1 cannot fire.
Tests whether BPE catches in-repo paradigm shifts (syntax/style, not framework swaps).

### Fixture catalogue

| fixture | pattern | hunk lines |
|---|---|---|
| walrus_operator | walrus := in while/if conditions — Python 3.8+ | 25 |
| match_case | match/case structural pattern matching — Python 3.10+ | 24 |
| dataclass_migration | @dataclass(frozen=True, slots=True) — host uses plain __init__ classes | 25 |
| fstring_adoption | f-strings with nested {val!r:>10} format specs throughout | 27 |
| async_adoption | asyncio.gather / to_thread / Semaphore concurrency primitives | 24 |
| genexpr_shift | sum/any/all genexpr chains where host uses list comprehensions | 21 |
| type_annotations | PEP 695 type parameters def f[T] / Protocol with covariant TypeVars | 27 |
| union_syntax | X | None / int | str union syntax throughout — Python 3.10+ | 23 |

### Per-fixture catch rate (Phase 2)

| fixture | flagged (of 4 hosts) | catch rate |
|---|---|---|
| walrus_operator | 4/4 | 100% |
| match_case | 4/4 | 100% |
| dataclass_migration | 4/4 | 100% |
| fstring_adoption | 4/4 | 100% |
| async_adoption | 4/4 | 100% |
| genexpr_shift | 4/4 | 100% |
| type_annotations | 4/4 | 100% |
| union_syntax | 4/4 | 100% |

### Per-host catch rate (Phase 2)

| host PR | threshold | flagged | catch rate |
|---|---|---|---|
| #14862 | 4.1047 | 8/8 | 100.0% |
| #14944 | 4.0155 | 8/8 | 100.0% |
| #14856 | 4.1115 | 8/8 | 100.0% |
| #14806 | 3.2696 | 8/8 | 100.0% |

### Phase 2 — per-fixture × per-host score table

| fixture | #14862 | #14944 | #14856 | #14806 |
|---|---|---|---|---|
| walrus_operator | **YES** 7.671>4.105 | **YES** 7.671>4.015 | **YES** 7.671>4.112 | **YES** 7.671>3.270 |
| match_case | **YES** 7.482>4.105 | **YES** 7.482>4.015 | **YES** 7.482>4.112 | **YES** 7.482>3.270 |
| dataclass_migration | **YES** 5.270>4.105 | **YES** 5.270>4.015 | **YES** 5.270>4.112 | **YES** 5.270>3.270 |
| fstring_adoption | **YES** 7.418>4.105 | **YES** 7.418>4.015 | **YES** 7.418>4.112 | **YES** 7.418>3.270 |
| async_adoption | **YES** 7.329>4.105 | **YES** 7.242>4.015 | **YES** 7.329>4.112 | **YES** 7.329>3.270 |
| genexpr_shift | **YES** 7.388>4.105 | **YES** 7.388>4.015 | **YES** 7.388>4.112 | **YES** 7.388>3.270 |
| type_annotations | **YES** 6.190>4.105 | **YES** 6.190>4.015 | **YES** 6.190>4.112 | **YES** 6.190>3.270 |
| union_syntax | **YES** 6.887>4.105 | **YES** 6.730>4.015 | **YES** 6.887>4.112 | **YES** 6.887>3.270 |

### Phase 2 — fixture-level analysis

#### walrus_operator (4/4 hosts)

Pattern: walrus := in while/if conditions — Python 3.8+
Max bpe_score: 7.6715 | top_token: `Ġpos` | top_llr: 7.6715

**4/4 — fires consistently across all host PRs.**

#### match_case (4/4 hosts)

Pattern: match/case structural pattern matching — Python 3.10+
Max bpe_score: 7.4816 | top_token: `Ġ<=` | top_llr: 7.4816

**4/4 — fires consistently across all host PRs.**

#### dataclass_migration (4/4 hosts)

Pattern: @dataclass(frozen=True, slots=True) — host uses plain __init__ classes
Max bpe_score: 5.2700 | top_token: `Record` | top_llr: 5.2700

**4/4 — fires consistently across all host PRs.**

#### fstring_adoption (4/4 hosts)

Pattern: f-strings with nested {val!r:>10} format specs throughout
Max bpe_score: 7.4182 | top_token: `table` | top_llr: 7.4182

**4/4 — fires consistently across all host PRs.**

#### async_adoption (4/4 hosts)

Pattern: asyncio.gather / to_thread / Semaphore concurrency primitives
Max bpe_score: 7.3290 | top_token: `Ġtimeout` | top_llr: 7.3290

**4/4 — fires consistently across all host PRs.**

#### genexpr_shift (4/4 hosts)

Pattern: sum/any/all genexpr chains where host uses list comprehensions
Max bpe_score: 7.3884 | top_token: `count` | top_llr: 7.3884

**4/4 — fires consistently across all host PRs.**

#### type_annotations (4/4 hosts)

Pattern: PEP 695 type parameters def f[T] / Protocol with covariant TypeVars
Max bpe_score: 6.1900 | top_token: `Protocol` | top_llr: 6.1900

**4/4 — fires consistently across all host PRs.**

#### union_syntax (4/4 hosts)

Pattern: X | None / int | str union syntax throughout — Python 3.10+
Max bpe_score: 6.8867 | top_token: `val` | top_llr: 6.8867

**4/4 — fires consistently across all host PRs.**

---

## §3 Verdict

| metric | value |
|---|---|
| Phase 1 catch rate (catalog breaks, Stage 2 only) | 95.2% |
| Phase 1 gate (≥50%) | PASS |
| Phase 2 catch rate (stage2-only fixtures) | 100.0% |
| Phase 2 gate (≥70%) | PASS |

**STAGE-2 FUNCTIONAL — V0 unblocked on this dimension.**

Both phases pass. Stage 2 is functional:
- It retains signal on catalog breaks even when Stage 1 is bypassed.
- It catches in-repo paradigm shifts (syntax/style patterns) above the 70% gate.
V0 is unblocked on the Stage-2 recall dimension.

