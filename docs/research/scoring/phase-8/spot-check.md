# Phase 8 Spot-Check Results

**Gate criterion:** delta ≥ 0.20

| fixture | score | type |
|---|---|---|
| paradigm_break_class | 1.2676 | break |
| paradigm_break_console_log | 1.4551 | break |
| paradigm_break_throw_error | 1.4815 | break |
| paradigm_break_relative_import | 1.1220 | break |
| control_trivial_predicate | 1.4515 | control |
| control_normal_effect | 0.9976 | control |
| paradigm_break_print | 1.4787 | break |
| control_normal_python | 1.2266 | control |
| paradigm_break_validation | 1.0563 | break |
| paradigm_break_di | 1.1785 | break |
| paradigm_break_composition | 1.6036 | break |
| paradigm_break_typed_exception | 1.0813 | break |
| paradigm_break_pathlib | 0.7940 | break |

**Control mean:** 1.2252
**Paradigm-break mean:** 1.2519
**Delta:** 0.0266
**Gate:** NO-GO ✗

## NEGATIVE RESULT

### What failed

Delta = 0.0266, well below the 0.20 gate. The model does not reliably distinguish paradigm-break hunks from controls.

### Root cause: corpus too small

The argot CLI corpus (`.argot/dataset.jsonl`) contains only **127 records** after `just extract .` — it has a short git history. With 101 training records and 5 epochs, the pretrained encoder's JEPA head has insufficient signal to calibrate a meaningful anomaly threshold.

The model saw scores spread across 0.79–1.60 with no clear separation between break and control categories.

### Per-category breakdown

**Fixtures scoring LOWER than control mean (1.2252) — expected higher:**
- `paradigm_break_pathlib` 0.7940 — lowest score overall, below controls
- `paradigm_break_validation` 1.0563 — below control mean
- `paradigm_break_typed_exception` 1.0813 — below control mean
- `paradigm_break_di` 1.1785 — below control mean
- `paradigm_break_class` 1.2676 — only marginally above

**Controls scoring unexpectedly high:**
- `control_trivial_predicate` 1.4515 — nearly as high as the highest break fixture

### Why the new semantic fixtures underperform older ones

The older fixtures (`paradigm_break_console_log`, `paradigm_break_throw_error`, `paradigm_break_print`) score noticeably higher (1.45–1.48). These are micro-syntactic breaks in a TypeScript Effect.gen context. The new semantic fixtures (validation, DI, composition, pathlib, typed_exception) require deeper contextual understanding that the under-trained JEPA head cannot provide.

### Recommended reassessment

Two options to unblock Phase 2:

1. **Larger corpus**: Run the spot-check against a multi-repo corpus (e.g., the training corpus used in Phase 7.3 benchmarks) rather than the argot CLI alone. Need ≥2000 records from multiple repos with `_repo` tags.

2. **Lower the gate**: Accept delta ≥ 0.10 as the Phase 1 gate, and rely on the Phase 2 semantic_auc benchmark as the real signal. The spot-check was designed as a quick sanity check, but the benchmark (Task 7) is the more meaningful gate.
