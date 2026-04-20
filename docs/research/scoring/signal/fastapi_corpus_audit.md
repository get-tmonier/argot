# FastAPI Corpus Audit — v1 Fixtures + Vocabulary Analysis

**Date:** 2026-04-20
**Corpus:** `engine/argot/acceptance/catalog/fastapi/corpus.jsonl` (2000 records)
**Corpus vocabulary size:** 5395 unique tokens
**Schema:** `hunk_tokens` is a list of `{"text": str, "node_type": str, "start_line": int, "end_line": int}` dicts

---

## 1. Corpus Composition

### 1.1 File Path Distribution (top 30)

| Rank | File Path | Record Count |
|------|-----------|-------------|
| 1 | `fastapi/routing.py` | 85 |
| 2 | `fastapi/param_functions.py` | 76 |
| 3 | `fastapi/applications.py` | 75 |
| 4 | `fastapi/dependencies/utils.py` | 65 |
| 5 | `fastapi/params.py` | 41 |
| 6 | `fastapi/openapi/utils.py` | 29 |
| 7 | `fastapi/__init__.py` | 28 |
| 8 | `fastapi/security/oauth2.py` | 17 |
| 9 | `tests/test_schema_extra_examples.py` | 16 |
| 10 | `fastapi/openapi/models.py` | 14 |
| 11 | `tests/test_request_params/test_header/test_list.py` | 14 |
| 12 | `.github/actions/people/app/main.py` | 14 |
| 13 | `tests/test_path.py` | 12 |
| 14 | `fastapi/utils.py` | 12 |
| 15 | `fastapi/_compat/v2.py` | 12 |
| 16 | `fastapi/security/http.py` | 11 |
| 17 | `fastapi/openapi/docs.py` | 11 |
| 18 | `tests/test_request_params/test_body/test_optional_str.py` | 11 |
| 19 | `tests/test_response_model_as_return_annotation.py` | 10 |
| 20 | `tests/test_request_params/test_query/test_optional_list.py` | 10 |
| 21 | `scripts/docs.py` | 10 |
| 22 | `fastapi/encoders.py` | 10 |
| 23 | `tests/test_request_params/test_file/test_list.py` | 9 |
| 24 | `tests/test_request_params/test_query/test_list.py` | 9 |
| 25 | `scripts/translate.py` | 9 |
| 26 | `tests/test_generate_unique_id_function.py` | 8 |
| 27 | `tests/test_request_params/test_header/test_required_str.py` | 8 |
| 28 | `tests/test_request_params/test_form/test_list.py` | 8 |
| 29 | `tests/test_request_params/test_body/test_list.py` | 8 |
| 30 | `tests/test_request_params/test_header/test_optional_list.py` | 8 |

**Observation:** The corpus is dominated by FastAPI's own internal source files (`routing.py`, `param_functions.py`, `applications.py`) and test infrastructure. Very few application-level endpoint files appear. This means application-level idioms like `APIRouter`, `Depends()`, `BaseModel` parameters, and `async def` endpoint bodies are underrepresented relative to framework internals (OpenAPI schema generation, dependency resolution machinery). Break fixtures that target application-level patterns will find a weaker vocabulary signal in the corpus.

---

### 1.2 Top-50 Corpus Tokens

| Rank | Token | Count |
|------|-------|-------|
| 1 | `"` | 61625 |
| 2 | `:` | 22881 |
| 3 | `,` | 21308 |
| 4 | `{` | 9546 |
| 5 | `}` | 9390 |
| 6 | `(` | 7985 |
| 7 | `)` | 7924 |
| 8 | `.` | 7118 |
| 9 | `=` | 6019 |
| 10 | `]` | 4374 |
| 11 | `[` | 4364 |
| 12 | `type` | 2731 |
| 13 | `response` | 2429 |
| 14 | `import` | 2245 |
| 15 | `title` | 2101 |
| 16 | `from` | 1948 |
| 17 | `None` | 1777 |
| 18 | `assert` | 1525 |
| 19 | `def` | 1497 |
| 20 | `client` | 1441 |
| 21 | `==` | 1418 |
| 22 | `str` | 1211 |
| 23 | `app` | 1120 |
| 24 | `@` | 1047 |
| 25 | `get` | 961 |
| 26 | `name` | 953 |
| 27 | `string` | 930 |
| 28 | `pytest` | 872 |
| 29 | `description` | 853 |
| 30 | `schema` | 850 |
| 31 | `TestClient` | 824 |
| 32 | `200` | 771 |
| 33 | `fastapi` | 754 |
| 34 | `json` | 747 |
| 35 | `status_code` | 719 |
| 36 | `FastAPI` | 698 |
| 37 | `return` | 698 |
| 38 | `"""` | 652 |
| 39 | `content` | 650 |
| 40 | `Annotated` | 646 |
| 41 | `application/json` | 568 |
| 42 | `required` | 561 |
| 43 | `path` | 546 |
| 44 | `$ref` | 522 |
| 45 | `\|` | 516 |
| 46 | `loc` | 513 |
| 47 | `Optional` | 512 |
| 48 | `msg` | 511 |
| 49 | `in` | 498 |
| 50 | `True` | 463 |

**Observation:** Tokens ranked 12–50 are heavily OpenAPI-schema vocabulary (`type`, `title`, `description`, `schema`, `$ref`, `required`, `string`, `application/json`) and test harness tokens (`assert`, `pytest`, `TestClient`, `client`, `200`). Application-level endpoint keywords (`Depends`, `HTTPException`, `router`, `async`, `await`, `BaseModel`) appear further down in the distribution, confirming the corpus skews toward framework internals and tests.

---

## 2. V1 Fixture OOV Rates

OOV = fixture hunk tokens not found in the corpus vocabulary set. Tokenization: `re.findall(r"[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[^\s\w]", hunk_text)`.

| Fixture | Type | Total Tokens | OOV Count | OOV Rate | Flag |
|---------|------|-------------|-----------|----------|------|
| `paradigm_break_flask_routing` | break | 356 | 31 | **8.7%** | — |
| `paradigm_break_django_cbv` | break | 525 | 59 | **11.2%** | — |
| `paradigm_break_aiohttp_handler` | break | 421 | 55 | **13.1%** | — |
| `paradigm_break_manual_validation` | break | 685 | 30 | **4.4%** | WEAK (OOV < 5%) |
| `paradigm_break_raw_response` | break | 440 | 16 | **3.6%** | WEAK (OOV < 5%) |
| `paradigm_break_subtle_wrong_exception` | break | 334 | 24 | **7.2%** | — |
| `paradigm_break_subtle_manual_status_check` | break | 456 | 16 | **3.5%** | WEAK (OOV < 5%) |
| `paradigm_break_subtle_sync_endpoint` | break | 515 | 74 | **14.4%** | — |
| `paradigm_break_subtle_exception_swallow` | break | 435 | 22 | **5.1%** | — |
| `control_router_endpoint` | control | 706 | 69 | **9.8%** | — |
| `control_dependency_injection` | control | 668 | 70 | **10.5%** | — |
| `control_exception_handling` | control | 413 | 20 | **4.8%** | — |

**Summary of flags:**
- No control fixture exceeds the 15% OOV red-flag threshold.
- Three breaks fall below the 5% OOV weakness threshold: `manual_validation` (4.4%), `raw_response` (3.6%), and `subtle_manual_status_check` (3.5%). Their foreign signal relies almost entirely on structural/sequential patterns rather than OOV vocabulary.
- The two control fixtures with elevated OOV (9.8% and 10.5%) are primarily due to user-defined names (`get_db`, `UserResponse`, `TokenPayload`, `SessionLocal`) rather than foreign framework vocabulary — not a correctness issue.

---

## 3. V1 Break Contamination Audit

---

**`paradigm_break_flask_routing`** (`routing`): OOV rate 8.7%. **Assessment: clean.**

The hunk contains no docstrings or inline comments. All OOV tokens are genuinely foreign Flask identifiers: `jsonify`, `get_json`, `abort`, `POST`, `PUT`, `DELETE` (HTTP method strings), `_users`, `_next_id`, `force`. There are no corpus-vocabulary injection points. The foreign signal is strong and concentrated: `jsonify`, `abort`, `get_json`, `app.run`, and `methods=` keyword are all absent from the corpus. The `@app.route(..., methods=[...])` decorator pattern has no equivalent in the FastAPI corpus.

---

**`paradigm_break_django_cbv`** (`framework_swap`): OOV rate 11.2%. **Assessment: clean.**

No docstrings or comments in the hunk. OOV tokens are authentic Django identifiers: `JsonResponse`, `HttpResponseNotFound`, `UserListView`, `UserDetailView`, `urlpatterns`, `safe`, `JSONDecodeError`, `View`. The `class … (View)` pattern, `request.body`, `json.loads(request.body)`, and `urlpatterns = [path(...)]` are entirely absent from the FastAPI corpus. Strong foreign signal.

---

**`paradigm_break_aiohttp_handler`** (`framework_swap`): OOV rate 13.1%. **Assessment: clean.**

No comments in the hunk body. OOV tokens are authentic aiohttp identifiers: `web`, `web.Request`, `web.Response`, `rel_url`, `json_response`, `HTTPBadRequest`, `HTTPNotFound`, `match_info`, `run_app`. The `web.Application()` + `app.router.add_get()` setup pattern and `request.match_info["user_id"]` are completely absent from the corpus. Strong foreign signal.

---

**`paradigm_break_manual_validation`** (`validation`): OOV rate 4.4%. **Assessment: WEAK — relies on structural pattern, not vocabulary.**

The hunk has no comments or docstrings. However, almost all tokens are individually present in the corpus: `isinstance`, `len`, `strip`, `int`, `str`, `raise`, `HTTPException`, `Body`. The only OOV tokens are string literals from error messages (`must`, `blank`, `chars`, `between`, `valid`), user-defined names (`_next_id`, `_users`, `update_user`), and the number `150`. The break signal is entirely structural (repeated `isinstance/len/if-raise` chains instead of a `BaseModel` parameter), not vocabulary-based. A bag-of-tokens scorer will score this near-corpus-level. Flag for Phase 2 replacement with a marshmallow/cerberus variant that injects genuinely absent vocabulary.

---

**`paradigm_break_raw_response`** (`serialization`): OOV rate 3.6%. **Assessment: WEAK — `Response`/`JSONResponse`/`jsonable_encoder` are present in corpus.**

No comments or docstrings. The OOV tokens are user-defined names only (`list_users`, `update_user`, `_users`, `_next_id`, `found`). `Response`, `JSONResponse`, `jsonable_encoder`, `json.dumps`, `media_type`, `content` are all present in the corpus (corpus originates from FastAPI's own source which defines these classes). The break signal relies entirely on the structural pattern of manually constructing `Response(content=json.dumps(...))` at every call site, which a vocabulary scorer cannot detect. Flag for Phase 2 augmentation with orjson tokens (`orjson.dumps`, `OPT_INDENT_2`) that are genuinely absent.

---

**`paradigm_break_subtle_wrong_exception`** (`exception_handling`): OOV rate 7.2%. **Assessment: clean, but signal is narrow.**

No comments. OOV tokens include `ValueError`, `KeyError`, `RuntimeError`, `health_check`, `UserResponse`, `get_db`, `already`, `registered`. Crucially `ValueError`, `KeyError`, and `RuntimeError` are absent from the corpus (the corpus uses `HTTPException` exclusively for HTTP error paths). The break signal is genuine but narrow: only 3 OOV tokens (`ValueError`, `KeyError`, `RuntimeError`) carry the actual break. The remaining OOV tokens are user-defined names.

---

**`paradigm_break_subtle_manual_status_check`** (`downstream_http`): OOV rate 3.5%. **Assessment: WEAK — all tokens present in corpus.**

No docstrings or comments in the hunk lines. OOV tokens are entirely private names (`_http_client`, `proxy_get_user`, `proxy_create_user`, `proxy_update_user`, `proxy_delete_user`, `proxy_search`, `search`) and the `#` comment-delimiter character. All structural tokens (`status_code`, `>=`, `400`, `raise`, `HTTPException`, `response`, `json`, `get`, `post`, `put`, `delete`) are present in the corpus. The break is behavioural: `raise_for_status()` replaced by explicit `if status_code >= 400: raise`, but `raise_for_status` itself has only 3 occurrences in the corpus (very rare). This is the weakest signal fixture in v1.

---

**`paradigm_break_subtle_sync_endpoint`** (`async_blocking`): OOV rate 14.4%. **Assessment: contaminated — inline comments inject vocabulary noise.**

The hunk has 5 inline comments: `# Blocking DB call in sync def — blocks the event loop thread pool`, `# Blocking requests.get() inside a FastAPI endpoint`, `# Blocking DB write in sync def`, `# Blocking delete with sleep`. These comments pull in tokens (`Blocking`, `DB`, `sync`, `blocks`, `loop`, `thread`, `pool`, `execute`) that inflate the apparent OOV count and dilute the foreign signal. A comment-aware tokenizer would strip these; a whitespace tokenizer treats them as hunk content. The structural break itself (`def` instead of `async def`, `time.sleep()`, `requests.get()`, raw SQL strings `SELECT * FROM`) is genuine, but the comment vocabulary constitutes contamination. The corpus does not contain `SELECT`, `FROM`, `WHERE`, `fetchone`, which appear in the raw SQL strings — these are legitimate OOV. Flag for v2: remove inline comments, keep SQL strings as the primary OOV source.

---

**`paradigm_break_subtle_exception_swallow`** (`exception_handling`): OOV rate 5.1%. **Assessment: clean, borderline signal.**

No inline comments in the hunk. OOV tokens are user-defined names (`UserResponse`, `get_db`, `update_user`, `delete_user`) and string content (`found`, `failed`, `warning`). The break signal (`except Exception: pass` and `except Exception as e: logger.warning(...)`) relies on `pass` (count=18 in corpus) and `warning` (absent). `Exception` appears 13 times in corpus but not in `except Exception:` context. The structural signal is weak against a bag-of-tokens approach but the `pass`-inside-except pattern combined with broad `Exception` catch is detectably non-idiomatic.

---

## 4. Break Vocabulary Shopping List

For each new category, tokens are listed with their corpus frequency. Frequency 0 = absent from corpus entirely; low frequency (≤5) = very rare.

### 4.1 Routing — Starlette bare mount

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `add_route` | 0 | aiohttp/starlette router method |
| `Route` | 2 | starlette Route class (nearly absent) |
| `add_resource` | 0 | aiohttp-style resource registration |
| `url_for` | 0 | starlette reverse-URL |
| `sub_app` | 0 | ASGI sub-app mount |
| `Router` | 0 | plain `Router()` (not `APIRouter`) |
| `{id:int}` | 0 | starlette path converter syntax |
| `web` | 0 | aiohttp `web.*` namespace |
| `RouteDef` | 0 | starlette RouteDef |
| `mount` | 1 | nearly absent |

**Recommended targets:** `add_route`, `url_for`, `Router`, `{id:int}`, `web`

---

### 4.2 Framework Swap — Tornado

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `tornado` | 0 | module name |
| `RequestHandler` | 0 | base class |
| `finish` | 0 | `self.finish()` response method |
| `get_argument` | 0 | `self.get_argument()` |
| `IOLoop` | 0 | tornado event loop |
| `ioloop` | 0 | lowercase variant |
| `Application` | 0 | `tornado.web.Application` |
| `write` | 5 | `self.write()` — rare but present |
| `HTTPServer` | 1 | nearly absent |
| `run_sync` | 1 | nearly absent |

**Recommended targets:** `tornado`, `RequestHandler`, `finish`, `get_argument`, `IOLoop`, `Application`

---

### 4.3 Validation — Marshmallow / Cerberus

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `marshmallow` | 0 | library import |
| `cerberus` | 0 | library import |
| `validates` | 0 | `@validates` decorator |
| `post_load` | 0 | `@post_load` decorator |
| `dump` | 0 | `schema.dump()` serialization |
| `ma` | 0 | conventional marshmallow alias |
| `Validator` | 9 | present (cerberus `Validator`) |
| `Schema` | 30 | present — needs context |
| `fields` | 52 | present — needs context |
| `strict` | 52 | present — needs context |

**Recommended targets:** `marshmallow`, `validates`, `post_load`, `dump`, `cerberus` — avoid `Schema`/`fields`/`strict` which are already in corpus.

---

### 4.4 Serialization — orjson

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `orjson` | 0 | library name |
| `OPT_INDENT_2` | 0 | orjson option constant |
| `OPT_NON_STR_KEYS` | 0 | orjson option constant |
| `OPT_SORT_KEYS` | 0 | orjson option constant |
| `b64encode` | 0 | bytes→base64 workaround for orjson |
| `msgpack` | 0 | alternative binary serializer |
| `cbor` | 0 | alternative binary serializer |
| `pack` | 0 | msgpack.pack |
| `ujson` | 1 | nearly absent |

**Recommended targets:** `orjson`, `OPT_INDENT_2`, `OPT_NON_STR_KEYS`, `OPT_SORT_KEYS`, `b64encode`

---

### 4.5 Exception Handling — Bare except / swallow

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `suppress` | 0 | `contextlib.suppress` |
| `swallow` | 0 | conceptual (not a Python keyword) |
| `absorb` | 0 | conceptual |
| `BaseException` | 2 | nearly absent |
| `ignore` | 3 | rare |
| `except` | 27 | present but low-density |
| `pass` | 18 | present but low-density |
| `Exception` | 13 | present but low-density |
| `contextlib` | 6 | rare |

**Recommended targets:** `suppress`, `contextlib.suppress`, `BaseException` (bare `except:` construct), plus structural pattern `except Exception: pass` as a two-token bigram. Note: bare `except:` with no exception type is fully absent from corpus.

---

### 4.6 Async Blocking

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `get_event_loop` | 0 | deprecated asyncio API |
| `run_until_complete` | 0 | sync-wrapping async from sync |
| `run_in_executor` | 0 | correct async pattern (absent) |
| `loop` | 0 | event loop variable |
| `blocking` | 0 | comment/variable term |
| `event_loop` | 0 | variable name |
| `asyncio` | 2 | rare |
| `sleep` | 2 | rare |
| `time` | 7 | rare |

**Recommended targets:** `get_event_loop`, `run_until_complete`, `loop`, `run_in_executor`, `event_loop`

---

### 4.7 Downstream HTTP — Synchronous `requests` in async endpoint

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `timeout` | 0 | requests timeout kwarg |
| `cert` | 0 | requests TLS cert |
| `urllib` | 0 | stdlib HTTP alternative |
| `raise_for_status` | 3 | nearly absent (but idiomatic!) |
| `requests` | 8 | rare — present but low |
| `verify` | 1 | nearly absent |
| `Session` | 25 | present — avoid |
| `httpx` | 3 | rare (async-safe, but present) |
| `auth` | 7 | present |

**Recommended targets:** `timeout`, `cert`, `urllib`, and the combination of `requests.get/post` inside `async def` (the structural signal). Avoid `Session` and `httpx` which have corpus presence.

---

### 4.8 Dependency Injection — Global Singletons

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `_db` | 0 | module-level private DB |
| `get_db` | 0 | plain function (no Depends) |
| `singleton` | 0 | pattern name |
| `global` | 0 | Python `global` keyword |
| `module_level` | 0 | comment term |
| `_cache` | 0 | module-level cache |
| `_conn` | 0 | module-level connection |
| `plain` | 0 | descriptive variable |
| `Depends` | 186 | present — avoid as OOV |
| `yield` | 48 | present — avoid as OOV |

**Recommended targets:** `global`, `_db`, `_cache`, `_conn`, `get_db` (called without `Depends()`). The structural break — calling `get_db()` directly vs. `Depends(get_db)` — is entirely vocabulary-invisible, so OOV tokens are the only differentiator.

---

### 4.9 Background Tasks — Threading

| Target Token | Corpus Freq | Notes |
|-------------|-------------|-------|
| `threading` | 0 | module import |
| `Thread` | 0 | `threading.Thread` |
| `thread` | 0 | lowercase variable |
| `daemon` | 0 | `t.daemon = True` |
| `Lock` | 0 | `threading.Lock` |
| `Event` | 0 | `threading.Event` |
| `Queue` | 0 | `queue.Queue` |
| `target` | 0 | Thread target kwarg |
| `join` | 4 | rare |
| `start` | 1 | nearly absent |

**Recommended targets:** `threading`, `Thread`, `daemon`, `Lock`, `target` — every one of these is fully absent from the corpus, making background-task threading fixtures the easiest category to build strong OOV signal for.

---

## Summary

| Category | v1 Status | OOV Strength | Phase 2 Priority |
|----------|-----------|--------------|-----------------|
| `routing` (flask) | shipped | 8.7% — adequate | Low (already shipped) |
| `framework_swap` (django) | shipped | 11.2% — adequate | Low |
| `framework_swap` (aiohttp) | shipped | 13.1% — adequate | Low |
| `validation` | shipped, WEAK | 4.4% — structural only | High — add marshmallow/cerberus |
| `serialization` | shipped, WEAK | 3.6% — structural only | High — add orjson |
| `exception_handling` (wrong exc) | shipped | 7.2% — narrow | Medium — 3 OOV tokens carry all signal |
| `downstream_http` | shipped, WEAK | 3.5% — worst in set | High — add `timeout`/`urllib` |
| `async_blocking` | shipped | 14.4% — contaminated | Medium — strip comments from v2 |
| `exception_handling` (swallow) | shipped | 5.1% — borderline | Medium |
| `routing` (starlette) | not yet | n/a | High — all targets absent |
| `framework_swap` (tornado) | not yet | n/a | High — all targets absent |
| `dependency_injection` | not yet | n/a | High — all targets absent |
| `background_tasks` | not yet | n/a | High — all targets absent |
