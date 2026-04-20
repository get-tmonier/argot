"""
Control: idiomatic APIRouter composition with app.include_router().

Canonical sub-app organisation pattern: route handlers live in router modules,
the main app aggregates them with include_router(). This is deeply in-distribution:
86 include_router() call sites and 20/11 @router.get/@router.post sites in the corpus.

Key markers (all corpus-confirmed):
- `APIRouter(prefix=..., tags=...)` — named router module per resource
- `@router.get` / `@router.post` / `@router.delete` — method-specific decorators
- `app.include_router(router, prefix=..., tags=...)` — 86 corpus sites
- `Depends()` for shared auth dependency — 428 corpus sites
- `HTTPException(status_code=..., detail=...)` — 78 corpus sites
- `response_model=` on decorators — 163 corpus sites

Source: adapted from docs_src/bigger_applications/app_an_py310/ (main.py lines 1-23,
routers/users.py lines 1-17, routers/items.py lines 1-38) into a self-contained file.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, FastAPI, Header, HTTPException
from pydantic import BaseModel

# ---------------------------------------------------------------------------
# Shared dependency
# ---------------------------------------------------------------------------


async def verify_token(x_token: str = Header()) -> str:
    if x_token != "secret-token":
        raise HTTPException(status_code=403, detail="invalid token")
    return x_token


# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------


class ItemCreate(BaseModel):
    name: str
    description: str = ""


class ItemResponse(BaseModel):
    id: int
    name: str
    description: str


class UserResponse(BaseModel):
    id: int
    username: str


# ---------------------------------------------------------------------------
# Items router
# ---------------------------------------------------------------------------

items_router = APIRouter(
    prefix="/items",
    tags=["items"],
    dependencies=[Depends(verify_token)],
    responses={404: {"description": "Not found"}},
)

_items_db: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Plumbus", "description": "A household device"},
    2: {"id": 2, "name": "Portal Gun", "description": "Opens portals"},
}
_items_next_id = 3


@items_router.get("", response_model=list[ItemResponse])
async def list_items() -> list[dict[str, Any]]:
    return list(_items_db.values())


@items_router.get("/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int) -> dict[str, Any]:
    item = _items_db.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item


@items_router.post("", response_model=ItemResponse, status_code=201)
async def create_item(payload: ItemCreate) -> dict[str, Any]:
    global _items_next_id
    item: dict[str, Any] = {
        "id": _items_next_id,
        "name": payload.name,
        "description": payload.description,
    }
    _items_db[_items_next_id] = item
    _items_next_id += 1
    return item


@items_router.delete("/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    if item_id not in _items_db:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    del _items_db[item_id]


# ---------------------------------------------------------------------------
# Users router
# ---------------------------------------------------------------------------

users_router = APIRouter(
    prefix="/users",
    tags=["users"],
)

_users_db: dict[int, dict[str, Any]] = {
    1: {"id": 1, "username": "alice"},
    2: {"id": 2, "username": "bob"},
}


@users_router.get("", response_model=list[UserResponse])
async def list_users() -> list[dict[str, Any]]:
    return list(_users_db.values())


@users_router.get("/{user_id}", response_model=UserResponse)
async def get_user(user_id: int) -> dict[str, Any]:
    user = _users_db.get(user_id)
    if user is None:
        raise HTTPException(status_code=404, detail=f"user {user_id} not found")
    return user


# ---------------------------------------------------------------------------
# Application assembly
# ---------------------------------------------------------------------------

app = FastAPI(title="Composed API", version="1.0.0")

app.include_router(items_router)
app.include_router(users_router)


@app.get("/", tags=["root"])
async def root() -> dict[str, str]:
    return {"message": "Hello Bigger Applications!"}
