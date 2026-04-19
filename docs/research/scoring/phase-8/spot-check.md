# Phase 8 Spot-Check Results

**Training:** Effect-TS model = argot `cli/` + vigie `packages/app/` (both Effect-TS repos)
Python model = argot `engine/` only

**Gate criterion:** overall delta ≥ 0.20

| fixture | scope | score | type |
|---|---|---|---|
| paradigm_break_class | cli | 1.2668 | break |
| paradigm_break_console_log | cli | 1.3871 | break |
| paradigm_break_throw_error | cli | 1.3997 | break |
| paradigm_break_promise | cli | 0.7971 | break |
| control_trivial_predicate | cli | 1.3189 | control |
| control_normal_effect | cli | 0.8593 | control |
| paradigm_break_print | engine | 1.1729 | break |
| control_normal_python | engine | 1.0543 | control |
| paradigm_break_validation | cli | 1.0659 | break |
| paradigm_break_di | cli | 1.2062 | break |
| paradigm_break_composition | cli | 1.4081 | break |
| paradigm_break_logging | engine | 0.7467 | break |
| paradigm_break_pathlib | engine | 0.9709 | break |

**CLI (TypeScript):** control=1.0891  break=1.2187  delta=0.1296
**Engine (Python):** control=1.0543  break=0.9635  delta=-0.0909
**Overall delta:** 0.0646
**Gate:** NO-GO ✗
