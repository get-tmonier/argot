# Phase 8 Spot-Check Results

**Training:** each scope trains on argot's own git history filtered by path
- `cli/` scope → TypeScript/Effect model (argot CLI package)
- `engine/` scope → Python model (argot engine package)

**Gate criterion:** overall delta ≥ 0.20

| fixture | scope | score | type |
|---|---|---|---|
| paradigm_break_class | cli | 1.1833 | break |
| paradigm_break_console_log | cli | 1.4832 | break |
| paradigm_break_throw_error | cli | 1.4123 | break |
| paradigm_break_promise | cli | 0.7586 | break |
| control_trivial_predicate | cli | 1.3968 | control |
| control_normal_effect | cli | 0.7690 | control |
| paradigm_break_print | engine | 1.1532 | break |
| control_normal_python | engine | 1.0613 | control |
| paradigm_break_validation | cli | 1.0647 | break |
| paradigm_break_di | cli | 1.1786 | break |
| paradigm_break_composition | cli | 1.5986 | break |
| paradigm_break_logging | engine | 0.7193 | break |
| paradigm_break_pathlib | engine | 0.9296 | break |

**CLI (TypeScript):** control=1.0829  break=1.2399  delta=0.1570
**Engine (Python):** control=1.0613  break=0.9340  delta=-0.1273
**Overall delta:** 0.0724
**Gate:** NO-GO ✗
