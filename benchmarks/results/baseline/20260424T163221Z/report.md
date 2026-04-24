# argot-bench report

Generated: 2026-04-24T16:47:18.210786+00:00

## Headline

| Corpus | Lang | AUC | Recall | FP | Gap | N_fix | N_ctrl | Thr |
|:---|:---|---:|---:|---:|---:|---:|---:|---:|
| fastapi | python | 0.9880 | 91.7% | 0.8% | -4.371 | 32 | 79623 | 5.278 |
| rich | python | 0.9780 | 95.0% | 0.8% | -3.040 | 16 | 68598 | 4.164 |
| faker | python | 0.9537 | 95.0% | 1.2% | -4.946 | 16 | 75996 | 5.211 |
| hono | typescript | 0.8312 | 78.3% | 0.5% | -7.471 | 17 | 54717 | 4.277 |
| ink | typescript | 0.9899 | 93.3% | 0.4% | -4.633 | 17 | 16678 | 4.826 |
| faker-js | typescript | 0.9463 | 53.3% | 1.0% | -7.066 | 17 | 255760 | 4.773 |

_Gap = min(break) − max(control). Positive = clean separation; negative = overlap._

## fastapi (python)

### Summary

- **AUC (catalog vs real-PR controls):** 0.9880
- **Recall (mean across categories):** 91.7%
- **FP rate on real PR hunks:** 0.8% (261/79623)
- **Threshold (mean across seeds):** 5.2778 (CV: 0.4%)
- **Calibration stability:** rel_var=0.000106, jaccard=0.0694
- **Separation gap (min break − max control):** -4.3710 (overlap)
- **Sample sizes:** 32 fixtures · 79623 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 32 | 2.980 | 5.191 | 5.877 | 6.581 | 6.852 | 7.361 |
| Control (real PR) | 79623 | -1.250 | 0.000 | 0.000 | 0.891 | 2.347 | 7.351 |

Threshold **5.2778** — 9/32 breaks fall below it (misses), 75/79623 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "fastapi — recall by category"
    x-axis ["async_blocking", "background_tasks", "dependency_injection", "downstream_http", "exception_handling", "framework_swap", "routing", "serialization", "validation"]
    y-axis "recall %" 0 --> 110
    bar [100.0, 100.0, 100.0, 100.0, 83.3, 100.0, 66.7, 100.0, 75.0]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| async_blocking | 100.0% | 3/3 | 6.619 | 5.247 | 7.361 | async_blocking_1, async_blocking_2, async_blocking_3 |
| background_tasks | 100.0% | 4/4 | 6.567 | 6.553 | 6.581 | background_tasks_1, background_tasks_2, background_tasks_3, background_tasks_4 |
| dependency_injection | 100.0% | 3/3 | 5.711 | 5.025 | 6.641 | dependency_injection_1, dependency_injection_2, dependency_injection_3 |
| downstream_http | 100.0% | 3/3 | 5.196 | 3.557 | 6.153 | downstream_http_1, downstream_http_2, downstream_http_3 |
| exception_handling | 83.3% | 5/6 | 5.303 | 2.980 | 6.852 | exception_handling_1, exception_handling_2, exception_handling_3, exception_handling_4, exception_handling_5, exception_handling_6 |
| framework_swap | 100.0% | 3/3 | 5.449 | 5.281 | 5.786 | framework_swap_1, framework_swap_2, framework_swap_3 |
| routing | 66.7% | 2/3 | 3.536 | 2.980 | 4.309 | routing_1, routing_2, routing_3 |
| serialization | 100.0% | 3/3 | 6.503 | 5.877 | 7.128 | serialization_1, serialization_2, serialization_3 |
| validation | 75.0% | 3/4 | 5.264 | 2.980 | 6.504 | validation_1, validation_2, validation_3, validation_4 |

### Per-fixture results

<details>
<summary>32 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| async_blocking_1 | async_blocking | 5.247 | ✓ | call_receiver | breaks/paradigm_break_subtle_sync_endpoint.py | 41–85 | the FastAPI corpus is dominated by async def endpoints; sync def endpoints with blocking I/O (time.sleep, requests.get, blocking DB calls… |
| async_blocking_2 | async_blocking | 7.361 | ✓ | bpe | breaks/paradigm_break_event_loop_blocking.py | 18–59 | asyncio.get_event_loop().run_until_complete() called inside async def endpoint bodies — incorrect usage that raises RuntimeError inside a… |
| async_blocking_3 | async_blocking | 7.250 | ✓ | bpe | breaks/paradigm_break_sync_file_io_async.py | 40–86 | OOV axis: blocking open() / Path.read_text() / json.loads() tight-loop calls inside async def endpoint bodies. Corpus evidence: 0 instanc… |
| background_tasks_1 | background_tasks | 6.581 | ✓ | bpe | breaks/paradigm_break_concurrent_futures_background.py | 55–87 | concurrent.futures executor.submit() called from each endpoint to dispatch deferred work, discarding the returned Future, instead of Fast… |
| background_tasks_2 | background_tasks | 6.553 | ✓ | bpe | breaks/paradigm_break_multiprocessing_background.py | 45–90 | multiprocessing.Process with daemon=True spawned from each endpoint instead of FastAPI BackgroundTasks.add_task(). multiprocessing.Proces… |
| background_tasks_3 | background_tasks | 6.553 | ✓ | bpe | breaks/paradigm_break_queue_carryover.py | 54–97 | Canonical pattern: docs_src/background_tasks/tutorial001_py310.py lines 12-15 — background_tasks.add_task() from an injected BackgroundTa… |
| background_tasks_4 | background_tasks | 6.581 | ✓ | bpe | breaks/paradigm_break_atexit_background.py | 68–100 | Deferred work accumulated in a module-level deque drained by a repeating threading.Timer, with atexit.register() as a flush safety net — … |
| dependency_injection_1 | dependency_injection | 5.467 | ✓ | bpe | breaks/paradigm_break_manual_generator_drain.py | 51–112 | Endpoints call next(get_db()) manually and manage teardown with try/finally, bypassing FastAPI's Depends() lifecycle. 428 Depends() sites… |
| dependency_injection_2 | dependency_injection | 5.025 | ✓ | call_receiver | breaks/paradigm_break_class_instance_no_depends.py | 73–120 | Service classes instantiated at module level and passed as plain default argument values (service: EmailService = email_service) instead … |
| dependency_injection_3 | dependency_injection | 6.641 | ✓ | import | breaks/paradigm_break_injector_di.py | 26–59 | injector library (injector.Injector, @inject, Module, singleton) for dependency injection instead of FastAPI's Depends() — foreign import… |
| downstream_http_1 | downstream_http | 3.557 | ✓ | call_receiver | breaks/paradigm_break_subtle_manual_status_check.py | 29–65 | FastAPI + httpx corpus uses response.raise_for_status() to propagate downstream errors; manually checking `if response.status_code >= 400… |
| downstream_http_2 | downstream_http | 6.153 | ✓ | bpe | breaks/paradigm_break_sync_requests_in_async.py | 17–73 | synchronous requests.get() / requests.post() with Session() inside async def endpoints blocks the event loop — the correct pattern is htt… |
| downstream_http_3 | downstream_http | 5.877 | ✓ | bpe | breaks/paradigm_break_aiohttp_no_context.py | 24–61 |  |
| exception_handling_1 | exception_handling | 6.852 | ✓ | bpe | breaks/paradigm_break_subtle_wrong_exception.py | 49–89 | decorators, Pydantic models, and Depends() are all idiomatic; the break is confined to the raise statements: ValueError, KeyError, and Ru… |
| exception_handling_2 | exception_handling | 2.980 | ✓ | call_receiver | breaks/paradigm_break_subtle_exception_swallow.py | 51–109 | decorators, Pydantic models, Depends(), HTTPException, and async def are all idiomatic; the break is purely structural: every endpoint bo… |
| exception_handling_3 | exception_handling | 6.781 | ✓ | bpe | breaks/paradigm_break_bare_except.py | 18–71 | every endpoint body wrapped in bare except: (no exception type) that silently returns empty data — broad exception-swallowing with no HTT… |
| exception_handling_4 | exception_handling | 3.028 | ✗ | none | breaks/paradigm_break_json_error_response.py | 51–100 | Endpoints return JSONResponse({"error": str(e)}) directly from except blocks instead of raising HTTPException. Canonical pattern: raise H… |
| exception_handling_5 | exception_handling | 6.852 | ✓ | bpe | breaks/paradigm_break_traceback_in_response.py | 46–100 | Endpoints expose full stack traces via traceback.format_exc() in response bodies on any exception. traceback.format_exc() is absent from … |
| exception_handling_6 | exception_handling | 5.326 | ✓ | bpe | breaks/paradigm_break_flask_errorhandler.py | 46–84 | Uses Flask's @app.errorhandler(...) decorator for exception registration instead of FastAPI's @app.exception_handler(...). Flask's errorh… |
| framework_swap_1 | framework_swap | 5.281 | ✓ | bpe | breaks/paradigm_break_django_cbv.py | 30–78 | FastAPI uses function-based endpoints with typed parameter injection; Django class-based views (View subclasses with def get/post methods… |
| framework_swap_2 | framework_swap | 5.281 | ✓ | bpe | breaks/paradigm_break_aiohttp_handler.py | 29–74 | FastAPI uses declarative method decorators with parameter injection; aiohttp's async def handler(request: web.Request), await request.jso… |
| framework_swap_3 | framework_swap | 5.786 | ✓ | bpe | breaks/paradigm_break_tornado_handler.py | 19–98 | tornado RequestHandler subclass with GET/POST methods using self.get_argument(), self.write(), self.finish(), and IOLoop — classic tornad… |
| routing_1 | routing | 4.309 | ✓ | call_receiver | breaks/paradigm_break_flask_routing.py | 28–67 | FastAPI uses @app.get / @router.post method-specific decorators and Pydantic parameter injection; Flask's @app.route(..., methods=[...]),… |
| routing_2 | routing | 3.321 | ✓ | call_receiver | breaks/paradigm_break_starlette_mount.py | 19–82 | bare Starlette Router with add_route() and url_for() instead of FastAPI's @router.get/@router.post DSL — imperative route registration wi… |
| routing_3 | routing | 2.980 | ✗ | none | breaks/paradigm_break_imperative_route_loop.py | 77–90 |  |
| serialization_1 | serialization | 5.877 | ✓ | bpe | breaks/paradigm_break_manual_json_response.py | 47–95 | explicit json.dumps() with custom default= for datetime/Decimal at every endpoint, wrapped in Response(content=..., media_type='applicati… |
| serialization_2 | serialization | 6.504 | ✓ | bpe | breaks/paradigm_break_manual_dict_response.py | 45–106 | Break axis: explicit field-by-field dict construction with manual float(), bool(), and .isoformat() coercions at every endpoint instead o… |
| serialization_3 | serialization | 7.128 | ✓ | bpe | breaks/paradigm_break_msgpack_response.py | 43–85 | Break axis: msgpack.packb() binary serialization with Response(media_type='application/x-msgpack') at every endpoint instead of JSON/resp… |
| validation_1 | validation | 6.226 | ✓ | bpe | breaks/paradigm_break_manual_validation.py | 29–84 | FastAPI's idiomatic pattern is Pydantic BaseModel parameter injection for automatic validation; accepting `body: dict = Body(...)` and th… |
| validation_2 | validation | 2.980 | ✗ | none | breaks/paradigm_break_voluptuous_validation.py | 30–84 | voluptuous Schema called manually inside endpoints for validation instead of Pydantic BaseModel injection — Schema, Required, Optional, A… |
| validation_3 | validation | 5.347 | ✓ | bpe | breaks/paradigm_break_cerberus_validation.py | 16–69 | cerberus Validator with schema dicts validates plain dict body params instead of Pydantic BaseModel injection — v.errors, validator.valid… |
| validation_4 | validation | 6.504 | ✓ | bpe | breaks/paradigm_break_assert_validation.py | 31–95 | Endpoints accept `body: dict = Body(...)` and validate fields with bare `assert` statements instead of Pydantic BaseModel injection. The … |

</details>

### Missed fixtures (3)

Breaks that didn't trip the scorer (threshold 5.2778):

- **exception_handling_4** (`exception_handling`) — score 3.0281, 2.2497 below threshold, reason: `none`
  - _Rationale:_ Endpoints return JSONResponse({"error": str(e)}) directly from except blocks instead of raising HTTPException. Canonical pattern: raise HTTPException(status_code=..., detail=...) at 78 corpus sites. Single axis: error response constructed and returned at call site vs. raised and dispatched through registered exception handlers. Inline JSONResponse({"error": ...}) at endpoint scope is rare in corpus.
- **validation_2** (`validation`) — score 2.9797, 2.2981 below threshold, reason: `none`
  - _Rationale:_ voluptuous Schema called manually inside endpoints for validation instead of Pydantic BaseModel injection — Schema, Required, Optional, All, Length, Range, Invalid, MultipleInvalid are voluptuous idioms entirely absent from the FastAPI corpus (0 import sites). Single axis: imperative schema-call validation vs declarative Pydantic parameter injection.
- **routing_3** (`routing`) — score 2.9797, 2.2981 below threshold, reason: `none`

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 7.351 | ✓ | bpe | docs_src/generate_clients/tutorial004.js | 0–36 |
| 2 | 7.351 | ✓ | bpe | docs_src/generate_clients/tutorial004.js | 0–29 |
| 3 | 7.351 | ✓ | bpe | docs_src/generate_clients/tutorial004.js | 0–36 |
| 4 | 7.351 | ✓ | bpe | docs_src/generate_clients/tutorial004.js | 0–29 |
| 5 | 7.351 | ✓ | bpe | docs_src/generate_clients/tutorial004.js | 0–36 |

_Threshold is 5.2778; top control scores 7.3506._

### Stage attribution

- `import`: 1 (3.1%)
- `call_receiver`: 6 (18.8%)
- `bpe`: 22 (68.8%)
- `none`: 3 (9.4%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 1/1 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 22/22 | Stage 2 BPE catch — token-level novelty, no foreign import |
| hard | 100.0% | 6/6 | Stage 1.5 call-receiver catch — receiver novelty |
| uncaught | 0.0% | 0/3 | Scorer currently misses — known gap |

## rich (python)

### Summary

- **AUC (catalog vs real-PR controls):** 0.9780
- **Recall (mean across categories):** 95.0%
- **FP rate on real PR hunks:** 0.8% (425/68598)
- **Threshold (mean across seeds):** 4.1642 (CV: 9.5%)
- **Calibration stability:** rel_var=0.037304, jaccard=0.0382
- **Separation gap (min break − max control):** -3.0401 (overlap)
- **Sample sizes:** 16 fixtures · 68598 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 16 | 1.607 | 4.215 | 5.934 | 6.345 | 6.730 | 7.668 |
| Control (real PR) | 68598 | -1.286 | 0.000 | 0.945 | 1.488 | 2.097 | 4.647 |

Threshold **4.1642** — 3/16 breaks fall below it (misses), 33/68598 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "rich — recall by category"
    x-axis ["ansi_raw", "colorama", "curses", "print_manual", "termcolor"]
    y-axis "recall %" 0 --> 110
    bar [100.0, 100.0, 100.0, 75.0, 100.0]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| ansi_raw | 100.0% | 3/3 | 4.068 | 2.530 | 5.457 | ansi_raw_1, ansi_raw_2, ansi_raw_3 |
| colorama | 100.0% | 3/3 | 6.180 | 5.845 | 6.730 | colorama_1, colorama_2, colorama_3 |
| curses | 100.0% | 3/3 | 6.637 | 6.027 | 7.668 | curses_1, curses_2, curses_3 |
| print_manual | 75.0% | 3/4 | 3.795 | 1.607 | 6.730 | print_manual_1, print_manual_2, print_manual_3, dict_render_1 |
| termcolor | 100.0% | 3/3 | 6.261 | 5.902 | 6.730 | termcolor_1, termcolor_2, termcolor_3 |

### Per-fixture results

<details>
<summary>16 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| ansi_raw_1 | ansi_raw | 5.457 | ✓ | bpe | breaks/break_ansi_raw_1.py | 18–43 | Manual ANSI escape code styling (\033[31m etc.) with print() in a rich-looking file |
| ansi_raw_2 | ansi_raw | 4.215 | ✓ | bpe | breaks/break_ansi_raw_2.py | 14–38 | Manual ANSI color code dict and progress bar using sys.stdout.write in a rich-looking file |
| ansi_raw_3 | ansi_raw | 2.530 | ✓ | import | breaks/break_ansi_raw_3.py | 13–21 | blessed.Terminal for dashboard rendering — competing terminal UI library imported inside hunk. Canonical: rich.console.Console at >200 co… |
| colorama_1 | colorama | 6.730 | ✓ | import | breaks/break_colorama_1.py | 19–44 | colorama.init() + Fore/Back/Style paradigm for colored table output in a rich-looking file |
| colorama_2 | colorama | 5.966 | ✓ | import | breaks/break_colorama_2.py | 16–41 | colorama spinner and level-based log rendering with Fore/Back/Style in a rich-looking file |
| colorama_3 | colorama | 5.845 | ✓ | bpe | breaks/break_colorama_3.py | 14–39 | Fore.CYAN + Style.BRIGHT + Back.BLUE + Style.RESET_ALL receiver chain in complex table renderer — colorama imported at file top, outside … |
| curses_1 | curses | 6.217 | ✓ | import | breaks/break_curses_1.py | 18–43 | curses.initscr/start_color/addstr/color_pair dashboard rendering in a rich-looking file |
| curses_2 | curses | 7.668 | ✓ | import | breaks/break_curses_2.py | 16–41 | curses navigable menu with KEY_UP/KEY_DOWN and color_pair highlight in a rich-looking file |
| curses_3 | curses | 6.027 | ✓ | bpe | breaks/break_curses_3.py | 15–30 | curses.textpad.Textbox interactive form — curses and curses.textpad imported at file top, outside hunk. curses.textpad.Textbox, newwin, i… |
| dict_render_1 | print_manual | 1.607 | ✗ | none | breaks/break_dict_render_1.py | 18–23 | plain print() loop in place of rich Table/Panel; no foreign import in hunk; era-5 limit: print/for/list tokens are ubiquitous in Python c… |
| print_manual_1 | print_manual | 2.630 | ✓ | call_receiver | breaks/break_print_manual_1.py | 19–40 | Plain print() with manual ljust/rjust for tabular output in a rich-looking file |
| print_manual_2 | print_manual | 6.730 | ✓ | bpe | breaks/break_print_manual_2.py | 19–42 | Plain print() with manual center/ljust/rjust for key-value report box in a rich-looking file |
| print_manual_3 | print_manual | 4.215 | ✓ | import | breaks/break_print_manual_3.py | 13–21 | tabulate library for table rendering imported inside hunk. Canonical: plain print() with ljust/rjust at break sites; tabulate at 0 rich c… |
| termcolor_1 | termcolor | 6.153 | ✓ | import | breaks/break_termcolor_1.py | 18–43 | termcolor.colored/cprint for banner, diff, and status line rendering in a rich-looking file |
| termcolor_2 | termcolor | 6.730 | ✓ | import | breaks/break_termcolor_2.py | 16–37 | termcolor.colored recursive dict-tree printer in a rich-looking file |
| termcolor_3 | termcolor | 5.902 | ✓ | bpe | breaks/break_termcolor_3.py | 15–36 | termcolor.colored with on_color and attrs (blink) in structured log emitter and progress bar — termcolor imported at file top, outside hu… |

</details>

### Missed fixtures (1)

Breaks that didn't trip the scorer (threshold 4.1642):

- **dict_render_1** (`print_manual`) — score 1.6069, 2.5574 below threshold, reason: `none`
  - _Rationale:_ plain print() loop in place of rich Table/Panel; no foreign import in hunk; era-5 limit: print/for/list tokens are ubiquitous in Python corpus, BPE score stays below threshold; call_receiver has no target

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 4.647 | ✓ | bpe | rich/traceback.py | 341–348 |
| 2 | 4.647 | ✓ | bpe | rich/traceback.py | 336–344 |
| 3 | 4.647 | ✓ | bpe | rich/traceback.py | 339–346 |
| 4 | 4.647 | ✓ | bpe | rich/traceback.py | 335–350 |
| 5 | 4.647 | ✓ | bpe | rich/traceback.py | 335–359 |

_Threshold is 4.1642; top control scores 4.6469._

### Stage attribution

- `import`: 8 (50.0%)
- `call_receiver`: 1 (6.2%)
- `bpe`: 6 (37.5%)
- `none`: 1 (6.2%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 8/8 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 5/5 | Stage 2 BPE catch — token-level novelty, no foreign import |
| hard | 100.0% | 2/2 | Stage 1.5 call-receiver catch — receiver novelty |
| uncaught | 0.0% | 0/1 | Scorer currently misses — known gap |

## faker (python)

### Summary

- **AUC (catalog vs real-PR controls):** 0.9537
- **Recall (mean across categories):** 95.0%
- **FP rate on real PR hunks:** 1.2% (393/75996)
- **Threshold (mean across seeds):** 5.2111 (CV: 3.7%)
- **Calibration stability:** rel_var=0.006984, jaccard=0.0665
- **Separation gap (min break − max control):** -4.9459 (overlap)
- **Sample sizes:** 16 fixtures · 75996 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 16 | 1.843 | 4.588 | 5.757 | 6.936 | 7.291 | 7.380 |
| Control (real PR) | 75996 | -1.321 | 0.000 | 0.000 | 1.518 | 2.300 | 6.789 |

Threshold **5.2111** — 6/16 breaks fall below it (misses), 163/75996 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "faker — recall by category"
    x-axis ["mimesis_alt", "numpy_random", "requests_source", "sqlalchemy_sink", "threading_provider"]
    y-axis "recall %" 0 --> 110
    bar [100.0, 75.0, 100.0, 100.0, 100.0]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| mimesis_alt | 100.0% | 3/3 | 3.201 | 1.843 | 5.729 | mimesis_alt_1, mimesis_alt_2, mimesis_alt_3 |
| numpy_random | 75.0% | 3/4 | 4.874 | 3.813 | 5.786 | numpy_random_1, numpy_random_2, numpy_random_3, synthetic_formula_1 |
| requests_source | 100.0% | 3/3 | 7.073 | 6.596 | 7.380 | requests_source_1, requests_source_2, requests_source_3 |
| sqlalchemy_sink | 100.0% | 3/3 | 5.545 | 4.571 | 6.957 | sqlalchemy_sink_1, sqlalchemy_sink_2, sqlalchemy_sink_3 |
| threading_provider | 100.0% | 3/3 | 6.776 | 6.059 | 7.339 | threading_provider_1, threading_provider_2, threading_provider_3 |

### Per-fixture results

<details>
<summary>16 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| mimesis_alt_1 | mimesis_alt | 1.843 | ✓ | import | breaks/break_mimesis_alt_1.py | 19–46 | mimesis Person/Address/Finance with Gender enum — competing fake-data library paradigm in a faker-looking file |
| mimesis_alt_2 | mimesis_alt | 5.729 | ✓ | bpe | breaks/break_mimesis_alt_2.py | 16–33 | polyfactory.ModelFactory with Pydantic BaseModel for type-safe factory generation — import outside hunk (file top), break is ModelFactory… |
| mimesis_alt_3 | mimesis_alt | 2.032 | ✓ | call_receiver | breaks/break_mimesis_alt_3.py | 14–26 | Faker() calling rare financial provider methods (aba, bban, iban, swift, cryptocurrency_code) — no foreign import, BPE familiar, but thes… |
| numpy_random_1 | numpy_random | 4.593 | ✓ | import | breaks/break_numpy_random_1.py | 19–46 | numpy.random Generator/default_rng for name and age generation instead of faker internals in a faker-looking file |
| numpy_random_2 | numpy_random | 5.786 | ✓ | bpe | breaks/break_numpy_random_2.py | 15–28 | scipy.stats distributions (truncnorm, pareto) for realistic data generation — scipy.stats imported outside hunk. Canonical: numpy.random … |
| numpy_random_3 | numpy_random | 5.304 | ✓ | call_receiver | breaks/break_numpy_random_3.py | 15–32 | numpy rng.choice and rng.shuffle called on a default_rng instance — receiver methods foreign to faker corpus. rng.integers is known but r… |
| synthetic_formula_1 | numpy_random | 3.813 | ✗ | none | breaks/break_synthetic_formula_1.py | 18–29 | string-formula synthesis (f-string concat) instead of faker.email()/faker.name(); no foreign import in hunk; era-6 limit: call_receiver h… |
| requests_source_1 | requests_source | 7.380 | ✓ | import | breaks/break_requests_source_1.py | 17–47 | requests.get() fetching real user data from HTTP APIs instead of local fake generation in a faker-looking file |
| requests_source_2 | requests_source | 7.242 | ✓ | bpe | breaks/break_requests_source_2.py | 15–25 | httpx.Client for HTTP source — httpx imported outside hunk. Canonical: requests.get at faker corpus sites; httpx.Client at 0 sites. |
| requests_source_3 | requests_source | 6.596 | ✓ | bpe | breaks/break_requests_source_3.py | 16–31 | aiohttp.ClientSession for async HTTP source — session.get/raise_for_status/json are foreign receiver patterns. aiohttp.ClientSession at 0… |
| sqlalchemy_sink_1 | sqlalchemy_sink | 6.957 | ✓ | import | breaks/break_sqlalchemy_sink_1.py | 17–49 | SQLAlchemy ORM model + session.add/commit to persist fakes — DB-sink paradigm in a faker-looking file |
| sqlalchemy_sink_2 | sqlalchemy_sink | 4.571 | ✓ | call_receiver | breaks/break_sqlalchemy_sink_2.py | 16–28 | csv.DictWriter + io.StringIO export sink — csv and io imported outside hunk. Canonical: SQLAlchemy session at break sites; csv.DictWriter… |
| sqlalchemy_sink_3 | sqlalchemy_sink | 5.109 | ✓ | call_receiver | breaks/break_sqlalchemy_sink_3.py | 15–30 | SQLAlchemy Session.bulk_save_objects + Session.flush + Session.expire_all — these receiver methods on a Session object are foreign to fak… |
| threading_provider_1 | threading_provider | 7.339 | ✓ | import | breaks/break_threading_provider_1.py | 19–53 | threading.Thread + Lock + Event parallel fake-data generator class in a faker-looking file |
| threading_provider_2 | threading_provider | 6.059 | ✓ | bpe | breaks/break_threading_provider_2.py | 15–22 | concurrent.futures.ProcessPoolExecutor for parallel fake generation — import outside hunk. Canonical: threading.Thread at faker corpus si… |
| threading_provider_3 | threading_provider | 6.929 | ✓ | bpe | breaks/break_threading_provider_3.py | 15–29 | multiprocessing.Queue.put/get_nowait and Process.start/join — receiver methods on Queue and Process objects are foreign to faker corpus. … |

</details>

### Missed fixtures (1)

Breaks that didn't trip the scorer (threshold 5.2111):

- **synthetic_formula_1** (`numpy_random`) — score 3.8133, 1.3977 below threshold, reason: `none`
  - _Rationale:_ string-formula synthesis (f-string concat) instead of faker.email()/faker.name(); no foreign import in hunk; era-6 limit: call_receiver has no target, BPE tokens (f-string, str, int format) are nominal in the Python corpus

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 6.789 | ✓ | bpe | faker/providers/address/vi_VN/__init__.py | 0–308 |
| 2 | 6.789 | ✓ | bpe | faker/providers/address/en_MS/__init__.py | 0–490 |
| 3 | 6.789 | ✓ | bpe | faker/providers/address/en_US/__init__.py | 528–553 |
| 4 | 6.789 | ✓ | bpe | faker/providers/address/vi_VN/__init__.py | 0–308 |
| 5 | 6.789 | ✓ | bpe | faker/providers/address/en_MS/__init__.py | 0–490 |

_Threshold is 5.2111; top control scores 6.7890._

### Stage attribution

- `import`: 5 (31.2%)
- `call_receiver`: 4 (25.0%)
- `bpe`: 6 (37.5%)
- `none`: 1 (6.2%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 5/5 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 5/5 | Stage 2 BPE catch — token-level novelty, no foreign import |
| hard | 100.0% | 5/5 | Stage 1.5 call-receiver catch — receiver novelty |
| uncaught | 0.0% | 0/1 | Scorer currently misses — known gap |

## hono (typescript)

### Summary

- **AUC (catalog vs real-PR controls):** 0.8312
- **Recall (mean across categories):** 78.3%
- **FP rate on real PR hunks:** 0.5% (159/54717)
- **Threshold (mean across seeds):** 4.2768 (CV: 3.0%)
- **Calibration stability:** rel_var=0.003804, jaccard=0.0161
- **Separation gap (min break − max control):** -7.4712 (overlap)
- **Sample sizes:** 17 fixtures · 54717 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 17 | -1.736 | 1.484 | 4.713 | 5.744 | 6.325 | 6.406 |
| Control (real PR) | 54717 | -7.061 | 0.000 | 0.000 | 1.038 | 1.782 | 5.735 |

Threshold **4.2768** — 7/17 breaks fall below it (misses), 157/54717 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "hono — recall by category"
    x-axis ["async_blocking", "framework_swap", "middleware", "routing", "validation"]
    y-axis "recall %" 0 --> 110
    bar [100.0, 75.0, 50.0, 100.0, 66.7]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| async_blocking | 100.0% | 3/3 | 5.988 | 5.744 | 6.406 | hono_async_blocking_1, hono_async_blocking_2, hono_async_blocking_3 |
| framework_swap | 75.0% | 3/4 | 4.053 | 1.484 | 6.373 | hono_framework_swap_1, hono_framework_swap_2, hono_framework_swap_3, hono_framework_swap_4 |
| middleware | 50.0% | 2/4 | 2.098 | -1.736 | 5.304 | hono_middleware_1, hono_middleware_2, hono_middleware_3, hono_middleware_4 |
| routing | 100.0% | 3/3 | 2.238 | 0.819 | 4.713 | hono_routing_1, hono_routing_2, hono_routing_3 |
| validation | 66.7% | 2/3 | 4.357 | 2.231 | 6.294 | hono_validation_1, hono_validation_2, hono_validation_3 |

### Per-fixture results

<details>
<summary>17 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| hono_async_blocking_1 | async_blocking | 5.813 | ✓ | bpe | breaks/break_async_blocking_1.ts | 9–12 | fs.readFileSync blocking I/O inside a Hono request handler |
| hono_async_blocking_2 | async_blocking | 6.406 | ✓ | bpe | breaks/break_async_blocking_2.ts | 8–14 | fs.readFileSync and fs.statSync inside a streaming endpoint |
| hono_async_blocking_3 | async_blocking | 5.744 | ✓ | bpe | breaks/break_async_blocking_3.ts | 7–10 | child_process.execSync blocks the event loop per request |
| hono_framework_swap_1 | framework_swap | 1.484 | ✗ | none | breaks/break_framework_swap_1.ts | 5–12 | Express Router() idiom with (req, res) handlers mounted into a Hono app |
| hono_framework_swap_2 | framework_swap | 5.211 | ✓ | bpe | breaks/break_framework_swap_2.ts | 6–10 | Express (req: Request, res: Response) callback signature instead of Hono's Context |
| hono_framework_swap_3 | framework_swap | 6.373 | ✓ | bpe | breaks/break_framework_swap_3.ts | 7–10 | Express body-parser + cors middleware chain wired via app.use |
| hono_framework_swap_4 | framework_swap | 3.145 | ✓ | import | breaks/break_framework_swap_4.ts | 1–15 | import express at line 1 — standalone Express Router module in a Hono project; 'express' appears 0 times in non-break hono corpus files v… |
| hono_middleware_1 | middleware | 5.304 | ✓ | bpe | breaks/break_middleware_1.ts | 5–12 | Express (req, res, next) middleware signature in app.use |
| hono_middleware_2 | middleware | 0.110 | ✗ | none | breaks/break_middleware_2.ts | 9–14 | Express 4-arg (err, req, res, next) error-handler signature |
| hono_middleware_3 | middleware | -1.736 | ✗ | none | breaks/break_middleware_3.ts | 10–11 | Calling next() synchronously instead of Hono's await next() |
| hono_middleware_4 | middleware | 4.713 | ✓ | import | breaks/break_middleware_4.ts | 1–21 | import Koa + @koa/router at line 1 — Koa ctx.body middleware pattern in a Hono project; Koa absent from all hono corpus PRs (0 grep hits) |
| hono_routing_1 | routing | 4.713 | ✓ | bpe | breaks/break_routing_1.ts | 6–12 | express.Router() mounted under app.use('/api', router) |
| hono_routing_2 | routing | 1.183 | ✓ | call_receiver | breaks/break_routing_2.ts | 6–14 | Router().route(path).get().post().delete() chain composition — era-8: <call>.route and <call>.get now caught by call-receiver |
| hono_routing_3 | routing | 0.819 | ✓ | call_receiver | breaks/break_routing_3.ts | 7–14 | Express-shaped app.all('*', (req, res)) wildcard catch-all |
| hono_validation_1 | validation | 6.294 | ✓ | bpe | breaks/break_validation_1.ts | 7–13 | Hand-rolled email/password guards instead of a zod/valibot schema |
| hono_validation_2 | validation | 2.231 | ✗ | none | breaks/break_validation_2.ts | 7–13 | Manual typeof + length guards where a zod schema would fit |
| hono_validation_3 | validation | 4.547 | ✓ | bpe | breaks/break_validation_3.ts | 10–16 | Regex-literal phone/zip validation instead of a validator library |

</details>

### Missed fixtures (4)

Breaks that didn't trip the scorer (threshold 4.2768):

- **hono_validation_2** (`validation`) — score 2.2307, 2.0460 below threshold, reason: `none`
  - _Rationale:_ Manual typeof + length guards where a zod schema would fit
- **hono_framework_swap_1** (`framework_swap`) — score 1.4841, 2.7926 below threshold, reason: `none`
  - _Rationale:_ Express Router() idiom with (req, res) handlers mounted into a Hono app
- **hono_middleware_2** (`middleware`) — score 0.1104, 4.1663 below threshold, reason: `none`
  - _Rationale:_ Express 4-arg (err, req, res, next) error-handler signature
- **hono_middleware_3** (`middleware`) — score -1.7359, 6.0126 below threshold, reason: `none`
  - _Rationale:_ Calling next() synchronously instead of Hono's await next()

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 5.735 | ✓ | bpe | src/helper/css/common.ts | 0–243 |
| 2 | 5.729 | ✓ | bpe | src/helper/css/common.ts | 0–243 |
| 3 | 5.724 | ✓ | bpe | src/helper/css/common.ts | 0–243 |
| 4 | 5.718 | ✓ | bpe | src/helper/css/common.ts | 0–243 |
| 5 | 5.716 | ✓ | bpe | src/helper/css/common.ts | 0–243 |

_Threshold is 4.2768; top control scores 5.7353._

### Stage attribution

- `import`: 2 (11.8%)
- `call_receiver`: 2 (11.8%)
- `bpe`: 9 (52.9%)
- `none`: 4 (23.5%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 2/2 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 9/9 | Stage 2 BPE catch — token-level novelty, no foreign import |
| hard | 100.0% | 1/1 | Stage 1.5 call-receiver catch — receiver novelty |
| uncaught | 20.0% | 1/5 | Scorer currently misses — known gap |

## ink (typescript)

### Summary

- **AUC (catalog vs real-PR controls):** 0.9899
- **Recall (mean across categories):** 93.3%
- **FP rate on real PR hunks:** 0.4% (32/16678)
- **Threshold (mean across seeds):** 4.8256 (CV: 6.9%)
- **Calibration stability:** rel_var=0.023265, jaccard=0.0943
- **Separation gap (min break − max control):** -4.6327 (overlap)
- **Sample sizes:** 17 fixtures · 16678 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 17 | 2.105 | 5.428 | 6.310 | 8.833 | 8.833 | 8.833 |
| Control (real PR) | 16678 | -3.575 | 0.000 | 0.000 | 0.359 | 1.530 | 6.738 |

Threshold **4.8256** — 2/17 breaks fall below it (misses), 29/16678 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "ink — recall by category"
    x-axis ["class_components", "dom_access", "error_flip", "jquery", "lifecycle"]
    y-axis "recall %" 0 --> 110
    bar [100.0, 66.7, 100.0, 100.0, 100.0]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| class_components | 100.0% | 4/4 | 8.833 | 8.833 | 8.833 | ink_class_components_1, ink_class_components_2, ink_class_components_3, ink_class_components_4 |
| dom_access | 66.7% | 2/3 | 4.210 | 2.105 | 6.310 | ink_dom_access_1, ink_dom_access_2, ink_dom_access_3 |
| error_flip | 100.0% | 3/3 | 8.833 | 8.833 | 8.833 | ink_error_flip_1, ink_error_flip_2, ink_error_flip_3 |
| jquery | 100.0% | 4/4 | 6.160 | 5.428 | 8.357 | ink_jquery_1, ink_jquery_2, ink_jquery_3, ink_jquery_4 |
| lifecycle | 100.0% | 3/3 | 5.428 | 5.428 | 5.428 | ink_lifecycle_1, ink_lifecycle_2, ink_lifecycle_3 |

### Per-fixture results

<details>
<summary>17 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| ink_class_components_1 | class_components | 8.833 | ✓ | bpe | breaks/break_class_components_1.tsx | 5–15 | ES6 class component in a hooks-only ink codebase |
| ink_class_components_2 | class_components | 8.833 | ✓ | bpe | breaks/break_class_components_2.tsx | 8–18 | `extends Component` class with this.props / this.state in an ink file |
| ink_class_components_3 | class_components | 8.833 | ✓ | bpe | breaks/break_class_components_3.tsx | 8–21 | Class component with explicit constructor(props) in an ink file |
| ink_class_components_4 | class_components | 8.833 | ✓ | bpe | breaks/break_class_components_4.tsx | 1–26 | import React, { Component } from 'react' at line 1 — class-based component in a hooks-only ink codebase; Component import absent from all… |
| ink_dom_access_1 | dom_access | 2.105 | ✓ | call_receiver | breaks/break_dom_access_1.tsx | 6–7 | document.getElementById + window.addEventListener in a terminal UI |
| ink_dom_access_2 | dom_access | 4.215 | ✗ | none | breaks/break_dom_access_2.tsx | 8–8 | window.location.href navigation inside a useInput handler |
| ink_dom_access_3 | dom_access | 6.310 | ✓ | bpe | breaks/break_dom_access_3.tsx | 9–10 | localStorage.getItem in an ink useEffect |
| ink_error_flip_1 | error_flip | 8.833 | ✓ | bpe | breaks/break_error_flip_1.tsx | 8–11 | throw new Error inside render via IIFE |
| ink_error_flip_2 | error_flip | 8.833 | ✓ | bpe | breaks/break_error_flip_2.tsx | 10–13 | throw inside a map callback in the render return |
| ink_error_flip_3 | error_flip | 8.833 | ✓ | bpe | breaks/break_error_flip_3.tsx | 8–15 | try/finally with throw but no catch inside render |
| ink_jquery_1 | jquery | 5.428 | ✓ | bpe | breaks/break_jquery_1.tsx | 7–9 | jQuery $('.item').on('click', ...) inside a functional component |
| ink_jquery_2 | jquery | 5.428 | ✓ | bpe | breaks/break_jquery_2.tsx | 8–12 | jQuery .show()/.hide() DOM manipulation from a hook |
| ink_jquery_3 | jquery | 8.357 | ✓ | bpe | breaks/break_jquery_3.tsx | 10–14 | $.ajax inside a useEffect where fetch/undici would be idiomatic |
| ink_jquery_4 | jquery | 5.428 | ✓ | import | breaks/break_jquery_4.tsx | 1–12 | import $ from 'jquery' at line 1 in an ink terminal component; jquery absent from all 3 ink corpus PRs (0 grep hits); import scorer fires… |
| ink_lifecycle_1 | lifecycle | 5.428 | ✓ | bpe | breaks/break_lifecycle_1.tsx | 10–12 | componentDidMount + this.setState instead of useEffect |
| ink_lifecycle_2 | lifecycle | 5.428 | ✓ | bpe | breaks/break_lifecycle_2.tsx | 11–15 | Legacy componentWillReceiveProps lifecycle |
| ink_lifecycle_3 | lifecycle | 5.428 | ✓ | bpe | breaks/break_lifecycle_3.tsx | 11–16 | componentWillUnmount cleanup instead of useEffect return fn |

</details>

### Missed fixtures (1)

Breaks that didn't trip the scorer (threshold 4.8256):

- **ink_dom_access_2** (`dom_access`) — score 4.2150, 0.6107 below threshold, reason: `none`
  - _Rationale:_ window.location.href navigation inside a useInput handler

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 6.738 | ✓ | bpe | xo.config.ts | 38–49 |
| 2 | 6.738 | ✓ | bpe | xo.config.ts | 14–20 |
| 3 | 6.738 | ✓ | bpe | xo.config.ts | 38–49 |
| 4 | 6.738 | ✓ | bpe | xo.config.ts | 14–20 |
| 5 | 6.738 | ✓ | bpe | xo.config.ts | 38–49 |

_Threshold is 4.8256; top control scores 6.7376._

### Stage attribution

- `import`: 1 (5.9%)
- `call_receiver`: 1 (5.9%)
- `bpe`: 14 (82.4%)
- `none`: 1 (5.9%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 2/2 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 13/13 | Stage 2 BPE catch — token-level novelty, no foreign import |
| uncaught | 50.0% | 1/2 | Scorer currently misses — known gap |

## faker-js (typescript)

### Summary

- **AUC (catalog vs real-PR controls):** 0.9463
- **Recall (mean across categories):** 53.3%
- **FP rate on real PR hunks:** 1.0% (428/255760)
- **Threshold (mean across seeds):** 4.7729 (CV: 3.7%)
- **Calibration stability:** rel_var=0.006461, jaccard=0.1422
- **Separation gap (min break − max control):** -7.0661 (overlap)
- **Sample sizes:** 17 fixtures · 255760 real-PR controls

### Score distribution

| | n | min | p25 | median | p75 | p90 | max |
|:---|---:|---:|---:|---:|---:|---:|---:|
| Break (catalog) | 17 | 0.520 | 2.841 | 3.767 | 4.593 | 5.479 | 6.683 |
| Control (real PR) | 255760 | -9.300 | 0.000 | 0.000 | 0.000 | 0.000 | 7.586 |

Threshold **4.7729** — 14/17 breaks fall below it (misses), 387/255760 controls fall at/above (false positives).

### Recall by category

```mermaid
xychart-beta
    title "faker-js — recall by category"
    x-axis ["error_flip", "foreign_rng", "http_sink", "runtime_fetch", "threading"]
    y-axis "recall %" 0 --> 110
    bar [33.3, 33.3, 75.0, 25.0, 100.0]
```

### Per-category detail

| Category | Recall | Hits | Mean break score | Min | Max | Fixtures |
|:---|---:|---:|---:|---:|---:|:---|
| error_flip | 33.3% | 1/3 | 4.565 | 4.053 | 5.095 | faker_js_error_flip_1, faker_js_error_flip_2, faker_js_error_flip_3 |
| foreign_rng | 33.3% | 1/3 | 1.412 | 0.520 | 3.195 | faker_js_foreign_rng_1, faker_js_foreign_rng_2, faker_js_foreign_rng_3 |
| http_sink | 75.0% | 3/4 | 3.414 | 2.841 | 3.767 | faker_js_http_sink_1, faker_js_http_sink_2, faker_js_http_sink_3, faker_js_http_sink_4 |
| runtime_fetch | 25.0% | 1/4 | 3.140 | 1.971 | 4.593 | faker_js_runtime_fetch_1, faker_js_runtime_fetch_2, faker_js_runtime_fetch_3, faker_js_runtime_fetch_4 |
| threading | 100.0% | 3/3 | 5.812 | 4.700 | 6.683 | faker_js_threading_1, faker_js_threading_2, faker_js_threading_3 |

### Per-fixture results

<details>
<summary>17 fixtures (click to expand)</summary>

| ID | Category | BPE | Flagged | Reason | File | Lines | Rationale |
|:---|:---|---:|:---:|:---|:---|:---|:---|
| faker_js_error_flip_1 | error_flip | 5.095 | ✓ | bpe | breaks/break_error_flip_1.ts | 3–11 | Provider throws mid-generation instead of returning a fake value |
| faker_js_error_flip_2 | error_flip | 4.546 | ✗ | none | breaks/break_error_flip_2.ts | 3–8 | Locale data accessor throws on missing entries instead of falling back |
| faker_js_error_flip_3 | error_flip | 4.053 | ✗ | none | breaks/break_error_flip_3.ts | 3–9 | seed() override throws, breaking the determinism contract |
| faker_js_foreign_rng_1 | foreign_rng | 0.520 | ✗ | none | breaks/break_foreign_rng_1.ts | 3–8 | Math.random instead of faker-js internal RNG |
| faker_js_foreign_rng_2 | foreign_rng | 3.195 | ✓ | call_receiver | breaks/break_foreign_rng_2.ts | 4–9 | crypto.randomBytes inside a word generator bypasses the seeded RNG |
| faker_js_foreign_rng_3 | foreign_rng | 0.520 | ✗ | none | breaks/break_foreign_rng_3.ts | 5–9 | Math.random index pick inside a person-name provider |
| faker_js_http_sink_1 | http_sink | 3.281 | ✓ | call_receiver | breaks/break_http_sink_1.ts | 4–13 | Provider method posts telemetry to an HTTP endpoint via axios |
| faker_js_http_sink_2 | http_sink | 3.767 | ✗ | none | breaks/break_http_sink_2.ts | 3–11 | Generator pipes every generated value to a remote log sink via fetch |
| faker_js_http_sink_3 | http_sink | 2.841 | ✓ | call_receiver | breaks/break_http_sink_3.ts | 3–9 | navigator.sendBeacon reporting inside a faker provider |
| faker_js_http_sink_4 | http_sink | 3.767 | ✓ | import | breaks/break_http_sink_4.ts | 1–8 | import axios at line 1 inside a faker-js provider utility; axios absent from all 3 faker-js corpus PRs (0 grep hits in non-break files); … |
| faker_js_runtime_fetch_1 | runtime_fetch | 3.548 | ✗ | none | breaks/break_runtime_fetch_1.ts | 3–8 | Runtime fetch from a locale data file |
| faker_js_runtime_fetch_2 | runtime_fetch | 2.449 | ✗ | none | breaks/break_runtime_fetch_2.ts | 3–11 | ImageProvider.url() issues a live network fetch |
| faker_js_runtime_fetch_3 | runtime_fetch | 1.971 | ✗ | none | breaks/break_runtime_fetch_3.ts | 3–12 | CompanyProvider.name() fetches from a remote name-service |
| faker_js_runtime_fetch_4 | runtime_fetch | 4.593 | ✓ | import | breaks/break_runtime_fetch_4.ts | 1–8 | import fetch from 'node-fetch' at line 1 inside a faker-js locale utility; node-fetch absent from all 3 faker-js corpus PRs; import score… |
| faker_js_threading_1 | threading | 4.700 | ✓ | call_receiver | breaks/break_threading_1.ts | 3–9 | Spawning a Worker from a locale module |
| faker_js_threading_2 | threading | 6.054 | ✓ | bpe | breaks/break_threading_2.ts | 1–10 | Module-level Worker pool inside a pure data file |
| faker_js_threading_3 | threading | 6.683 | ✓ | bpe | breaks/break_threading_3.ts | 3–10 | Generator implementation delegates to a Worker via postMessage |

</details>

### Missed fixtures (8)

Breaks that didn't trip the scorer (threshold 4.7729):

- **faker_js_error_flip_2** (`error_flip`) — score 4.5463, 0.2267 below threshold, reason: `none`
  - _Rationale:_ Locale data accessor throws on missing entries instead of falling back
- **faker_js_error_flip_3** (`error_flip`) — score 4.0527, 0.7202 below threshold, reason: `none`
  - _Rationale:_ seed() override throws, breaking the determinism contract
- **faker_js_http_sink_2** (`http_sink`) — score 3.7667, 1.0062 below threshold, reason: `none`
  - _Rationale:_ Generator pipes every generated value to a remote log sink via fetch
- **faker_js_runtime_fetch_1** (`runtime_fetch`) — score 3.5478, 1.2251 below threshold, reason: `none`
  - _Rationale:_ Runtime fetch from a locale data file
- **faker_js_runtime_fetch_2** (`runtime_fetch`) — score 2.4487, 2.3242 below threshold, reason: `none`
  - _Rationale:_ ImageProvider.url() issues a live network fetch
- **faker_js_runtime_fetch_3** (`runtime_fetch`) — score 1.9712, 2.8017 below threshold, reason: `none`
  - _Rationale:_ CompanyProvider.name() fetches from a remote name-service
- **faker_js_foreign_rng_1** (`foreign_rng`) — score 0.5200, 4.2529 below threshold, reason: `none`
  - _Rationale:_ Math.random instead of faker-js internal RNG
- **faker_js_foreign_rng_3** (`foreign_rng`) — score 0.5200, 4.2529 below threshold, reason: `none`
  - _Rationale:_ Math.random index pick inside a person-name provider

### Top 5 real-PR controls (closest to false positives)

| Rank | BPE | Flagged | Reason | File | Lines |
|---:|---:|:---:|:---|:---|:---|
| 1 | 7.586 | ✓ | bpe | src/locales/en_US/location/postcode_by_state.ts | 0–54 |
| 2 | 7.586 | ✓ | bpe | src/locales/en_US/location/postcode_by_state.ts | 0–54 |
| 3 | 7.586 | ✓ | bpe | src/locales/en_US/location/postcode_by_state.ts | 0–54 |
| 4 | 7.586 | ✓ | bpe | src/locales/en_US/location/postcode_by_state.ts | 0–54 |
| 5 | 7.586 | ✓ | bpe | src/locales/en_US/location/postcode_by_state.ts | 0–54 |

_Threshold is 4.7729; top control scores 7.5861._

### Stage attribution

- `import`: 2 (11.8%)
- `call_receiver`: 4 (23.5%)
- `bpe`: 3 (17.6%)
- `none`: 8 (47.1%)

### Recall by difficulty

| Difficulty | Recall | Hits | Definition |
|:---|---:|---:|:---|
| easy | 100.0% | 2/2 | Stage 1 import catch — foreign module in hunk |
| medium | 100.0% | 3/3 | Stage 2 BPE catch — token-level novelty, no foreign import |
| hard | 100.0% | 2/2 | Stage 1.5 call-receiver catch — receiver novelty |
| uncaught | 20.0% | 2/10 | Scorer currently misses — known gap |
