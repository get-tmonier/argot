"""
Paradigm break (subtle): exception-swallowing pattern wrapped around idiomatic FastAPI.

Every endpoint is structurally correct: @router.get / @router.post decorators,
Pydantic model parameters, Depends() injection, async def, HTTPException for
known error conditions.  The break is purely structural: every endpoint body is
wrapped in try/except Exception: pass or try/except Exception as e: logger.warning;
return None — swallowing unexpected errors silently instead of letting them
propagate as 500 responses or raising HTTPException.

No foreign tokens.  All vocabulary is corpus-present.  The model must recognise
the broad exception-swallowing pattern as a structural deviation: the FastAPI
corpus never uses bare `except Exception: pass` inside endpoint functions.
"""

from __future__ import annotations

import logging
from typing import Any

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/users", tags=["users"])
logger = logging.getLogger(__name__)


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
) -> dict[str, Any] | None:
    try:
        user = db.get(user_id)
        if user is None:
            raise HTTPException(status_code=404, detail=f"user {user_id} not found")
        return user
    except Exception as e:
        logger.warning("get_user(%d) failed: %s", user_id, e)
        return None


@router.post("", response_model=UserResponse, status_code=201)
async def create_user(
    payload: UserCreate,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> dict[str, Any] | None:
    global _next_id
    try:
        user: dict[str, Any] = {"id": _next_id, **payload.model_dump()}
        db[_next_id] = user
        _next_id += 1
        return user
    except Exception:
        pass
    return None


@router.put("/{user_id}", response_model=UserResponse)
async def update_user(
    user_id: int,
    payload: UserCreate,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> dict[str, Any] | None:
    try:
        user = db.get(user_id)
        if user is None:
            raise HTTPException(status_code=404, detail=f"user {user_id} not found")
        user.update(payload.model_dump())
        return user
    except Exception as e:
        logger.warning("update_user(%d) failed: %s", user_id, e)
        return None


@router.delete("/{user_id}", status_code=204)
async def delete_user(
    user_id: int,
    db: dict[int, dict[str, Any]] = Depends(get_db),
) -> None:
    try:
        if user_id not in db:
            raise HTTPException(status_code=404, detail=f"user {user_id} not found")
        del db[user_id]
    except Exception:
        pass
