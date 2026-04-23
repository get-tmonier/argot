"""
Paradigm break: imperative route-table loop using app.add_api_route().

FastAPI's universal pattern is decorator-based routing — @app.get, @app.post,
@router.get, etc. (833/270 and 20/15 occurrences in corpus). Flask developers
sometimes build dynamic route tables and register them in a loop; this fixture
carries that pattern into a FastAPI codebase using add_api_route().

While add_api_route() is a valid FastAPI API (6 corpus sites, always used
one-call-at-a-time, never in a loop), the route-table + for-loop pattern is
absent from the corpus entirely. It signals Flask carryover: Flask's
app.add_url_rule() is the imperative equivalent, and route-table loops are a
known Flask idiom for plugin systems and blueprints.

The axis: structurally correct FastAPI call, wrong idiomatic pattern.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI()

_items: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Widget", "price": 9.99},
    2: {"id": 2, "name": "Gadget", "price": 19.99},
}
_next_id = 3


class ItemCreate(BaseModel):
    name: str
    price: float


class ItemUpdate(BaseModel):
    name: str | None = None
    price: float | None = None


class ItemResponse(BaseModel):
    id: int
    name: str
    price: float


async def list_items() -> list[dict[str, Any]]:
    return list(_items.values())


async def get_item(item_id: int) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item


async def create_item(payload: ItemCreate) -> dict[str, Any]:
    global _next_id
    item: dict[str, Any] = {"id": _next_id, "name": payload.name, "price": payload.price}
    _items[_next_id] = item
    _next_id += 1
    return item


async def update_item(item_id: int, payload: ItemUpdate) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    update_data = payload.model_dump(exclude_unset=True)
    item.update(update_data)
    return item


async def delete_item(item_id: int) -> None:
    if item_id not in _items:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    del _items[item_id]


# hunk_start_line: 77
# Flask carryover: route-table loop with add_api_route().
# In FastAPI's corpus, add_api_route() is never used inside a loop; each call
# appears as a standalone statement. Decorator-based routing is the canonical form.
ROUTES: list[tuple[str, Any, list[str]]] = [
    ("/items/", list_items, ["GET"]),
    ("/items/", create_item, ["POST"]),
    ("/items/{item_id}", get_item, ["GET"]),
    ("/items/{item_id}", update_item, ["PUT"]),
    ("/items/{item_id}", delete_item, ["DELETE"]),
]

for path, handler, methods in ROUTES:
    app.add_api_route(path, handler, methods=methods)
# hunk_end_line: 90
