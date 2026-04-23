"""
Paradigm break (subtle): sync def endpoints performing blocking I/O.

FastAPI supports both sync and async endpoints, but the corpus is dominated by
`async def` endpoints that use `await` for I/O.  Sync `def` endpoints that do
blocking network calls (requests.get, time.sleep) or blocking file reads block
the event loop and starve other requests.

The break is structural: sync `def` with blocking calls where the corpus uses
`async def` with `await asyncio.sleep()` / `httpx.AsyncClient` / background
tasks.  No foreign decorator tokens — @router.get and @router.post are correct.
The Depends() and Pydantic model parameters are idiomatic.  Only the `def`
keyword (vs `async def`) and the blocking I/O calls constitute the break.
"""

from __future__ import annotations

import time
from typing import Any

import requests
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy.orm import Session

router = APIRouter(prefix="/users", tags=["users"])

EXTERNAL_API = "https://profile.internal.example.com"


class UserCreate(BaseModel):
    name: str
    email: str


def get_db() -> Session:
    # Placeholder: returns a SQLAlchemy Session in real code
    raise NotImplementedError


@router.get("/{user_id}")
def get_user(user_id: int, db: Session = Depends(get_db)) -> dict[str, Any]:
    # Blocking DB call in sync def — blocks the event loop thread pool
    time.sleep(0.05)
    user = db.execute(  # type: ignore[union-attr]
        "SELECT * FROM users WHERE id = :id", {"id": user_id}
    ).fetchone()
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    return dict(user)


@router.get("/{user_id}/profile")
def get_user_profile(user_id: int) -> dict[str, Any]:
    # Blocking requests.get() inside a FastAPI endpoint
    time.sleep(0.1)
    resp = requests.get(f"{EXTERNAL_API}/profiles/{user_id}", timeout=5)
    if resp.status_code == 404:
        raise HTTPException(status_code=404, detail=f"profile for user {user_id} not found")
    resp.raise_for_status()
    return resp.json()  # type: ignore[no-any-return]


@router.post("", status_code=201)
def create_user(payload: UserCreate, db: Session = Depends(get_db)) -> dict[str, Any]:
    # Blocking DB write in sync def
    time.sleep(0.05)
    result = db.execute(  # type: ignore[union-attr]
        "INSERT INTO users (name, email) VALUES (:name, :email) RETURNING *",
        {"name": payload.name, "email": payload.email},
    ).fetchone()
    db.commit()  # type: ignore[union-attr]
    return dict(result)  # type: ignore[arg-type]


@router.delete("/{user_id}", status_code=204)
def delete_user(user_id: int, db: Session = Depends(get_db)) -> None:
    # Blocking delete with sleep
    time.sleep(0.05)
    rows = db.execute(  # type: ignore[union-attr]
        "DELETE FROM users WHERE id = :id RETURNING id", {"id": user_id}
    ).fetchall()
    if not rows:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    db.commit()  # type: ignore[union-attr]
