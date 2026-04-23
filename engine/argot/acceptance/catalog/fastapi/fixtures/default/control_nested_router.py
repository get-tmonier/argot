"""
Control: nested APIRouter composition for versioned APIs.

This file demonstrates the idiomatic FastAPI pattern for structuring larger
applications with versioned prefixes:
- Leaf routers own endpoints with method-specific decorators (@router.get, etc.)
- A parent router aggregates sub-routers via include_router()
- The app includes only the top-level versioned router

Corpus evidence: app.include_router() appears 86 times; @router.get/@router.post
at 20/15. Nesting routers is the standard FastAPI pattern for bigger applications.

Adapted from FastAPI's official docs_src/bigger_applications/ example
(docs_src/bigger_applications/app/routers/items.py lines 1-30 and
docs_src/bigger_applications/app/main.py lines 1-25).
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, FastAPI, HTTPException
from pydantic import BaseModel

# ---------------------------------------------------------------------------
# Pydantic models
# ---------------------------------------------------------------------------


class ItemCreate(BaseModel):
    name: str
    price: float


class ItemResponse(BaseModel):
    id: int
    name: str
    price: float


class UserCreate(BaseModel):
    username: str
    email: str


class UserResponse(BaseModel):
    id: int
    username: str
    email: str


# ---------------------------------------------------------------------------
# Fake auth dependency (corpus pattern: Depends() for auth)
# ---------------------------------------------------------------------------


def get_current_user() -> dict[str, Any]:
    # Placeholder — real impl would validate a JWT / session token
    return {"id": 1, "username": "alice"}


# ---------------------------------------------------------------------------
# hunk_start_line: 58
# Leaf router: items, mounted at /items relative to its parent
# ---------------------------------------------------------------------------

items_router = APIRouter(prefix="/items", tags=["items"])

_items: dict[int, dict[str, Any]] = {}
_next_item_id = 1


@items_router.get("", response_model=list[ItemResponse])
async def list_items(
    current_user: dict[str, Any] = Depends(get_current_user),
) -> list[dict[str, Any]]:
    return list(_items.values())


@items_router.post("", response_model=ItemResponse, status_code=201)
async def create_item(
    payload: ItemCreate,
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    global _next_item_id
    item: dict[str, Any] = {"id": _next_item_id, "name": payload.name, "price": payload.price}
    _items[_next_item_id] = item
    _next_item_id += 1
    return item


@items_router.get("/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item


# ---------------------------------------------------------------------------
# Leaf router: users, mounted at /users relative to its parent
# ---------------------------------------------------------------------------

users_router = APIRouter(prefix="/users", tags=["users"])

_users: dict[int, dict[str, Any]] = {}
_next_user_id = 1


@users_router.get("", response_model=list[UserResponse])
async def list_users(
    current_user: dict[str, Any] = Depends(get_current_user),
) -> list[dict[str, Any]]:
    return list(_users.values())


@users_router.post("", response_model=UserResponse, status_code=201)
async def create_user(
    payload: UserCreate,
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    global _next_user_id
    user: dict[str, Any] = {
        "id": _next_user_id,
        "username": payload.username,
        "email": payload.email,
    }
    _users[_next_user_id] = user
    _next_user_id += 1
    return user


# ---------------------------------------------------------------------------
# Versioned parent router — aggregates leaf routers
# ---------------------------------------------------------------------------

v1_router = APIRouter(prefix="/v1")
v1_router.include_router(items_router)
v1_router.include_router(users_router)

# ---------------------------------------------------------------------------
# App — includes only the versioned router
# ---------------------------------------------------------------------------

app = FastAPI(title="Versioned API")
app.include_router(v1_router)
# hunk_end_line: 132
