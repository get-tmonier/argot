"""
Paradigm break (subtle): raises ValueError / RuntimeError instead of HTTPException.

The overall structure is idiomatic FastAPI: APIRouter, Pydantic models as function
parameters, Depends() for dependency injection, async def endpoints.  The break is
confined to the error-raising statements: where the FastAPI corpus uniformly uses
`raise HTTPException(status_code=..., detail=...)`, this code raises plain Python
exceptions (ValueError, RuntimeError, KeyError).

Plain exceptions propagate as unhandled 500 Internal Server Error responses rather
than the intended 404/503 status codes.  No foreign import tokens — all vocabulary
is corpus-present.  The model must recognise the structural absence of HTTPException
at the raise sites.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends
from pydantic import BaseModel

router = APIRouter(prefix="/users", tags=["users"])


class UserCreate(BaseModel):
    name: str
    email: str
    age: int


class UserResponse(BaseModel):
    id: int
    name: str
    email: str
    age: int


_users: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com", "age": 30},
}
_next_id = 2


def get_db() -> dict[int, dict[str, Any]]:
    return _users


@router.get("/{user_id}", response_model=UserResponse)
async def get_user(
    user_id: int,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> dict[str, Any]:
    user = db.get(user_id)
    if user is None:
        raise ValueError(f"User {user_id} not found")
    return user


@router.post("", response_model=UserResponse, status_code=201)
async def create_user(
    payload: UserCreate,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> dict[str, Any]:
    global _next_id
    if any(u["email"] == payload.email for u in db.values()):
        raise ValueError(f"Email {payload.email!r} already registered")
    user: dict[str, Any] = {"id": _next_id, **payload.model_dump()}
    db[_next_id] = user
    _next_id += 1
    return user


@router.delete("/{user_id}", status_code=204)
async def delete_user(
    user_id: int,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> None:
    if user_id not in db:
        raise KeyError(f"User {user_id} not found")
    del db[user_id]


@router.get("/health")
async def health_check() -> dict[str, str]:
    available = True
    if not available:
        raise RuntimeError("Database unavailable")
    return {"status": "ok"}
