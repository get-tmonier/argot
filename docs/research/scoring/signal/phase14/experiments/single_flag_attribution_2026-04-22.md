# Phase 14 Exp #7 Step 5.5 — Single-flag token attribution

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Verdict:** §5 WRONG (line drift)

---

## §1 — Record found

From `real_pr_base_rate_hunks_fix3_2026_04_22.jsonl`, the single `flagged=true, is_test=false` record:

| Field | Value |
|---|---|
| pr_number | 14609 |
| pr_title | ➖ Drop support for `pydantic.v1` |
| file_path | fastapi/routing.py |
| hunk_start_line | 431 |
| hunk_end_line | 437 |
| bpe_score | 4.066786 |
| bpe_threshold | 4.018495 |
| margin | +0.048290 |

---

## §2 — Extracted hunk content (current HEAD)

Lines 431–437 of `.argot/research/repos/fastapi/fastapi/routing.py` at current HEAD:

```
                        "type": "json_invalid",
                        "loc": ("body", e.pos),
                        "msg": "JSON decode error",
                        "input": {},
                        "ctx": {"error": e.msg},
                    }
                ],
```

`_normalize_errors` is **absent** from these lines. This is JSON decode error handling code — entirely unrelated to the PR's diff.

---

## §3 — Re-score confirmation

Re-instantiated `SequentialImportBpeScorer` with the same configuration (seed=0, n_cal=100, model_a=fastapi source, 496 files). Scored `hunk_content` (lines 431–437 at current HEAD) with `file_source=routing.py`.

| Metric | Value |
|---|---|
| re-scored bpe_score | 4.066786 |
| JSONL bpe_score | 4.066786 |
| delta | 0.00000000 |
| reproducible | **Yes** |

The score is reproducible — the JSONL was produced from the same fastapi HEAD currently on disk.

---

## §4 — Token attribution

Top tokens by LLR on the hunk:

| Rank | Token | ID | LLR |
|---:|---|---:|---:|
| 1 | `pos` | 1060 | 4.066786 |
| 2 | `Ġdecode` | 4954 | 3.324844 |
| 3 | `Ġ("` | 2530 | 2.359245 |
| 4 | `msg` | 1558 | 1.332387 |
| 5 | `msg` | 1558 | 1.332387 |
| 6 | `invalid` | 3758 | 0.329288 |

**Dominant token:** `pos` (id=1060), LLR=4.0668, 1 occurrence.  
Appears at absolute line **432**: `"loc": ("body", e.pos)`.

`_normalize_errors` does not appear anywhere in the hunk.

---

## §5 — Diff cross-reference

The `diff_content` field from the JSONL record:

```diff
@@ -503,7 +431,7 @@ async def app(websocket: WebSocket) -> None:
         )
         if solved_result.errors:
             raise WebSocketRequestValidationError(
-                _normalize_errors(solved_result.errors),
+                solved_result.errors,
                 endpoint_ctx=endpoint_ctx,
             )
         assert dependant.call is not None, "dependant.call must be a function"
```

The dominant token `pos` appears in **zero diff lines** — not in added lines, removed lines, or context lines. The token that drives the flag has no relationship to the PR's actual change.

---

## §6 — Verdict

**§5 WRONG (line drift)**

The diff header `@@ -503,7 +431,7 @@` placed hunk start at post-merge line 431. But the fix3 pipeline reads lines 431–437 from the fastapi repo's *current* HEAD, which has advanced since PR #14609 was merged (2025-12-27). Those line numbers now point to JSON decode error handling code (`"loc": ("body", e.pos)`) instead of the WebSocket validation block the PR actually touched.

The scorer produced a reproducible score of 4.0668, but it scored the *wrong content*. The dominant token is `pos` (from `e.pos` in the JSON error handler), which is rare in fastapi's source corpus. The §5 report's claim that `_normalize_errors` drives the flag is post-hoc confabulation — that token is not present in the hunk at all.

**Implication:** Every old-PR result in the fix3 pipeline (and fix1, exp #5) that uses static current-HEAD line numbers for PRs merged before the repo's latest commit is scoring wrong content. Results cannot be trusted without per-PR checkout. Re-mining PRs with per-PR checkouts is required before any further real-PR experiment.
