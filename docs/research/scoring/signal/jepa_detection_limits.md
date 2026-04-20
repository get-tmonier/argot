# JEPA Predictor — Detection Limits

**Date:** 2026-04-20
**Based on:** Phases 1–7 signal sweep + v2 fixture expansion (27 fixtures, 9 categories)

---

## What the scorer actually does

The JEPA predictor is trained on commit history for a target repo. It learns "what does a normal hunk look like, given the context before it?" Scoring is anomaly-based: high score = the hunk surprised the model. It is fundamentally a **vocabulary-level + structural-sequence detector**, not a semantic reasoner.

---

## Breaks it detects reliably (delta ≥ 0.20)

These all share one property: the hunk contains **tokens entirely absent from the training corpus**.

| category | example | why it works |
|---|---|---|
| `routing` | Flask `@app.route(..., methods=[...])` in a FastAPI repo | `jsonify`, `abort`, `get_json`, `app.run` are zero-frequency in FastAPI corpus |
| `framework_swap` | Django CBVs, aiohttp handlers, tornado RequestHandler | Entire class hierarchies (`View`, `web.Request`, `RequestHandler`) are absent |
| `serialization` | orjson at every call site | `orjson`, `OPT_INDENT_2`, `OPT_SORT_KEYS` are absent |
| `exception_handling` | wrong exception types (`ValueError` instead of `HTTPException`) | `ValueError`, `KeyError`, `RuntimeError` in raise position are absent |

**Rule of thumb:** if the break hunk OOV rate is ≥ 8% against corpus vocabulary, the scorer will likely detect it.

---

## Breaks it struggles with (delta < 0.15)

These share the opposite property: the hunk uses **familiar tokens in an unusual arrangement**. The scorer sees nothing surprising at the vocabulary level.

| category | example | why it fails |
|---|---|---|
| `downstream_http` | `requests.get()` inside `async def` | `requests` appears 8× in corpus; the structural problem (sync in async) is invisible to the model |
| `background_tasks` | `threading.Thread().start()` in endpoint | Threading tokens are absent, but the hunk structure (function call in endpoint body) is indistinguishable from normal |
| `dependency_injection` | global singletons, no `Depends()` | All tokens (`get_db()`, global vars) are corpus-present; the absence of `Depends()` is not detectable |
| `async_blocking` | `asyncio.get_event_loop().run_until_complete()` | `asyncio` appears 2× in corpus — barely absent, not strongly foreign |
| `validation` | marshmallow/cerberus instead of Pydantic | `Schema`, `fields`, `Validator` are corpus-present; only the module import (`marshmallow`) carries signal |

**Rule of thumb:** if the break is "correct vocabulary, wrong idiom" — the scorer cannot detect it. It has no model of what a correct idiom is.

---

## The ceiling

After 7 stages of hyperparameter tuning (ensemble size, InfoNCE contrastive loss, corpus sampling diversity):

| metric | value |
|---|---|
| Best delta on obvious breaks (5 fixtures) | 0.3295 |
| Best delta on full v1 set (9 breaks + 3 controls) | 0.2291 |
| delta on v2 set (19 breaks + 8 controls, 9 categories) | ~0.09–0.14 |

The gap between v1 and v2 is not a scorer tuning problem — it reflects the proportion of detectable vs undetectable break types. Adding more subtle-idiom categories pulls the mean down regardless of scorer quality.

---

## What a better approach looks like

For the undetectable categories, vocabulary-based anomaly scoring has reached its limit. The violations are **semantically defined** — "this call blocks the event loop", "this function should use `Depends()`" — and require a model that understands code semantics.

**LLM-based scoring** (give the model the corpus vocabulary + hunk, ask "is this idiomatic?") has no vocabulary ceiling on these cases. A hybrid approach makes sense:
- JEPA scorer for coarse foreign-framework detection (fast, cheap, works well)
- LLM scorer for semantic idiom violations (slower, more expensive, necessary for subtle cases)
