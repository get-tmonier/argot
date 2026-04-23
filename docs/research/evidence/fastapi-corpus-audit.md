# FastAPI corpus audit: grounding fixtures in real frequency

## Setup

Phase 10 parsed a full FastAPI clone at HEAD
`2fa00db8581bb4e74b2d00d859c8469b6da296c4` to build per-category
feature frequency tables, so each paradigm-break fixture could be
audited against how often its "break" vocabulary actually appeared
in the target corpus. The goal was twofold: catch fixtures whose
break pattern was more corpus-present than their control (inverting
the eval signal), and expand the v2 set into a balanced 9-category
matrix with at least one control per category.

## Results

1083 FastAPI files parsed, 0 parse errors. Selected feature counts
across `fastapi/`, `docs_src/`, and `tests/`:

| pattern | count |
|:---|---:|
| `@app.get` | 833 |
| `Depends(...)` | 428 |
| `BaseModel` | 384 |
| `raise HTTPException` | 78 |
| `BackgroundTasks` parameter | 10 |
| `asyncio.create_task` in endpoint | 0 |

Fixture set expanded from 27 to 51 (31 break, 20 control, 9 categories). Two inverted fixtures were caught and replaced:
`paradigm_break_raw_response` (its `jsonable_encoder` pattern
appeared 78× in corpus — firmly in-vocabulary, so the break scored
*lower* OOV than the control) and `paradigm_break_global_singletons`
(module-level singleton assignments occurred 922× — the
highest-frequency DI-adjacent pattern in the corpus, making the
"break" indistinguishable from normal code). The two new breaks —
`msgpack_response` (0 corpus sites) and `class_instance_no_depends`
(0 sites) — restore the intended signal.

## Interpretation

Grounding fixture vocabulary in corpus frequency turned the eval
from "plausibly wrong" to falsifiable. The audit also surfaced the
structural categories where token frequency alone cannot work —
`BackgroundTasks` parameter × 10 vs `Thread` / `create_task`
in endpoint × 0 — forewarning the later `background_tasks`
inversion under both JEPA and tfidf.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase10_corpus_analysis_2026-04-21.md`.
Re-written here for clarity, not copied.*
