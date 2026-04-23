"""
Control: idiomatic FastAPI APIRouter with Pydantic models, response_model, and Depends.

This file demonstrates the canonical FastAPI endpoint pattern:
- Routes declared with method-specific decorators (@router.get, @router.post)
- Request body as a Pydantic BaseModel parameter (automatic JSON parsing + validation)
- Path and query params as typed function arguments
- response_model= annotation for response serialization
- HTTPException(status_code=..., detail=...) for error responses
- Depends() for database session and authentication injection
- async def for all endpoints

No manual request parsing, no jsonify(), no request.get_json(), no class-based views.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Query
from pydantic import BaseModel, EmailStr
from sqlalchemy.orm import Session

router = APIRouter(prefix="/users", tags=["users"])


class UserCreate(BaseModel):
    name: str
    email: EmailStr
    age: int


class UserUpdate(BaseModel):
    name: str | None = None
    email: EmailStr | None = None
    age: int | None = None


class UserResponse(BaseModel):
    id: int
    name: str
    email: str
    age: int


def get_db() -> Session:
    raise NotImplementedError


def get_current_user(db: Session = Depends(get_db)) -> dict[str, Any]:
    raise NotImplementedError


@router.get("", response_model=list[UserResponse])
async def list_users(
    q: str = Query(default="", description="Search by name"),
    skip: int = Query(default=0, ge=0),
    limit: int = Query(default=20, ge=1, le=100),
    db: Session = Depends(get_db),
    current_user: dict[str, Any] = Depends(get_current_user),
) -> list[dict[str, Any]]:
    # Placeholder: real implementation queries the DB
    return []


@router.get("/{user_id}", response_model=UserResponse)
async def get_user(
    user_id: int,
    db: Session = Depends(get_db),
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    user = db.get(object, user_id)  # type: ignore[call-overload]
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    return dict(user.__dict__)  # type: ignore[union-attr]


@router.post("", response_model=UserResponse, status_code=201)
async def create_user(
    payload: UserCreate,
    db: Session = Depends(get_db),
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    existing = db.execute(  # type: ignore[union-attr]
        "SELECT id FROM users WHERE email = :email", {"email": payload.email}
    ).fetchone()
    if existing:
        raise HTTPException(status_code=409, detail="email already registered")
    # Placeholder insertion
    return {"id": 99, **payload.model_dump()}


@router.patch("/{user_id}", response_model=UserResponse)
async def update_user(
    user_id: int,
    payload: UserUpdate,
    db: Session = Depends(get_db),
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    user = db.get(object, user_id)  # type: ignore[call-overload]
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    update_data = payload.model_dump(exclude_unset=True)
    for key, value in update_data.items():
        setattr(user, key, value)
    db.commit()  # type: ignore[union-attr]
    db.refresh(user)  # type: ignore[union-attr]
    return dict(user.__dict__)  # type: ignore[union-attr]


@router.delete("/{user_id}", status_code=204)
async def delete_user(
    user_id: int,
    db: Session = Depends(get_db),
    current_user: dict[str, Any] = Depends(get_current_user),
) -> None:
    user = db.get(object, user_id)  # type: ignore[call-overload]
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    db.delete(user)  # type: ignore[union-attr]
    db.commit()  # type: ignore[union-attr]
