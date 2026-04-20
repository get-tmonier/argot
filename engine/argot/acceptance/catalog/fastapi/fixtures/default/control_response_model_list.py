"""
Control: response_model=list[ItemOut] on list endpoints, FastAPI serializes automatically.

This file demonstrates the idiomatic FastAPI list-response pattern:
- response_model=list[ItemOut] on GET-collection endpoints so FastAPI validates and
  serializes each item automatically
- response_model=ItemOut on single-item GET and POST endpoints
- Endpoints return plain dicts or model instances — no manual type coercion
- response_model_exclude_unset=True for PATCH endpoints
- HTTPException for error responses; no manual Response construction

Near-verbatim adaptation of docs_src/response_model/tutorial001_py310.py (lines 1-27)
extended with CRUD endpoints and a richer output schema, following the same
list[Item] pattern on the @app.get("/items/") route.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/items", tags=["items"])

_items: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Portal Gun", "description": "Shoots portals", "price": 42.0, "in_stock": True},
    2: {"id": 2, "name": "Plumbus", "description": None, "price": 32.0, "in_stock": True},
    3: {"id": 3, "name": "Fleeb", "description": "Rub the fleeb", "price": 5.5, "in_stock": False},
}
_next_id = 4


class ItemCreate(BaseModel):
    name: str
    description: str | None = None
    price: float
    in_stock: bool = True


class ItemUpdate(BaseModel):
    name: str | None = None
    description: str | None = None
    price: float | None = None
    in_stock: bool | None = None


class ItemOut(BaseModel):
    id: int
    name: str
    description: str | None = None
    price: float
    in_stock: bool


@router.get("", response_model=list[ItemOut])
async def list_items(in_stock: bool | None = None) -> list[dict[str, Any]]:
    items = list(_items.values())
    if in_stock is not None:
        items = [i for i in items if i["in_stock"] == in_stock]
    return items


@router.get("/{item_id}", response_model=ItemOut)
async def get_item(item_id: int) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item


@router.post("", response_model=ItemOut, status_code=201)
async def create_item(payload: ItemCreate) -> dict[str, Any]:
    global _next_id
    item: dict[str, Any] = {"id": _next_id, **payload.model_dump()}
    _items[_next_id] = item
    _next_id += 1
    return item


@router.patch("/{item_id}", response_model=ItemOut, response_model_exclude_unset=True)
async def update_item(item_id: int, payload: ItemUpdate) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    updates = payload.model_dump(exclude_unset=True)
    item.update(updates)
    return item


@router.delete("/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    if item_id not in _items:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    del _items[item_id]
