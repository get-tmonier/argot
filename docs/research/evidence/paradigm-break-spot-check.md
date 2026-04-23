# Paradigm-break spot-check: no transfer to real style violations

## Setup

Phase 8 was the handoff sanity check at the end of Era 2. The best
Phase 7 pipeline was trained on two production repos — argot's own
codebase split by language — and scored against hand-authored
paradigm-break fixtures. The Effect-TS model trained on argot `cli/` +
vigie `packages/app/` (both Effect-TS); the Python model trained on
argot `engine/` only. Fixtures paired paradigm-break code (OOP class
in an Effect codebase, `console.log` instead of `Effect.log`, raw
`throw`, `print` in Python, stdlib `logging`, raw `pathlib`) against
controls of idiomatic code in the same codebase. Decision gate:
overall delta ≥ 0.20 between break and control scores.

## Results

| side | control | break  | delta  |
|:-----|--------:|-------:|-------:|
| CLI (TypeScript) | 1.0891 | 1.2187 | 0.1296 |
| Engine (Python)  | 1.0543 | 0.9635 | **−0.0909** |
| **Overall**      |        |        | **0.0646** |

**Gate: NO-GO.** Measured overall delta 0.0646 against gate 0.20 —
0.1354 short. The Python side came in at **−0.0909**: paradigm-break
fixtures scored *lower* than controls, the opposite of the intended
direction. Per-fixture highlights: `paradigm_break_composition`
(CLI) hit 1.4081 vs control 1.3189 (smallest per-fixture gap on the
working side); engine-side `paradigm_break_logging` scored 0.7467 and
`paradigm_break_pathlib` scored 0.9709, both below their 1.0543
control.

## Interpretation

Whatever signal the Phase 7 pipeline had learned did not transfer to
real-world style violations. The spot-check made the Era 2 verdict
general: not just synthetic-eval-specific, not just mutation-specific
— the "distance from home" framing failed on paradigm breaks that were
obviously wrong to human readers. The negative Python delta ruled out
a soft fail where the pipeline "almost works"; it worked in the wrong
direction on half the scope, forcing the reframe into Era 3.
