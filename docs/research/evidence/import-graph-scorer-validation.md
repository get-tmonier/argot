# ImportGraphScorer: phase-13 domain validation

## Setup

Phase 14 opened with a one-line hypothesis: the false-negative patterns
on both AST and BPE pointed at the same class of breaks — hunks that
introduced foreign libraries (`import flask`, `import colorama`,
`import mimesis`). `ImportGraphScorer` counts top-level module imports
in the hunk that were never seen in `model_A`. Zero training, one pass
over the source tree, no calibration.

The validation ran the scorer across all three phase-13 corpora using
their existing fixture sets: FastAPI (20 control files into `model_A`,
31 breaks + 20 controls), rich (72 source files, 10 breaks + 10 controls),
and faker (722 source files, 5 breaks + 159 calibration hunks). A
critical diagnostic: does the scorer ignore `faker_hunk_0047`, the
error-handling hunk that caused faker FULL OVERLAP in era 3?

## Results

Combined break recall 67% (31/46); FP rate 0% across every validation
set.

| domain | breaks flagged | recall | FP / total | FP rate | verdict |
|:---|:---:|---:|:---:|---:|:---|
| FastAPI | 20/31 | 65% | 0/20 | 0% | PARTIAL |
| rich | 6/10 | 60% | 0/10 | 0% | PARTIAL |
| faker | 5/5 | 100% | 0/159 | 0% | STRONG |
| **combined** | **31/46** | **67%** | **0/189** | **0%** | — |

Per-domain: 100% on faker, 60–65% on FastAPI and rich.
`faker_hunk_0047` returned `import_score = 0` — correctly ignored. The
hunk is plain error-handling code with no foreign imports, so the axis
matches the intuition. FNs concentrated on stdlib-only paradigm breaks
(`assert_validation`, `bare_except`, `manual_generator_drain`, raw ANSI
escape codes, `print_manual`) — breaks that express non-idiomatic
patterns without crossing a module boundary.

## Interpretation

The scorer is complementary-signal, not a standalone primary scorer.
It delivers instant, high-precision flagging on library-crossing breaks
with zero training cost, but cannot see stdlib-only paradigm violations.
The 0% FP rate across all validation sets — and the correct suppression
of `faker_hunk_0047` — argued for the sequential pipeline that followed:
import as a fast pre-filter, BPE as the recovery axis for the 33% it
misses.
