# Phase 10 — Fixture Audit (2026-04-21)

## Summary

All 27 FastAPI acceptance fixtures were audited against Phase 10 corpus evidence (1083 files, per-category frequency tables). 21 fixtures are classified **keep**: corpus evidence confirms both the idiomatic control patterns and the absence of the break patterns. 4 fixtures are classified **replace**: either the break pattern is measurable in the corpus (reducing contrast), the fixture conflates two independent structural axes, or the signal is so sparse the category produces no useful AUC (serialization AUC 0.0 in ast_oov). 2 fixtures are classified **split**: `paradigm_break_subtle_wrong_exception` and `paradigm_break_subtle_exception_swallow` both mix fully-idiomatic scaffolding with a single structural axis of deviation, and splitting them off a shared control base would give cleaner per-axis signal. The serialization inversion hypothesis was **partially confirmed** — the raw_response break fixture uses `jsonable_encoder` (78 corpus sites, in-vocabulary), suppressing its OOV score, while the orjson fixture generates heavy OOV via `orjson.*` calls (only 2 corpus import sites).

---

## Serialization Inversion Analysis

### Hypothesis

The inversion hypothesis: the two serialization break fixtures may produce *fewer* OOV features than the control (`control_response_model.py`) because `jsonable_encoder` is common in the corpus (78 calls) and would be treated as in-vocabulary by the `ast_oov` scorer.

### Method

Inline script using `argot.research.signal.ast_features.extract_features` on all three files. The `ast_oov` scorer counts `(AST-node-class, dotted-identifier)` pairs unseen in the corpus training vocabulary; a higher count = more surprising = higher break score.

### Results

**Feature type counts (non-zero AST key types):**

| File | Non-zero feature types | Total identifier tokens |
|------|----------------------:|------------------------:|
| `paradigm_break_raw_response.py` | 15 | 97 |
| `paradigm_break_orjson_serialization.py` | 14 | 109 |
| `control_response_model.py` | 17 | 118 |

**Keys unique to break (not in control) — these become OOV candidates vs. control:**

| Break file | Keys not in control |
|-----------|---------------------|
| `paradigm_break_raw_response.py` | `FunctionDef`, `DictComp` (2 keys) |
| `paradigm_break_orjson_serialization.py` | `FunctionDef`, `AsyncFunctionDef` (2 keys) |

**Keys unique to control (not in break) — structural features the scorers find "missing" from breaks:**

| Comparison | Keys in control but not in break |
|-----------|----------------------------------|
| control vs raw_response | `ClassDef`, `keyword`, `Return`, `BinOp` (4 keys) |
| control vs orjson | `ClassDef`, `FormattedValue`, `ListComp`, `Return`, `comprehension` (5 keys) |

### Interpretation

**`paradigm_break_raw_response.py`:** The fixture calls `jsonable_encoder` (3 times) and `json.dumps` (3 times). The corpus has 78 `jsonable_encoder` calls — so `jsonable_encoder` is firmly in-vocabulary and contributes *zero* OOV score. The fixture also lacks the `ClassDef` (Pydantic BaseModel subclasses), `Return`, and `keyword` nodes that the control has. This means the **control** is actually *more structurally diverse* than the break by AST node count. The break produces only 2 unique OOV key types (vs. 4 keys the control has that the break lacks), confirming the hypothesis: the break looks structurally simpler than the control, not more complex.

**`paradigm_break_orjson_serialization.py`:** Generates 18 `orjson.*` Attribute tokens and 7 `orjson.dumps` Call tokens. The corpus has only 2 `orjson` import sites — so virtually all `orjson.*` identifiers are OOV relative to the corpus vocabulary. This break should produce a *high* OOV score. However, it still lacks the `ClassDef`, `ListComp`, `Return`, `comprehension`, and `FormattedValue` nodes present in the control, meaning the control has 5 structural patterns the break misses.

### Verdict

**Partially confirmed.** The `raw_response` break is confirmed inverted: `jsonable_encoder` is in-vocabulary (78 corpus sites), so this break contributes fewer OOV tokens than an unfamiliar library would. The corpus evidence explains why `serialization` AUC = 0.0 for `ast_oov` (Phase 9). The `orjson` break is *not* inverted — `orjson.*` identifiers are mostly OOV (2 corpus import sites) and this fixture likely generates more OOV than the control. The inversion is asymmetric between the two break fixtures: `raw_response` is the true culprit for the 0.0 AUC in the category.

**Recommended fix:** Replace `paradigm_break_raw_response.py` with a break using genuinely foreign serialization vocabulary (e.g., `msgpack.dumps`, `pickle.dumps`, or marshmallow `schema.dump()`) that is absent from the corpus rather than using `jsonable_encoder` which is corpus-idiomatic.

---

## Per-fixture verdict

| Name | Category | is_break | Verdict | Cited evidence | Action |
|------|----------|----------|---------|----------------|--------|
| paradigm_break_flask_routing | routing | yes | keep | `@app.route` + `request.get_json()` + `jsonify()` + `abort()`: 0 sites in corpus; `@app.get` dominant (833 sites) | none |
| paradigm_break_django_cbv | framework_swap | yes | keep | Django `View` subclasses + `request.body` + `JsonResponse` + `urlpatterns`: 0 corpus sites; FastAPI uses function-based endpoints throughout | none |
| paradigm_break_aiohttp_handler | framework_swap | yes | keep | `web.Request` + `web.json_response()` + `web.HTTPNotFound()` + `app.router.add_get()`: 0 corpus sites; entirely foreign vocabulary | none |
| paradigm_break_manual_validation | validation | yes | keep | `manual isinstance+raise in endpoint`: 0 corpus sites; `BaseModel subclasses`: 384 corpus sites confirms the contrast | none |
| paradigm_break_raw_response | serialization | yes | replace | `jsonable_encoder()`: 78 corpus sites (IN vocabulary) — inversion confirmed; break uses in-corpus identifier, suppressing OOV score; `JSONResponse(content=...) in endpoint`: 9 corpus sites — not actually absent | Replace with a fixture using msgpack/pickle/schema.dump() vocabulary genuinely absent from corpus |
| paradigm_break_subtle_wrong_exception | exception_handling | yes | split | `ValueError`: 10 corpus sites, `RuntimeError`: 4 sites — not zero; break pattern exists in corpus; `HTTPException`: 78 sites confirms canonical idiom; fixture mixes idiomatic scaffolding with non-zero-count exception vocabulary | Split: create a dedicated control that uses only HTTPException, and a break that uses only ValueError/KeyError with zero HTTPException — avoids mixed structural signal |
| paradigm_break_subtle_manual_status_check | downstream_http | yes | keep | `raise_for_status()`: 18 corpus sites confirms canonical pattern; `if response.status_code >= 400` branching: 0 corpus sites — clean contrast | none |
| paradigm_break_subtle_sync_endpoint | async_blocking | yes | keep | `time.sleep(sync)`: 5 corpus sites (all in non-endpoint utility code); `anyio.to_thread`: 2 sites for intentional offload; sync+blocking-I/O inside `async def` endpoint: 0 corpus sites | none |
| paradigm_break_subtle_exception_swallow | exception_handling | yes | split | Broad `except Exception: pass` / `logger.warning`: 0 corpus sites confirms absence; but fixture duplicates the same scaffolding as wrong_exception break — same category, same control anchor, redundant structural overlap | Split: keep as separate fixture but pair with a dedicated control that has no try/except at all, giving a cleaner before/after contrast |
| control_router_endpoint | routing | no | keep | `@router.get/@router.post`: 20+11 corpus sites; `Depends()`: 428 sites; `HTTPException(404)`: 78 sites; `response_model=`: 163 sites — all patterns deeply in-distribution | none |
| control_dependency_injection | dependency_injection | no | keep | `Depends(...)`: 428 sites; `Annotated[T, Depends(...)]`: 1136 Annotated usages; generator deps with `yield`: 75 sites — all canonical, corpus well-populated | none |
| control_exception_handling | exception_handling | no | keep | `@app.exception_handler(...)`: 12 corpus sites; `raise HTTPException(...)`: 78 sites; `JSONResponse` with `exc.errors()`: supported by corpus pattern — idiomatic | none |
| paradigm_break_starlette_mount | routing | yes | keep | `add_api_route() imperative`: 6 corpus sites; bare `Router` + `add_route()` + `url_for()` + `{id:int}` path converters: 0 corpus sites for this pattern combination; `app.mount()`: 6 sites but with sub-FastAPI apps not raw Starlette | none |
| paradigm_break_tornado_handler | framework_swap | yes | keep | `RequestHandler` subclass + `self.get_argument()` + `self.write()` + `IOLoop`: 0 corpus sites — entirely foreign vocabulary | none |
| paradigm_break_marshmallow_schema | validation | yes | keep | `marshmallow Schema.load()` + `@validates` + `@post_load` + `fields.String/Float`: 0 corpus sites; `BaseModel subclasses`: 384 sites confirms the contrast | none |
| paradigm_break_cerberus_validation | validation | yes | keep | `cerberus Validator` + `v.errors` + `validator.validate()` + `validator.normalized()`: 0 corpus sites — foreign vocabulary absent from corpus | none |
| paradigm_break_orjson_serialization | serialization | yes | keep | `orjson import sites`: 2 corpus sites (nearly OOV); `orjson.dumps()` with OPT_* flags at every endpoint: 0 endpoint-level sites; contrast with `response_model=` (163 sites) is strong | none (inversion does not apply here — orjson vocabulary is genuinely OOV) |
| paradigm_break_bare_except | exception_handling | yes | keep | Bare `except:` with no exception type silently returning empty data: 0 corpus sites; `raise HTTPException(...)`: 78 sites confirms the control idiom | none |
| paradigm_break_event_loop_blocking | async_blocking | yes | keep | `asyncio.get_event_loop().run_until_complete()` inside `async def` endpoint: 0 corpus sites; `anyio.to_thread`: 2 sites for the correct pattern | none |
| paradigm_break_sync_requests_in_async | downstream_http | yes | keep | `requests.Session` with `cert=/verify=` in async endpoint: 0 corpus sites; `httpx.AsyncClient`: corpus-confirmed async pattern; `requests usage (non-test files)`: 1 site (not endpoint use) | none |
| paradigm_break_global_singletons | dependency_injection | yes | replace | `module-level singleton assignments`: 922 corpus sites — module-level globals are extremely common in corpus (highest frequency of any DI feature); the fixture's break pattern (globals without Depends) overlaps heavily with normal corpus patterns; `Depends()`: 428 sites present in corpus but absence in this fixture is insufficient contrast given 922 global assignments in corpus | Replace with a fixture that uses a clearly non-idiomatic DI anti-pattern with genuinely absent vocabulary (e.g. using `threading.local()` for request state, or `contextvars` accessed directly without Depends()) |
| paradigm_break_threading_background | background_tasks | yes | keep | `Thread` + `Lock` + `daemon` + `t.join()`: 0 corpus sites in endpoint code; `BackgroundTasks parameter`: 10 sites; `background_tasks.add_task()`: 10 sites — strong contrast | none |
| control_pydantic_validator | validation | no | keep | `BaseModel subclasses`: 384 sites; `Query() param defaults`: 178 sites; `Field constraints`: corpus-confirmed — all patterns deeply in-distribution | none |
| control_response_model | serialization | no | keep | `response_model= on decorator`: 163 sites — dominant serialization pattern; `ClassDef` (BaseModel) + `Return` plain dict/model: corpus-confirmed idiomatic pattern | none |
| control_async_streaming | async_blocking | no | keep | Custom `APIRoute` + `Request`/`Response` + `HTTPException`: corpus-confirmed; `@asynccontextmanager`: 16 sites; async patterns idiomatic | none |
| control_httpx_async | downstream_http | no | keep | `raise_for_status()`: 18 corpus sites; `httpx.AsyncClient` as context manager in `Depends()`: canonical pattern; no `requests` library | none |
| control_annotated_depends | dependency_injection | no | keep | `Annotated[T, Depends(...)]`: 1136 Annotated usages in corpus (note: manifest cites 646, corpus doc shows 1136); generator deps with `yield`: 75 sites — deeply in-distribution | none |

---

## Replacement actions summary

| Fixture | Reason | Proposed replacement |
|---------|--------|----------------------|
| `paradigm_break_raw_response.py` | `jsonable_encoder` (78 corpus sites) is in-vocabulary; inversion confirmed; AUC 0.0 in ast_oov | Break using `msgpack.packb()` / `pickle.dumps()` or marshmallow `schema.dump()` — vocabulary genuinely absent from corpus |
| `paradigm_break_global_singletons.py` | `module-level singleton assignments`: 922 corpus sites — the break pattern is the most common DI-adjacent pattern in the corpus; contrast is weak | Break using `threading.local()` or raw `contextvars.ContextVar` accessed directly in endpoints without `Depends()` — neither appears in corpus |
| `paradigm_break_subtle_wrong_exception.py` (split) | `ValueError`: 10 sites, `RuntimeError`: 4 sites — non-zero in corpus; mixes idiomatic scaffolding with a break that partly overlaps corpus | Split into: (1) a pure HTTPException control, (2) a break that raises only ValueError/KeyError with no HTTPException at all |
| `paradigm_break_subtle_exception_swallow.py` (split) | Redundant structural overlap with wrong_exception in same category; needs a dedicated clean control for the swallow axis | Split: pair with a minimal control (no try/except), keep the swallow fixture as-is but remove idiomatic try/except blocks from the control anchor |

---

## Notes on category-level concerns

- **serialization** (AUC 0.0 in ast_oov, Phase 9): Root cause is `paradigm_break_raw_response.py` — the inversion makes the control look *more* OOV than the break. Fixing this single fixture should restore signal.
- **exception_handling** (AUC 0.6667 across all scorers, Phase 9): Three break fixtures / one control; the two fixtures flagged for split (`wrong_exception`, `exception_swallow`) contribute mixed signal. Cleaner splits would likely lift this category above 0.6667.
- **framework_swap** and **background_tasks**: no per-category AUC reported in Phase 9 (n/a) — likely a single-class evaluation issue. Fixture quality is good but the evaluation harness needs at least one control per category.
- **downstream_http**: AUC 1.0 with JEPA — fixtures are well-contrasted and corpus-backed. No changes needed.
