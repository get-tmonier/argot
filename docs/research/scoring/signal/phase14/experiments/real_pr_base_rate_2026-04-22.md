# Phase 14 Experiment 5 — Real-PR Base-Rate Validation on FastAPI (2026-04-22)

**Scorer:** `SequentialImportBpeScorer` V1 (unmodified from exp #2c, seed 0 calibration)

**Hypothesis (single, binary):** V1 flags fewer than 15% of merged FastAPI PRs from the last year.

**Mining criteria:** merged 2025-04-22 to 2026-04-22, non-bot authors, touches fastapi/*.py.

**Extraction:** file-start-to-hunk-end on current HEAD of cached repo (`.argot/research/repos/fastapi`).
Note: shallow clone (depth=1) — line-number drift is possible for PRs merged >90 days ago.

**Calibration:** seed=0, n=100 hunks from fastapi source.  BPE threshold: `4.0185`

**Pre-registered verdict table:**

| criterion | V1 USEFUL | V1 PLAUSIBLE | V1 INCONCLUSIVE | V1 USELESS |
|---|---|---|---|---|
| % PRs with ≥1 flagged hunk | <15% | 15–30% | 30–60% | >60% |

V1 USEFUL additionally requires ≥50% of flagged PRs to have at least one hunk judged LIKELY_STYLE_DRIFT.

---

## §1. PR-Level Summary Table

Total PRs scored: **50**  |  Total PRs flagged: **22**  |  PR flag rate: **44.0%**

| PR# | Title (60 chars) | Merged | Hunks | Flagged | Flag% | By stage |
|---|---|---|---|---|---|---|
| [15149](https://github.com/fastapi/fastapi/pull/15149) | ⬆️ Support free-threaded Python 3.14t | 2026-04-16 | 2 | 0 | 0% | — |
| [15363](https://github.com/fastapi/fastapi/pull/15363) | 🔥 Remove April Fool's `@app.vibe()` 🤪 | 2026-04-16 | 2 | 1 | 50% | import:1 |
| [15280](https://github.com/fastapi/fastapi/pull/15280) | ✨ Add support for `@app.vibe()` | 2026-04-01 | 2 | 1 | 50% | import:1 |
| [14946](https://github.com/fastapi/fastapi/pull/14946) | ✏️ Fix typo for `client_secret` in OAuth2 form docstrings | 2026-03-24 | 2 | 0 | 0% | — |
| [15116](https://github.com/fastapi/fastapi/pull/15116) | 📝 Fix duplicated words in docstrings | 2026-03-16 | 4 | 1 | 25% | bpe:1 |
| [15091](https://github.com/fastapi/fastapi/pull/15091) | 👷 Add `ty` to precommit | 2026-03-15 | 49 | 8 | 16% | bpe:5+import:3 |
| [14944](https://github.com/fastapi/fastapi/pull/14944) | 📝 Fix doctrings for `max_digits` and `decimal_places` | 2026-03-04 | 14 | 0 | 0% | — |
| [15038](https://github.com/fastapi/fastapi/pull/15038) | 🐛 Fix, avoid yield from a TaskGroup, only as an async contex | 2026-03-01 | 4 | 3 | 75% | bpe:3 |
| [15030](https://github.com/fastapi/fastapi/pull/15030) | ✨ Add support for Server Sent Events | 2026-03-01 | 12 | 5 | 42% | bpe:5 |
| [15022](https://github.com/fastapi/fastapi/pull/15022) | ✨ Add support for streaming JSON Lines and binary data with  | 2026-02-27 | 16 | 5 | 31% | bpe:5 |
| [14986](https://github.com/fastapi/fastapi/pull/14986) | ♻️ Refactor logic to handle OpenAPI and Swagger UI escaping  | 2026-02-24 | 4 | 1 | 25% | import:1 |
| [14978](https://github.com/fastapi/fastapi/pull/14978) | 🔒️ Add `strict_content_type` checking for JSON requests | 2026-02-23 | 13 | 10 | 77% | bpe:6+import:4 |
| [14964](https://github.com/fastapi/fastapi/pull/14964) | 🗑️ Deprecate `ORJSONResponse` and `UJSONResponse` | 2026-02-22 | 4 | 0 | 0% | — |
| [14962](https://github.com/fastapi/fastapi/pull/14962) | ✨ Serialize JSON response with Pydantic (in Rust), when ther | 2026-02-22 | 5 | 2 | 40% | bpe:2 |
| [14953](https://github.com/fastapi/fastapi/pull/14953) | ♻️ Fix JSON Schema for bytes, use `"contentMediaType": "appl | 2026-02-21 | 3 | 0 | 0% | — |
| [14898](https://github.com/fastapi/fastapi/pull/14898) | 🎨 Update internal types for Python 3.10 | 2026-02-11 | 586 | 284 | 48% | bpe:34+import:250 |
| [14897](https://github.com/fastapi/fastapi/pull/14897) | ➖ Drop support for Python 3.9 | 2026-02-11 | 22 | 0 | 0% | — |
| [14884](https://github.com/fastapi/fastapi/pull/14884) | ♻️ Simplify reading files in memory, do it sequentially inst | 2026-02-10 | 3 | 0 | 0% | — |
| [14258](https://github.com/fastapi/fastapi/pull/14258) | ✨ Show a clear error on attempt to include router into itsel | 2026-02-10 | 1 | 1 | 100% | import:1 |
| [14873](https://github.com/fastapi/fastapi/pull/14873) | 🐛 Fix `on_startup` and `on_shutdown` parameters of `APIRoute | 2026-02-09 | 2 | 2 | 100% | bpe:2 |
| [14862](https://github.com/fastapi/fastapi/pull/14862) | ♻️ Refactor and simplify Pydantic v2 (and v1) compatibility  | 2026-02-07 | 20 | 0 | 0% | — |
| [14860](https://github.com/fastapi/fastapi/pull/14860) | ♻️ Refactor internals, simplify Pydantic v2/v1 utils, `creat | 2026-02-07 | 22 | 2 | 9% | bpe:2 |
| [14857](https://github.com/fastapi/fastapi/pull/14857) | ♻️ Simplify internals, remove Pydantic v1 only logic, no lon | 2026-02-06 | 11 | 0 | 0% | — |
| [14856](https://github.com/fastapi/fastapi/pull/14856) | ♻️ Refactor internals, cleanup unneeded Pydantic v1 specific | 2026-02-06 | 10 | 2 | 20% | bpe:2 |
| [14851](https://github.com/fastapi/fastapi/pull/14851) | ♻️ Re-implement `on_event` in FastAPI for compatibility with | 2026-02-06 | 5 | 2 | 40% | bpe:1+import:1 |
| [14616](https://github.com/fastapi/fastapi/pull/14616) | 🐛 Fix using `Json[list[str]]` type (issue #10997) | 2026-02-05 | 2 | 0 | 0% | — |
| [14794](https://github.com/fastapi/fastapi/pull/14794) | ✨ Allow `Response` type hint as dependency annotation | 2026-02-05 | 2 | 0 | 0% | — |
| [14791](https://github.com/fastapi/fastapi/pull/14791) | 🐛 Update `ValidationError` schema to include `input` and `ct | 2026-02-04 | 1 | 0 | 0% | — |
| [14816](https://github.com/fastapi/fastapi/pull/14816) | 🎨 Tweak types for mypy | 2026-02-04 | 1 | 0 | 0% | — |
| [14789](https://github.com/fastapi/fastapi/pull/14789) | 🐛 Fix TYPE_CHECKING annotations for Python 3.14 (PEP 649) | 2026-02-04 | 1 | 0 | 0% | — |
| [14786](https://github.com/fastapi/fastapi/pull/14786) | 🐛 Strip whitespaces from `Authorization` header credentials | 2026-02-04 | 1 | 0 | 0% | — |
| [14777](https://github.com/fastapi/fastapi/pull/14777) | ✨ Add `viewport` meta tag to improve Swagger UI on mobile de | 2026-02-04 | 1 | 0 | 0% | — |
| [14641](https://github.com/fastapi/fastapi/pull/14641) | 🏷️ Re-export `IncEx` type from Pydantic instead of duplicati | 2026-02-04 | 1 | 0 | 0% | — |
| [14479](https://github.com/fastapi/fastapi/pull/14479) | 🚸 Improve error message for invalid query parameter type ann | 2026-02-04 | 1 | 0 | 0% | — |
| [14463](https://github.com/fastapi/fastapi/pull/14463) | 🐛 Fix OpenAPI duplication of `anyOf` refs for app-level resp | 2026-02-04 | 2 | 0 | 0% | — |
| [14776](https://github.com/fastapi/fastapi/pull/14776) | 📝 Add links to related sections of docs to docstrings | 2026-02-04 | 57 | 3 | 5% | import:3 |
| [14756](https://github.com/fastapi/fastapi/pull/14756) | 📝 Use `WSGIMiddleware` from `a2wsgi` instead of deprecated ` | 2026-02-04 | 1 | 0 | 0% | — |
| [14814](https://github.com/fastapi/fastapi/pull/14814) | 💡 Update comment for Pydantic internals | 2026-02-04 | 2 | 0 | 0% | — |
| [14806](https://github.com/fastapi/fastapi/pull/14806) | 👷 Run mypy by pre-commit | 2026-02-03 | 5 | 0 | 0% | — |
| [14609](https://github.com/fastapi/fastapi/pull/14609) | ➖ Drop support for `pydantic.v1` | 2025-12-27 | 86 | 3 | 3% | bpe:3 |
| [14605](https://github.com/fastapi/fastapi/pull/14605) | 🔊 Add a custom `FastAPIDeprecationWarning` | 2025-12-26 | 15 | 2 | 13% | bpe:2 |
| [14583](https://github.com/fastapi/fastapi/pull/14583) | 🔊 Add deprecation warnings when using `pydantic.v1` | 2025-12-21 | 6 | 2 | 33% | bpe:2 |
| [14575](https://github.com/fastapi/fastapi/pull/14575) | ➖ Drop support for Pydantic v1, keeping short temporary supp | 2025-12-20 | 29 | 0 | 0% | — |
| [14564](https://github.com/fastapi/fastapi/pull/14564) | ♻️ Upgrade internal syntax to Python 3.9+ 🎉 | 2025-12-17 | 301 | 129 | 43% | bpe:20+import:109 |
| [14371](https://github.com/fastapi/fastapi/pull/14371) | 🐛 Fix parameter aliases | 2025-12-12 | 13 | 0 | 0% | — |
| [14512](https://github.com/fastapi/fastapi/pull/14512) | 🐛 Fix support for tagged union with discriminator inside of  | 2025-12-12 | 3 | 0 | 0% | — |
| [14485](https://github.com/fastapi/fastapi/pull/14485) | 🐛 Fix support for `if TYPE_CHECKING`,  non-evaluated stringi | 2025-12-10 | 2 | 0 | 0% | — |
| [14482](https://github.com/fastapi/fastapi/pull/14482) | 🐛 Fix handling arbitrary types when using `arbitrary_types_a | 2025-12-10 | 5 | 0 | 0% | — |
| [14306](https://github.com/fastapi/fastapi/pull/14306) | 🚸  Improve tracebacks by adding endpoint metadata | 2025-12-06 | 13 | 4 | 31% | bpe:4 |
| [14458](https://github.com/fastapi/fastapi/pull/14458) | 🐛 Fix using class (not instance) dependency that has `__call | 2025-12-05 | 4 | 0 | 0% | — |

---

## §2. Aggregate Stats

- **Total PRs scored:** 50
- **Total source hunks scored:** 1373
- **PRs with ≥1 flagged hunk:** 22 (44.0%)

### PR-level flag_pct distribution

| bin | count | % of PRs |
|---|---|---|
| 0% | 28 | 56% |
| 1–10% | 3 | 6% |
| 10–25% | 5 | 10% |
| 25–50% | 10 | 20% |
| 50–100% | 4 | 8% |

### Stage attribution (flagged source hunks)

| stage | count | % of flagged hunks |
|---|---|---|
| Stage 1 (import) | 374 | 79% |
| Stage 2 (BPE only) | 99 | 21% |

---

## §3. Drift Check (PR age vs flag rate)

| age bucket | n_prs | n_flagged | flag_rate |
|---|---|---|---|
| ≤90 days | 39 | 17 | 44% |
| 91–180 days | 11 | 5 | 45% |
| 181–365 days | 0 | — | — |

Drift interpretation: if recent PRs flag dramatically less than old PRs,
line-number drift from the shallow clone is inflating old-PR flag rates.

---

## §4. Test-File Diagnostic

Test-file hunks scored: 1200
Test-file hunks flagged: 1157 (96.4%)

High flag rate on test hunks would confirm that test files need their own calibration treatment.

---

## §5. Sample Inspection (up to 10 flagged source hunks)

### PR #15363 — 🔥 Remove April Fool's `@app.vibe()` 🤪

- **URL:** https://github.com/fastapi/fastapi/pull/15363
- **File:** `fastapi/applications.py`  lines 4559–4564
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -4563,60 +4559,6 @@ def trace_item(item_id: str):
             generate_unique_id_function=generate_unique_id_function,
         )
 
-    def vibe(
-        self,
-        path: Annotated[
-            str,
-            Doc(
-                """
-                The URL path to be used for this *path operation*.
-
-                For example, in `http://example.com/vibes`, the path is `/vibes`.
-                """
-            ),
-        ],
-        *,
-        prompt: Annotated[
-            str,
-            Doc(
-                """
-                The prompt to send to the LLM provi
```

### PR #15280 — ✨ Add support for `@app.vibe()`

- **URL:** https://github.com/fastapi/fastapi/pull/15280
- **File:** `fastapi/applications.py`  lines 4563–4622
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -4559,6 +4563,60 @@ def trace_item(item_id: str):
             generate_unique_id_function=generate_unique_id_function,
         )
 
+    def vibe(
+        self,
+        path: Annotated[
+            str,
+            Doc(
+                """
+                The URL path to be used for this *path operation*.
+
+                For example, in `http://example.com/vibes`, the path is `/vibes`.
+                """
+            ),
+        ],
+        *,
+        prompt: Annotated[
+            str,
+            Doc(
+                """
+                The prompt to send to the LLM provi
```

### PR #15091 — 👷 Add `ty` to precommit

- **URL:** https://github.com/fastapi/fastapi/pull/15091
- **File:** `fastapi/applications.py`  lines 1002–1013
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -1006,11 +1002,12 @@ class Item(BaseModel):
         self.exception_handlers.setdefault(
             RequestValidationError, request_validation_exception_handler
         )
+
+        # Starlette still has incorrect type specification for the handlers
         self.exception_handlers.setdefault(
             WebSocketRequestValidationError,
-            # Starlette still has incorrect type specification for the handlers
-            websocket_request_validation_exception_handler,  # type: ignore
-        )
+            websocket_request_validation_exception_handler,  # type: ignore[arg-typ
```

### PR #14986 — ♻️ Refactor logic to handle OpenAPI and Swagger UI escaping 

- **URL:** https://github.com/fastapi/fastapi/pull/14986
- **File:** `fastapi/applications.py`  lines 1101–1118
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -1101,16 +1101,18 @@ def openapi(self) -> dict[str, Any]:
 
     def setup(self) -> None:
         if self.openapi_url:
-            urls = (server_data.get("url") for server_data in self.servers)
-            server_urls = {url for url in urls if url}
 
             async def openapi(req: Request) -> JSONResponse:
                 root_path = req.scope.get("root_path", "").rstrip("/")
-                if root_path not in server_urls:
-                    if root_path and self.root_path_in_servers:
-                        self.servers.insert(0, {"url": root_path})
-
```

### PR #14978 — 🔒️ Add `strict_content_type` checking for JSON requests

- **URL:** https://github.com/fastapi/fastapi/pull/14978
- **File:** `fastapi/applications.py`  lines 840–868
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -840,6 +840,29 @@ class Item(BaseModel):
                 """
             ),
         ] = None,
+        strict_content_type: Annotated[
+            bool,
+            Doc(
+                """
+                Enable strict checking for request Content-Type headers.
+
+                When `True` (the default), requests with a body that do not include
+                a `Content-Type` header will **not** be parsed as JSON.
+
+                This prevents potential cross-site request forgery (CSRF) attacks
+                that exploit the browser's ability to send requests without a
+
```

### PR #14898 — 🎨 Update internal types for Python 3.10

- **URL:** https://github.com/fastapi/fastapi/pull/14898
- **File:** `fastapi/applications.py`  lines 74–80
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -77,7 +74,7 @@ def __init__(
             ),
         ] = False,
         routes: Annotated[
-            Optional[list[BaseRoute]],
+            list[BaseRoute] | None,
             Doc(
                 """
                 **Note**: you probably shouldn't use this parameter, it is inherited
```

### PR #14258 — ✨ Show a clear error on attempt to include router into itsel

- **URL:** https://github.com/fastapi/fastapi/pull/14258
- **File:** `fastapi/routing.py`  lines 1393–1402
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -1393,6 +1393,10 @@ def read_users():
         app.include_router(internal_router)
         ```
         """
+        assert self is not router, (
+            "Cannot include the same APIRouter instance into itself. "
+            "Did you mean to include a different router?"
+        )
         if prefix:
             assert prefix.startswith("/"), "A path prefix must start with '/'"
             assert not prefix.endswith("/"), (
```

### PR #14851 — ♻️ Re-implement `on_event` in FastAPI for compatibility with

- **URL:** https://github.com/fastapi/fastapi/pull/14851
- **File:** `fastapi/routing.py`  lines 4570–4627
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -4473,6 +4570,58 @@ def trace_item(item_id: str):
             generate_unique_id_function=generate_unique_id_function,
         )
 
+    # TODO: remove this once the lifespan (or alternative) interface is improved
+    async def _startup(self) -> None:
+        """
+        Run any `.on_startup` event handlers.
+
+        This method is kept for backward compatibility after Starlette removed
+        support for on_startup/on_shutdown handlers.
+
+        Ref: https://github.com/Kludex/starlette/pull/3117
+        """
+        for handler in self.on_startup:
+            if is_async_callab
```

### PR #14776 — 📝 Add links to related sections of docs to docstrings

- **URL:** https://github.com/fastapi/fastapi/pull/14776
- **File:** `fastapi/applications.py`  lines 672–678
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -672,7 +672,7 @@ async def read_items():
                 in the autogenerated OpenAPI using the `root_path`.
 
                 Read more about it in the
-                [FastAPI docs for Behind a Proxy](https://fastapi.tiangolo.com/advanced/behind-a-proxy/#disable-automatic-server-from-root_path).
+                [FastAPI docs for Behind a Proxy](https://fastapi.tiangolo.com/advanced/behind-a-proxy/#disable-automatic-server-from-root-path).
 
                 **Example**
```

### PR #14564 — ♻️ Upgrade internal syntax to Python 3.9+ 🎉

- **URL:** https://github.com/fastapi/fastapi/pull/14564
- **File:** `fastapi/applications.py`  lines 77–83
- **Stage:** Stage 1 (import_score=1.0)
- **Judgment:** LIKELY_STYLE_DRIFT
- **Rationale:** Stage 1 flagged foreign module(s) never seen in fastapi/ source: Starlette.

```diff
@@ -81,7 +77,7 @@ def __init__(
             ),
         ] = False,
         routes: Annotated[
-            Optional[List[BaseRoute]],
+            Optional[list[BaseRoute]],
             Doc(
                 """
                 **Note**: you probably shouldn't use this parameter, it is inherited
```


---

## §6. High-Flag PRs (flag_pct > 50%)

| PR# | Title | flag_pct | stage breakdown |
|---|---|---|---|
| [14258](https://github.com/fastapi/fastapi/pull/14258) | ✨ Show a clear error on attempt to include router into itsel | 100% | import:1 |
| [14873](https://github.com/fastapi/fastapi/pull/14873) | 🐛 Fix `on_startup` and `on_shutdown` parameters of `APIRoute | 100% | bpe:2 |
| [14978](https://github.com/fastapi/fastapi/pull/14978) | 🔒️ Add `strict_content_type` checking for JSON requests | 77% | bpe:6+import:4 |
| [15038](https://github.com/fastapi/fastapi/pull/15038) | 🐛 Fix, avoid yield from a TaskGroup, only as an async contex | 75% | bpe:3 |

**Sample hunk from each high-flag PR:**

**PR #15038** `fastapi/routing.py` lines 527–536
reason=bpe | judgment=AMBIGUOUS

```diff
@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes:
                 else:
                     sse_aiter = iterate_in_threadpool(gen)
 
-                async def _async_stream_sse() -> AsyncIterator[bytes]:
+                @asynccontextmanager
+                async def _sse_producer_cm() -> AsyncIterator[
+                    ObjectReceiveStream[bytes]
+                ]:
```

**PR #14978** `fastapi/applications.py` lines 840–868
reason=import | judgment=LIKELY_STYLE_DRIFT

```diff
@@ -840,6 +840,29 @@ class Item(BaseModel):
                 """
             ),
         ] = None,
+        strict_content_type: Annotated[
+            bool,
+            Doc(
+                """
+                Enable strict checking for request Content-Type headers.
+
+                When `True` (the default), requests with a body that do not include
+                a `Content-Type` header
```

**PR #14258** `fastapi/routing.py` lines 1393–1402
reason=import | judgment=LIKELY_STYLE_DRIFT

```diff
@@ -1393,6 +1393,10 @@ def read_users():
         app.include_router(internal_router)
         ```
         """
+        assert self is not router, (
+            "Cannot include the same APIRouter instance into itself. "
+            "Did you mean to include a different router?"
+        )
         if prefix:
             assert prefix.startswith("/"), "A path prefix must start with '/'"
```

**PR #14873** `fastapi/routing.py` lines 952–957
reason=bpe | judgment=AMBIGUOUS

```diff
@@ -952,16 +952,6 @@ def __init__(
             ),
         ] = Default(generate_unique_id),
     ) -> None:
-        # Handle on_startup/on_shutdown locally since Starlette removed support
-        # Ref: https://github.com/Kludex/starlette/pull/3117
-        # TODO: deprecate this once the lifespan (or alternative) interface is improved
-        self.on_startup: list[Callable[[], Any]] = (
-
```


---

## §7. Stage 1 (Import) Breakdown — Foreign Modules

Total Stage 1 flags: 374

| foreign module | hunk count |
|---|---|
| `Starlette` | 374 |

**All 374 Stage 1 flags are from one "module": `Starlette` (capital S). This is not a real import.**

### §7a. Root Cause: ast.parse Failure → Regex Fallback → Docstring Text

Verified experimentally. `fastapi/applications.py` has 4691 lines. When a hunk's `end_line` falls
inside a function body (e.g. line 4564, inside a `def` starting at line 4562), the "file-start-to-hunk-end"
extraction is syntactically invalid Python. `ast.parse` raises `SyntaxError`:

```
SyntaxError: expected an indented block after function definition on line 4562
```

`ImportGraphScorer._imports_from_ast` then falls back to `_imports_from_regex`, which applies:

```python
_RE_FROM_IMPORT = re.compile(r"^\s*from\s+([A-Za-z_]\w*)", re.MULTILINE)
```

This regex matches **docstring prose** at lines 77 and 87 of `applications.py`:

```
from Starlette and supported for compatibility.
```

The regex captures `Starlette` (capital S — a class name in prose). `_repo_modules` contains
`starlette` (lowercase, from `from starlette.applications import Starlette`), but NOT `Starlette`
(capital S). The scorer treats it as a foreign module and fires Stage 1.

**Impact:** Every hunk in `fastapi/applications.py` or `fastapi/routing.py` whose `end_line` truncates
the file mid-function triggers this bug. Since these are the two largest files in the codebase, the bug
propagates to nearly every PR that touches them.

**Fix:** In `ImportGraphScorer._imports_from_ast`, do NOT fall back to regex on SyntaxError — return
`set()` instead. Regex cannot distinguish real import lines from prose in docstrings.

### §7b. Post-Fix Estimated Impact

With Stage 1 corrected (regex fallback removed), all 374 `Starlette` flags disappear.
PRs with ONLY Stage 1 flags become unflagged:

| PR | Stage breakdown | Post-fix status |
|---|---|---|
| #15363 | import:1 | UNFLAGGED |
| #15280 | import:1 | UNFLAGGED |
| #14986 | import:1 | UNFLAGGED |
| #14258 | import:1 | UNFLAGGED |
| #14776 | import:3 | UNFLAGGED |

PRs with Stage 2 (BPE) flags retain those flags regardless of the Stage 1 fix.

**Estimated post-fix PR flag rate: 17/50 = 34% → still V1 INCONCLUSIVE.**

Stage 2 false positives require separate investigation (see §8).

---

## §8. Verdict

| metric | value |
|---|---|
| PRs scored | 50 |
| PRs flagged | 22 (44.0%) |
| Source hunks scored | 1373 |
| Source hunks flagged | 473 |
| BPE threshold (seed=0) | 4.0185 |
| Stage 1 flags | 374 (all false positives — see §7a) |
| Stage 2 flags | 99 |
| Flagged hunks → LIKELY_STYLE_DRIFT | 374/473 (79%) |

**PR flag rate: 44.0% → V1 INCONCLUSIVE**

The headline rate overstates actual V1 behavior. Decomposing by root cause:

| root cause | PRs with ≥1 such flag | PRs UNFLAGGED after fix |
|---|---|---|
| Stage 1 regex fallback bug (§7a) | 10 | 5 (import-only) |
| Stage 2 BPE over-trigger | 17 | requires separate analysis |

**After Stage 1 fix alone: ~17/50 = 34% → still V1 INCONCLUSIVE.**

### Stage 2 (BPE) over-trigger

The 99 Stage 2 flags span 17 PRs. The two highest-volume flaggers:

- **PR #14898** (Update internal types for Python 3.10): 34 BPE flags / 586 hunks (6%).
  Bulk type-annotation migration (`Optional[X]` → `X | None`). The BPE scorer sees large
  docstring-heavy file extractions; calibration (100 short function bodies) likely under-represents
  docstring token density, making the threshold too tight for full-file extractions.

- **PR #14564** (Upgrade internal syntax to Python 3.9+): 20 BPE flags / 301 hunks (7%).
  Same pattern: bulk syntax upgrade touching docstring-heavy files.

- **PR #15038** (Fix TaskGroup yield): 3 BPE flags / 4 hunks (75%). Focused bugfix with high
  hunk flag rate — BPE threshold appears too tight for async routing code patterns.

**Hypothesis:** Calibration hunks (100 sampled short function bodies) under-represent docstring-heavy
content. Large PRs that expand into long `Doc(...)` parameter descriptions produce extractions
dominated by English prose. Model_B (generic corpus) scores English prose higher than model_A
(per-repo code corpus) → BPE score exceeds threshold.

### Minimum changes before real-world re-test

1. **Fix Stage 1 regex fallback** (§7a): `_imports_from_ast` should return `set()` on SyntaxError
   rather than falling back to regex. Verified fix: this removes all 374 false Stage 1 flags.

2. **Investigate Stage 2 calibration gap**: Verify whether calibration hunks include docstring-heavy
   code proportional to what real PRs touch. If not: expand calibration sample or switch to
   per-file-type thresholds.

3. **Re-validate on synthetic fixtures** after each fix to confirm recall is preserved.

4. **Re-run this experiment** on the fixed scorer to measure actual post-fix base rate.

