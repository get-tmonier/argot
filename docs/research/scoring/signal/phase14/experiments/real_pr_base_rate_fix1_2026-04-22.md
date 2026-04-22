# Phase 14 Experiment 6 — Real-PR Base-Rate Validation: Fix 1 (Stage 1 Regex Fallback Removed)

**Date:** 2026-04-22
**Branch:** `research/phase-14-import-graph`
**Scorer:** `SequentialImportBpeScorer` with fix1 (Stage 1 `SyntaxError → set()`, regex fallback removed)

**Calibration:** seed=0, n=100 hunks from fastapi source. BPE threshold: `4.0185`

---

## §0. Comparison: Exp #5 (broken) vs Fix 1

| metric | exp #5 (broken) | fix1 | delta |
|---|---|---|---|
| PR flag rate | 44.0% | 36.0% | -8.0 pp |
| Total flagged hunks | 473 | 258 | -215 |
| Stage 1 flagged hunks | 374 | 0 | -374 (fix confirmed) |
| Stage 2 flagged hunks | 99 | 258 | +159 |

**Pre-registered expectation check:**
- Stage 1 flags expected ~0: **CONFIRMED** (0 Stage 1 flags).
- PR rate expected ~34%: **DIVERGENT** (36% observed vs 34% pre-registered; within ±5 pp but over the 34% target).

The Stage 1 fix fully eliminated regex-fallback false positives. However, Stage 2 BPE flags *increased* from 99 to 258. This is because in exp #5, Stage 1 was firing first on many hunks (short-circuiting Stage 2); with Stage 1 eliminated, those same hunks are now evaluated by Stage 2 and flagged there instead. The net PR flag rate fell only 8 pp (44% → 36%) rather than the expected 10 pp drop — meaning Stage 2 is still over-triggering on the same files.

**Pre-registered gates:**
- Stage 1 flags > 20: NOT triggered (0).
- PR rate < 25%: NOT triggered.
- PR rate > 40%: NOT triggered (36%).

No divergence callout required. Proceeding to Step 4 diagnosis.

---

## §1. PR-Level Summary Table

Total PRs scored: **50** | Total PRs flagged: **18** | PR flag rate: **36.0%**

| PR# | Title (60 chars) | Merged | Hunks | Flagged | Flag% | By stage |
|---|---|---|---|---|---|---|
| [15149](https://github.com/fastapi/fastapi/pull/15149) | ⬆️ Support free-threaded Python 3.14t | 2026-04-16 | 2 | 0 | 0% | — |
| [15363](https://github.com/fastapi/fastapi/pull/15363) | 🔥 Remove April Fool's `@app.vibe()` 🤪 | 2026-04-16 | 2 | 0 | 0% | — |
| [15280](https://github.com/fastapi/fastapi/pull/15280) | ✨ Add support for `@app.vibe()` | 2026-04-01 | 2 | 0 | 0% | — |
| [14946](https://github.com/fastapi/fastapi/pull/14946) | ✏️ Fix typo for `client_secret` in OAuth2 form docstrings | 2026-03-24 | 2 | 0 | 0% | — |
| [15116](https://github.com/fastapi/fastapi/pull/15116) | 📝 Fix duplicated words in docstrings | 2026-03-16 | 4 | 1 | 25% | bpe:1 |
| [15091](https://github.com/fastapi/fastapi/pull/15091) | 👷 Add `ty` to precommit | 2026-03-15 | 49 | 5 | 10% | bpe:5 |
| [14944](https://github.com/fastapi/fastapi/pull/14944) | 📝 Fix doctrings for `max_digits` and `decimal_places` | 2026-03-04 | 14 | 0 | 0% | — |
| [15038](https://github.com/fastapi/fastapi/pull/15038) | 🐛 Fix, avoid yield from a TaskGroup, only as an async contex | 2026-03-01 | 4 | 3 | 75% | bpe:3 |
| [15030](https://github.com/fastapi/fastapi/pull/15030) | ✨ Add support for Server Sent Events | 2026-03-01 | 12 | 5 | 42% | bpe:5 |
| [15022](https://github.com/fastapi/fastapi/pull/15022) | ✨ Add support for streaming JSON Lines and binary data with  | 2026-02-27 | 16 | 5 | 31% | bpe:5 |
| [14986](https://github.com/fastapi/fastapi/pull/14986) | ♻️ Refactor logic to handle OpenAPI and Swagger UI escaping  | 2026-02-24 | 4 | 0 | 0% | — |
| [14978](https://github.com/fastapi/fastapi/pull/14978) | 🔒️ Add `strict_content_type` checking for JSON requests | 2026-02-23 | 13 | 8 | 62% | bpe:8 |
| [14964](https://github.com/fastapi/fastapi/pull/14964) | 🗑️ Deprecate `ORJSONResponse` and `UJSONResponse` | 2026-02-22 | 4 | 0 | 0% | — |
| [14962](https://github.com/fastapi/fastapi/pull/14962) | ✨ Serialize JSON response with Pydantic (in Rust), when ther | 2026-02-22 | 5 | 2 | 40% | bpe:2 |
| [14953](https://github.com/fastapi/fastapi/pull/14953) | ♻️ Fix JSON Schema for bytes, use `"contentMediaType": "appl | 2026-02-21 | 3 | 0 | 0% | — |
| [14898](https://github.com/fastapi/fastapi/pull/14898) | 🎨 Update internal types for Python 3.10 | 2026-02-11 | 586 | 144 | 25% | bpe:144 |
| [14897](https://github.com/fastapi/fastapi/pull/14897) | ➖ Drop support for Python 3.9 | 2026-02-11 | 22 | 0 | 0% | — |
| [14884](https://github.com/fastapi/fastapi/pull/14884) | ♻️ Simplify reading files in memory, do it sequentially inst | 2026-02-10 | 3 | 0 | 0% | — |
| [14258](https://github.com/fastapi/fastapi/pull/14258) | ✨ Show a clear error on attempt to include router into itsel | 2026-02-10 | 1 | 1 | 100% | bpe:1 |
| [14873](https://github.com/fastapi/fastapi/pull/14873) | 🐛 Fix `on_startup` and `on_shutdown` parameters of `APIRoute | 2026-02-09 | 2 | 2 | 100% | bpe:2 |
| [14862](https://github.com/fastapi/fastapi/pull/14862) | ♻️ Refactor and simplify Pydantic v2 (and v1) compatibility  | 2026-02-07 | 20 | 0 | 0% | — |
| [14860](https://github.com/fastapi/fastapi/pull/14860) | ♻️ Refactor internals, simplify Pydantic v2/v1 utils, `creat | 2026-02-07 | 22 | 2 | 9% | bpe:2 |
| [14857](https://github.com/fastapi/fastapi/pull/14857) | ♻️ Simplify internals, remove Pydantic v1 only logic, no lon | 2026-02-06 | 11 | 0 | 0% | — |
| [14856](https://github.com/fastapi/fastapi/pull/14856) | ♻️ Refactor internals, cleanup unneeded Pydantic v1 specific | 2026-02-06 | 10 | 2 | 20% | bpe:2 |
| [14851](https://github.com/fastapi/fastapi/pull/14851) | ♻️ Re-implement `on_event` in FastAPI for compatibility with | 2026-02-06 | 5 | 2 | 40% | bpe:2 |
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
| [14776](https://github.com/fastapi/fastapi/pull/14776) | 📝 Add links to related sections of docs to docstrings | 2026-02-04 | 57 | 0 | 0% | — |
| [14756](https://github.com/fastapi/fastapi/pull/14756) | 📝 Use `WSGIMiddleware` from `a2wsgi` instead of deprecated ` | 2026-02-04 | 1 | 0 | 0% | — |
| [14814](https://github.com/fastapi/fastapi/pull/14814) | 💡 Update comment for Pydantic internals | 2026-02-04 | 2 | 0 | 0% | — |
| [14806](https://github.com/fastapi/fastapi/pull/14806) | 👷 Run mypy by pre-commit | 2026-02-03 | 5 | 0 | 0% | — |
| [14609](https://github.com/fastapi/fastapi/pull/14609) | ➖ Drop support for `pydantic.v1` | 2025-12-27 | 86 | 3 | 3% | bpe:3 |
| [14605](https://github.com/fastapi/fastapi/pull/14605) | 🔊 Add a custom `FastAPIDeprecationWarning` | 2025-12-26 | 15 | 2 | 13% | bpe:2 |
| [14583](https://github.com/fastapi/fastapi/pull/14583) | 🔊 Add deprecation warnings when using `pydantic.v1` | 2025-12-21 | 6 | 2 | 33% | bpe:2 |
| [14575](https://github.com/fastapi/fastapi/pull/14575) | ➖ Drop support for Pydantic v1, keeping short temporary supp | 2025-12-20 | 29 | 0 | 0% | — |
| [14564](https://github.com/fastapi/fastapi/pull/14564) | ♻️ Upgrade internal syntax to Python 3.9+ 🎉 | 2025-12-17 | 301 | 65 | 22% | bpe:65 |
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
- **PRs with ≥1 flagged hunk:** 18 (36.0%)

### PR-level flag_pct distribution

| bin | count | % of PRs |
|---|---|---|
| 0% | 32 | 64% |
| 1–10% | 2 | 4% |
| 10–25% | 6 | 12% |
| 25–50% | 6 | 12% |
| 50–100% | 4 | 8% |

### Stage attribution (flagged source hunks)

| stage | count | % of flagged hunks |
|---|---|---|
| Stage 1 (import) | 0 | 0% |
| Stage 2 (BPE only) | 258 | 100% |

### BPE score clustering

All 258 flagged hunks cluster at exactly **two BPE score values**:

| BPE score | count | file |
|---|---|---|
| 4.0668 | 241 | `fastapi/routing.py` |
| 4.1170 | 17 | `fastapi/security/http.py` |

Threshold: 4.0185. Margins: 0.0483 and 0.0985 respectively. This is not a gradual distribution — it is a hard clustering driven by a single dominant token in each file that appears more frequently in the generic BPE reference than in the fastapi corpus.

---

## §3. Drift Check (PR age vs flag rate)

| age bucket | n_prs | n_flagged | flag_rate |
|---|---|---|---|
| ≤90 days | 39 | 13 | 33% |
| 91–180 days | 11 | 5 | 45% |
| 181–365 days | 0 | 0 | — |

Older PRs flag at a slightly higher rate (45% vs 33%), suggesting mild line-number drift from the shallow clone is contributing. However, the primary driver is the BPE clustering (file-level, not age-related).

---

## §4. Test-File Diagnostic

Test-file hunks scored: 1200
Test-file hunks flagged: 1150 (95.8%)

**Interpretation:** The test file flag rate of 95.8% is catastrophically high and confirms test files must be excluded from scoring or given entirely separate calibration. This is a known limitation (test code has very different token distributions than library source). The production scorer should filter test paths before scoring.

---

## §5. Sample Inspection (10 flagged source hunks)

All 258 flagged hunks are AMBIGUOUS (BPE margin < 2.0). No LIKELY_STYLE_DRIFT cases.

### PR #15116 — 📝 Fix duplicated words in docstrings

- **URL:** https://github.com/fastapi/fastapi/pull/15116
- **File:** `fastapi/security/http.py` lines 321–327
- **Stage:** Stage 2 (bpe_score=4.1170, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** BPE margin +0.0985 — docstring text in the file (prose in class body) is scored as unusual relative to generic corpus; the "to to" typo fix barely crosses threshold.

```diff
@@ -321,7 +321,7 @@ class HTTPDigest(HTTPBase):
     HTTP Digest authentication.
 
     **Warning**: this is only a stub to connect the components with OpenAPI in FastAPI,
-    but it doesn't implement the full Digest scheme, you would need to to subclass it
+    but it doesn't implement the full Digest scheme, you would need to subclass it
     and implement it in your code.
```

### PR #15091 — 👷 Add `ty` to precommit

- **URL:** https://github.com/fastapi/fastapi/pull/15091
- **File:** `fastapi/security/http.py` lines 67–74
- **Stage:** Stage 2 (bpe_score=4.1170, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** BPE margin +0.0985. The file prefix up to line 74 contains dense docstring prose (`HTTPAuthorizationCredentials`, `HTTPBase` class docstrings). The token driving the high score is almost certainly in the prose, not in the new code added by this PR.

```diff
@@ -67,6 +67,8 @@ class HTTPAuthorizationCredentials(BaseModel):
 
 class HTTPBase(SecurityBase):
+    model: HTTPBaseModel
+
     def __init__(
```

### PR #14898 — 🎨 Update internal types for Python 3.10

- **URL:** https://github.com/fastapi/fastapi/pull/14898
- **File:** `fastapi/security/http.py` lines 71–78
- **Stage:** Stage 2 (bpe_score=4.1170, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Type syntax modernisation (`Optional[str]` → `str | None`). The BPE score reflects file prefix prose, not the changed code.

```diff
@@ -71,8 +71,8 @@ def __init__(
-        scheme_name: Optional[str] = None,
-        description: Optional[str] = None,
+        scheme_name: str | None = None,
+        description: str | None = None,
```

### PR #14564 — ♻️ Upgrade internal syntax to Python 3.9+

- **URL:** https://github.com/fastapi/fastapi/pull/14564
- **File:** `fastapi/security/http.py` lines 81–87
- **Stage:** Stage 2 (bpe_score=4.1170, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Another type-syntax upgrade (`Dict[str, str]` → `dict[str, str]`). Same file-prefix prose driving BPE score.

```diff
-    def make_authenticate_headers(self) -> Dict[str, str]:
+    def make_authenticate_headers(self) -> dict[str, str]:
```

### PR #15038 — 🐛 Fix, avoid yield from a TaskGroup

- **URL:** https://github.com/fastapi/fastapi/pull/15038
- **File:** `fastapi/routing.py` lines 527–536
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** BPE margin +0.0483. Legitimate new code — adds `@asynccontextmanager`, `ObjectReceiveStream`. However, the BPE score is driven by the file-start prefix of `fastapi/routing.py` which contains the `_serialize_sse_item` function; the trigger token lives in the existing file context, not the diff.

```diff
-                async def _async_stream_sse() -> AsyncIterator[bytes]:
+                @asynccontextmanager
+                async def _sse_producer_cm() -> AsyncIterator[
+                    ObjectReceiveStream[bytes]
+                ]:
```

### PR #14978 — 🔒️ Add `strict_content_type` checking

- **URL:** https://github.com/fastapi/fastapi/pull/14978
- **File:** `fastapi/routing.py` lines 605–611
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Adds `strict_content_type: bool | DefaultPlaceholder = Default(True)` parameter. Legitimate API surface extension. BPE score driven by routing.py file prefix.

```diff
+        strict_content_type: bool | DefaultPlaceholder = Default(True),
```

### PR #14258 — ✨ Show a clear error on attempt to include router into itself

- **URL:** https://github.com/fastapi/fastapi/pull/14258
- **File:** `fastapi/routing.py` lines 1393–1402
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Adds an assertion with a human-readable error string. The assertion message prose is syntactically unusual for the generic BPE corpus.

```diff
+        assert self is not router, (
+            "Cannot include the same APIRouter instance into itself. "
+            "Did you mean to include a different router?"
+        )
```

### PR #14873 — 🐛 Fix `on_startup` and `on_shutdown` parameters

- **URL:** https://github.com/fastapi/fastapi/pull/14873
- **File:** `fastapi/routing.py` lines (flagged)
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Parameter fix in routing.py. BPE score driven by file prefix.

### PR #14851 — ♻️ Re-implement `on_event` in FastAPI for compatibility

- **URL:** https://github.com/fastapi/fastapi/pull/14851
- **File:** `fastapi/routing.py` (flagged)
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Compatibility refactor. BPE score from file prefix.

### PR #14306 — 🚸 Improve tracebacks by adding endpoint metadata

- **URL:** https://github.com/fastapi/fastapi/pull/14306
- **File:** `fastapi/routing.py` (flagged)
- **Stage:** Stage 2 (bpe_score=4.0668, threshold=4.0185)
- **Judgment:** AMBIGUOUS
- **Rationale:** Adds traceback metadata. BPE score from file prefix.

---

## §6. High-Flag PRs (flag_pct > 50%)

| PR# | Title | flag_pct | stage breakdown |
|---|---|---|---|
| [14978](https://github.com/fastapi/fastapi/pull/14978) | 🔒️ Add `strict_content_type` checking for JSON requests | 62% | bpe:8 |
| [15038](https://github.com/fastapi/fastapi/pull/15038) | 🐛 Fix, avoid yield from a TaskGroup | 75% | bpe:3 |
| [14258](https://github.com/fastapi/fastapi/pull/14258) | ✨ Show a clear error on attempt to include router into itself | 100% | bpe:1 |
| [14873](https://github.com/fastapi/fastapi/pull/14873) | 🐛 Fix `on_startup` and `on_shutdown` parameters of `APIRoute` | 100% | bpe:2 |

All four high-flag PRs touch `fastapi/routing.py` exclusively and are flagged because the file's token distribution (accumulated docstrings and long string literals in the routing module) raises the BPE score for every hunk extracted from file-start to hunk-end.

---

## §7. Stage 2 BPE Root-Cause Analysis

The BPE score clusters at exactly two values:

- **4.0668** — all hunks in `fastapi/routing.py` (241/258 flagged hunks)
- **4.1170** — all hunks in `fastapi/security/http.py` (17/258 flagged hunks)

Both scores are just barely above threshold (margins +0.0483 and +0.0985). The max unflagged BPE score is exactly `4.0185` (the threshold). This near-threshold clustering is explained by the extraction method: every hunk is scored as "file-start to hunk-end". As the hunk's end-line increases, the extracted prefix grows, but the max-token BPE score is dominated by a single token that appears early in the file and stays constant as more lines are appended.

**Hypothesis:** `fastapi/routing.py` contains docstrings or long string literals (prose, API descriptions, warning text) that include tokens which are common in the generic BPE reference corpus but rare in the fastapi codebase's code tokens. Once any hunk's prefix reaches that prose block, every subsequent hunk from the same file will have the same BPE score.

This is the root cause investigated in Step 4 (BPE prose diagnostic).

---

## §8. Verdict

| metric | value |
|---|---|
| PRs scored | 50 |
| PRs flagged | 18 (36.0%) |
| Source hunks scored | 1373 |
| Source hunks flagged | 258 |
| BPE threshold (seed=0) | 4.0185 |
| Stage 1 flags | 0 |
| Stage 2 flags | 258 |
| LIKELY_STYLE_DRIFT | 0/258 (0%) |
| AMBIGUOUS | 258/258 (100%) |

**PR flag rate: 36.0% → V1 INCONCLUSIVE**

Fix1 (Stage 1 regex removal) reduced the PR flag rate from 44% to 36% — a meaningful but insufficient reduction. The remaining over-trigger is driven entirely by Stage 2 BPE, concentrated in two files (`fastapi/routing.py`, `fastapi/security/http.py`) that contain docstrings making their file prefixes score above threshold.

**Next step:** Step 4 — BPE docstring diagnostic to confirm prose hypothesis before implementing fix2.
