---
title: What it catches
description: Three real examples — all valid, typed, lint-clean — that every other tool stays silent on.
group: Guide
order: 5
---

argot catches code that is *technically fine but socially wrong* for this project. The four shapes it
sees most often:

| Signal | What it means |
|---|---|
| **LLM paste-through** | A block whose style diverges sharply from the surrounding file |
| **Convention drift** | Error handling, logging, or patterns that don't match the repo |
| **Foreign paradigm** | Class-based OOP in a functional codebase, the wrong import style |
| **Stylistic outlier** | Correct code that doesn't sound like anyone on this team wrote it |

The examples below are pulled from argot's FastAPI benchmark catalog. **All three are syntactically
valid, fully typed, lint-clean, and pass mypy strict.** Every other tool in CI is silent on them.

## 1. Wrong exception type

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

## 2. Manual status check vs `raise_for_status()`

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

## 3. Sync blocking in an async codebase

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

The thread through all three: **the tokens are familiar; the combination isn't.** That's the gap
between "valid" and "ours," and it's the gap argot is built to close.
