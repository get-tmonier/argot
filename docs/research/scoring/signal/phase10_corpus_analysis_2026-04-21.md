# Phase 10 — FastAPI Corpus Analysis (2026-04-21)

FastAPI clone: `/tmp/argot-fastapi-static` (HEAD SHA: `2fa00db8581bb4e74b2d00d859c8469b6da296c4`)
Analysis date: 2026-04-21

## Summary

Corpus: **1083** Python files parsed across `fastapi/`, `docs_src/`, and `tests/` (0 parse errors).
**Rich signal:** `validation` (BaseModel subclasses, field_validator, Query/Path/Body), `dependency_injection` (Depends sites, Annotated style), `routing` (decorator distribution, include_router), and `exception_handling` (raise HTTPException dominates). **Moderate signal:** `serialization` (response_model, jsonable_encoder) and `downstream_http` (httpx vs requests split across test/non-test). **Sparse signal:** `background_tasks` (narrow API — BackgroundTasks param + add_task only) and `async_blocking` (almost no blocking calls inside async functions, as expected for a clean async codebase). `framework_swap` overlaps heavily with routing.

## exception_handling

### Canonical idiom (top 3 citations)

- `docs_src/app_testing/app_b_an_py310/main.py:25 — raise HTTPException(...)`
- `docs_src/app_testing/app_b_an_py310/main.py:27 — raise HTTPException(...)`
- `docs_src/app_testing/app_b_an_py310/main.py:34 — raise HTTPException(...)`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `raise HTTPException(...)` | 78 |
| `@app.exception_handler(...)` | 12 |
| `JSONResponse({"error": ...})` | 0 |
| `make_not_authenticated_error` | 13 |
| `credentials_exception` | 12 |
| `ValueError` | 10 |
| `OtherDependencyError` | 6 |
| `RuntimeError` | 4 |
| `InternalError` | 4 |
| `FastAPIError` | 3 |

### Candidate break axes

- **raise HTTPException vs register exception_handler**: 78 inline raises vs 12 handler registrations — strong separator between 'handle-inline' and 'handle-globally' idioms.
- **JSONResponse({'error': ...}) vs HTTPException**: 0 manual JSON error responses — early/legacy pattern vs modern HTTPException.
- **Exception class diversity**: custom exception classes (non-HTTPException raises) indicate app-level error design sophistication.

## async_blocking

### Canonical idiom (top 3 citations)

- `fastapi/concurrency.py:32`
- `fastapi/concurrency.py:39`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `httpx.AsyncClient usage` | 0 |
| `anyio.to_thread usage` | 2 |
| `time.sleep(sync)` | 5 |

### Candidate break axes

- **httpx.AsyncClient present vs absent**: 0 AsyncClient usages — strongest positive signal for 'async-aware downstream'.
- **anyio.to_thread.run_sync present**: 2 sites — marks intentional thread offloading, not accidental blocking.
- **blocking call inside async def**: near-zero in this corpus (clean codebase), so presence = strong negative signal.

## dependency_injection

### Canonical idiom (top 3 citations)

- `fastapi/param_functions.py:2369`
- `docs_src/authentication_error_status_code/tutorial001_an_py310.py:16`
- `docs_src/background_tasks/tutorial002_an_py310.py:22`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `Depends(...) call sites` | 428 |
| `Annotated[ usage` | 1136 |
| `generator deps (yield in Depends arg)` | 75 |
| `module-level singleton assignments` | 922 |

### Candidate break axes

- **Depends count**: 428 sites — raw count correlates with DI adoption depth.
- **Annotated vs bare Depends**: 1136 Annotated usages — modern (3.9+ / FastAPI 0.95+) style marker; break between legacy and idiomatic.
- **Generator deps**: 75 generator-style deps — resource-managing pattern (DB sessions, HTTP clients).

## background_tasks

### Canonical idiom (top 3 citations)

- `fastapi/dependencies/utils.py:598 — def solve_dependencies(..., background_tasks: BackgroundTasks)`
- `docs_src/background_tasks/tutorial001_py310.py:13 — def send_notification(..., background_tasks: BackgroundTasks)`
- `docs_src/background_tasks/tutorial002_an_py310.py:13 — def get_query(..., background_tasks: BackgroundTasks)`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `BackgroundTasks parameter` | 10 |
| `background_tasks.add_task(...)` | 10 |
| `asyncio.create_task (in endpoint)` | 0 |
| `Thread (in endpoint)` | 0 |
| `loop.run_in_executor (in endpoint)` | 0 |

### Candidate break axes

- **BackgroundTasks + add_task**: the canonical pair; 10 param usages / 10 add_task calls.
- **asyncio.create_task in endpoint**: 0 occurrences — non-idiomatic, leaks task lifecycle.
- **Thread in endpoint**: 0 occurrences — synchronous offload antipattern, strong negative signal.
- Key break: `BackgroundTasks` (canonical) vs `asyncio.create_task` / `Thread` (antipattern) vs nothing.

## serialization

### Canonical idiom (top 3 citations)

- `docs_src/additional_responses/tutorial001_py310.py:18 — app.get(response_model=...)`
- `docs_src/additional_responses/tutorial002_py310.py:14 — app.get(response_model=...)`
- `docs_src/additional_responses/tutorial003_py310.py:18 — app.get(response_model=...)`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `response_model= on decorator` | 163 |
| `jsonable_encoder() total` | 78 |
| `jsonable_encoder() inside endpoint` | 4 |
| `JSONResponse(content=...) total` | 14 |
| `JSONResponse(content=...) in endpoint` | 9 |
| `orjson import sites` | 2 |
| `ujson import sites` | 0 |

### Candidate break axes

- **response_model= present**: 163 usages — primary model-driven serialization axis.
- **jsonable_encoder inside endpoint**: 4 — manual serialization step, indicates custom output shaping.
- **orjson vs default**: 2 orjson imports — performance optimization marker; almost absent means default encoder dominates.
- **JSONResponse(content=) vs response_model**: explicit response construction vs declarative schema.

## routing

### Canonical idiom (top 3 citations)

- `docs_src/additional_responses/tutorial001_py310.py:18 — @app.get`
- `docs_src/additional_responses/tutorial002_py310.py:14 — @app.get`
- `docs_src/additional_responses/tutorial003_py310.py:18 — @app.get`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `@app.get` | 833 |
| `@pytest.mark.parametrize` | 427 |
| `@app.post` | 270 |
| `@pytest.fixture` | 195 |
| `@app.put` | 38 |
| `@router.get` | 20 |
| `@app.websocket` | 18 |
| `@router.post` | 11 |
| `@app.on_event` | 10 |
| `@app.exception_handler` | 9 |
| `add_api_route() imperative` | 6 |
| `app.mount()` | 6 |
| `include_router()` | 86 |
| `@asynccontextmanager` | 16 |

### Candidate break axes

- **@router.* vs @app.***: use of APIRouter signals modular app structure.
- **include_router count**: 86 — proxy for app decomposition into multiple routers.
- **add_api_route imperative**: 6 — programmatic vs declarative routing.
- **app.mount**: 6 — sub-application mounting (ASGI composition).

## framework_swap

### Canonical idiom (top 3 citations)

- `fastapi/applications.py:1550`
- `docs_src/bigger_applications/app_an_py310/main.py:10`
- `docs_src/bigger_applications/app_an_py310/main.py:11`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `include_router() calls` | 86 |
| `app.mount() calls` | 6 |
| `add_api_route() calls` | 6 |
| `@asynccontextmanager (lifespan)` | 16 |

### Candidate break axes

- **framework_swap overlaps heavily with routing**: both are concerned with application composition and lifecycle. The distinguishing axis for framework_swap is app-level structural patterns:
  - `app.mount()` (6) — ASGI sub-app composition
  - `@asynccontextmanager` (16) — lifespan event handling
  - Presence of multiple APIRouter modules vs a single flat app
- **add_api_route imperative**: strongly indicates dynamic/generated routing (e.g., plugin system).

## validation

### Canonical idiom (top 3 citations)

- `fastapi/openapi/models.py:57 — class BaseModelWithConfig(BaseModel)`
- `fastapi/openapi/models.py:61 — class Contact(BaseModel)`
- `fastapi/openapi/models.py:67 — class License(BaseModel)`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `BaseModel subclasses` | 384 |
| `@field_validator sites` | 2 |
| `Query() param defaults` | 178 |
| `Path() param defaults` | 66 |
| `Body() param defaults` | 103 |
| `manual isinstance+raise in endpoint` | 0 |

### Candidate break axes

- **BaseModel count**: 384 subclasses — raw schema coverage; higher = more declarative validation.
- **@field_validator**: 2 sites — custom validation logic beyond type checks.
- **Query/Path/Body defaults**: 178/66/103 — parameter-level validation (constraints, descriptions).
- **manual isinstance+raise**: 0 — imperative fallback, negative signal.

## downstream_http

### Canonical idiom (top 3 citations)

- `docs_src/async_tests/app_a_py310/test_main.py:2`

### Feature frequency table

| Pattern | Count |
|---------|-------|
| `httpx usage sites` | 1 |
| `requests usage (non-test files)` | 1 |
| `requests usage (test files)` | 0 |
| `raise_for_status() calls` | 18 |

### Candidate break axes

- **httpx vs requests in non-test code**: 1 httpx vs 1 requests (non-test) — httpx is the async-compatible choice; requests in production code = blocking smell.
- **raise_for_status()**: 18 sites — explicit error propagation from HTTP calls.
- **test files**: requests is expected in test clients (TestClient is requests-based); 0 test-file usages should be excluded from scoring.
