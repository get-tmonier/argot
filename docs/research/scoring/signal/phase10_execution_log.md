# Phase 10 — Execution Log (2026-04-21)

## Phases completed

| Phase | Status | Notes |
|-------|--------|-------|
| 1A — Corpus reconnaissance | ✅ | 1083 files, 0 parse errors |
| 1B — Fixture audit | ✅ | 4 flagged: 2 replaced, 2 kept |
| 2A — Parent-context features | ✅ | 25 tests passing |
| 2B — Co-occurrence features | ✅ | 27 tests passing |
| 1D wave 1 — background_tasks | ✅ | 3 break + 2 control |
| 1D wave 1 — framework_swap | ✅ | 3 control (breaks kept) |
| 1D wave 1 — serialization | ✅ | 2 break + 1 control, 1 inverted fixture replaced |
| 1D wave 1 — exception_handling | ✅ | 3 break + 1 control |
| 1D wave 2 — dependency_injection | ✅ | 2 break + 1 control, 1 in-vocab break replaced |
| 1D wave 2 — async_blocking | ✅ | 1 break + 1 control |
| 1D wave 2 — downstream_http | ✅ | 1 break + 1 control |
| 1D wave 2 — validation | ✅ | 1 break + 1 control |
| 1D wave 2 — routing | ✅ | 1 break + 1 control |
| 1E — Manifest consolidation | ✅ | 51 fixtures, 0 load failures |
| 3A — Evaluation grid | ✅ | 5 configs, 51 fixtures, caching enabled |
| 3B — Final report | ✅ | This document |

## Anomalies / skipped items

- `paradigm_break_subtle_wrong_exception` and `paradigm_break_subtle_exception_swallow`: audit recommended split but split was not executed to keep scope manageable. Logged for Phase 11.
- All pre-existing lint failures in `ast_structural_audit_test.py`, `llm_rerank_experiment.py`, `static_chunk_audit_test.py` are pre-existing, not introduced by Phase 10.
- `full` config results file contains only `ast_structural_full_oov` (the ll and zscore variants were not run for the full config); this is consistent with the Phase 10 plan, which targeted OOV as the primary full-config scorer.
- Per-category z-delta values are not stored in the results JSON (only overall Δ(z) per scorer); raw fixture scores are available in the `.pt` and `.json` files for offline recomputation.
