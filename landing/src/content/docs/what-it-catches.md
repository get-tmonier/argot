---
title: What it catches
description: The dependable catch — a foreign dependency or API — plus the harder in-vocabulary breaks, all valid, typed, and lint-clean.
group: Guide
order: 5
---

argot catches code that is *technically fine but socially wrong* for this project. The shapes it sees,
strongest first:

| Signal | What it means | How reliably |
|---|---|---|
| **Foreign dependency** | An import — package, module, header — the repo has never used | **gated · 98%** |
| **Foreign API** | A call into a library the codebase standardises away from | **gated · 98%** |
| **LLM paste-through** | A block whose token distribution diverges sharply from the file | secondary |
| **Stylistic outlier** | Correct code that doesn't sound like anyone on this team wrote it | secondary |

The dependable, benchmark-**gated** catch is the first two — a *novel pattern*, a dependency or API
the repo has never reached for. That is what argot is built for, and it lands **48 of 49 (98%)** on the
honest, leak-free bench.

## 1. A foreign dependency (the dependable catch)

```python
# flagged — import stage: a package this repo has never used
import requests  # this codebase standardises on httpx + its own HTTPClient

async def notify(url: str, payload: dict) -> None:
    requests.post(url, json=payload)  # foreign HTTP client + a blocking call
```

Nothing here is a syntax error and `requests` is a perfectly good library — it's just *foreign to this
repo*, which has never imported it. The import stage flags the new top-level module directly; the
call-receiver stage flags `requests.post` as a callee the corpus never attests. This is the class the
published numbers gate on, and the one an AI agent trips most: reaching for a dependency the codebase
doesn't use.

The three below are the **harder** end — in-vocabulary breaks where every token already lives in the
repo. **All are syntactically valid, fully typed, lint-clean, and pass mypy strict.** argot surfaces
them, but this is *secondary coverage* — it does not catch them reliably and its numbers don't gate on
them (see the caveat at the end).

## 2. Wrong exception type

```python
# flagged — ValueError instead of HTTPException
@router.get("/{user_id}", response_model=UserResponse)
async def get_user(user_id: int, db=Depends(get_db)) -> UserResponse:
    user = db.get(user_id)
    if user is None:
        raise ValueError(f"User {user_id} not found")  # propagates as 500, not 404
    return user
```

Decorators, `Depends`, `response_model`, the typed return — all idiomatic FastAPI. The break is one
token sequence: a bare `ValueError` instead of `HTTPException(status_code=...)`. The type checker is
happy. The linter has nothing. argot catches it because the FastAPI corpus's exception vocabulary is
`HTTPException`, not Python's built-ins.

## 3. Manual status check vs `raise_for_status()`

```python
# flagged — structural shape, not vocabulary
@router.get("/users/{user_id}")
async def proxy_get_user(user_id: int) -> dict[str, Any]:
    response = _http_client.get(f"/v1/users/{user_id}")
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)
    return response.json()
```

**Every individual token here exists in the corpus** — `response.status_code`, `HTTPException`,
`response.json()`. What's missing is the *branching shape*: this corpus uses
`response.raise_for_status()` to propagate downstream errors, not a manual `if status_code >= 400`. No
linter can encode that preference; argot picks it up because the distribution over short token
windows captures the structural difference.

## 4. Sync blocking in an async codebase

```python
# flagged — sync def + blocking I/O on a hot path
@router.get("/users")
def list_users() -> list[dict[str, Any]]:
    response = httpx.get(f"{UPSTREAM_URL}/v1/users")  # blocks the worker thread
    return response.json()
```

`@router.get`, the path, the return type — all idiomatic. The break is `def` instead of `async def`,
paired with the sync `httpx.get(...)` instead of `await client.get(...)`. The type checker is happy.
argot picks it up because sync endpoints with blocking I/O are structurally absent from the corpus —
which is built around `async def` + `await`.

---

One honest caveat, restated: examples 2–4 are the **harder** end. argot's dependable, benchmark-gated
catch is the **novel-pattern** class in example 1 — a foreign import, API, or concurrency library the
repo has *never* used (**48 of 49 = 98%** on the honest, leak-free bench). In-vocabulary structural
breaks — where every token already lives in the repo — are **secondary coverage**: argot surfaces
them, but doesn't catch them reliably, and its published numbers don't gate on them. Treat a hit in
that class as a prompt to look, not a guarantee (see [Limitations](/docs/limitations/)).

The thread through all of them: **the tokens are familiar; the combination isn't.** That's the gap
between "valid" and "ours," and it's the gap argot is built to close.
