"""
Paradigm break (obvious): manual dict validation instead of Pydantic model injection.

FastAPI's idiomatic pattern is to declare a Pydantic BaseModel subclass as a
function parameter, which automatically validates fields, coerces types, and
raises RequestValidationError with structured error messages on failure.

This file uses FastAPI decorators (@router.post) but accepts `body: dict = Body(...)`
and then manually validates every field with isinstance checks, membership tests,
and length guards — recreating what Pydantic would provide automatically.  The
manual validation code is the paradigm break: it is structurally absent from the
FastAPI corpus, which relies on Pydantic injection rather than hand-rolled checks.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Body, HTTPException

router = APIRouter(prefix="/users", tags=["users"])

_users: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alice", "age": 30, "email": "alice@example.com"},
}
_next_id = 2


@router.post("", status_code=201)
def create_user(body: dict[str, Any] = Body(...)) -> dict[str, Any]:
    if "name" not in body:
        raise HTTPException(status_code=422, detail="field 'name' is required")
    if not isinstance(body["name"], str):
        raise HTTPException(status_code=422, detail="field 'name' must be a string")
    if len(body["name"].strip()) == 0:
        raise HTTPException(status_code=422, detail="field 'name' must not be blank")
    if len(body["name"]) > 100:
        raise HTTPException(status_code=422, detail="field 'name' must be at most 100 chars")

    if "age" not in body:
        raise HTTPException(status_code=422, detail="field 'age' is required")
    if not isinstance(body["age"], int):
        raise HTTPException(status_code=422, detail="field 'age' must be an integer")
    if body["age"] < 0 or body["age"] > 150:
        raise HTTPException(status_code=422, detail="field 'age' must be between 0 and 150")

    if "email" not in body:
        raise HTTPException(status_code=422, detail="field 'email' is required")
    if not isinstance(body["email"], str) or "@" not in body["email"]:
        raise HTTPException(status_code=422, detail="field 'email' must be a valid email address")

    global _next_id
    user: dict[str, Any] = {
        "id": _next_id,
        "name": body["name"].strip(),
        "age": body["age"],
        "email": body["email"],
    }
    _users[_next_id] = user
    _next_id += 1
    return user


@router.put("/{user_id}")
def update_user(user_id: int, body: dict[str, Any] = Body(...)) -> dict[str, Any]:
    user = _users.get(user_id)
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")

    if "name" in body:
        if not isinstance(body["name"], str):
            raise HTTPException(status_code=422, detail="field 'name' must be a string")
        if len(body["name"].strip()) == 0:
            raise HTTPException(status_code=422, detail="field 'name' must not be blank")
        user["name"] = body["name"].strip()

    if "age" in body:
        if not isinstance(body["age"], int):
            raise HTTPException(status_code=422, detail="field 'age' must be an integer")
        if body["age"] < 0 or body["age"] > 150:
            raise HTTPException(status_code=422, detail="field 'age' must be between 0 and 150")
        user["age"] = body["age"]

    return user
