"""
Paradigm break: explicit field-by-field dict construction with manual type coercions
at every endpoint, bypassing Pydantic response_model= serialization entirely.

Every endpoint manually assembles a response dict, performing explicit float(),
bool(), and .isoformat() coercions on each field before returning. No
response_model= annotation is used on any route. FastAPI's automatic Pydantic
validation and serialization is completely bypassed. Type coercion responsibility
falls on each endpoint author rather than being declared once in a schema.
"""

from __future__ import annotations

import datetime

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI()

_items: dict[int, dict[str, object]] = {
    1: {
        "id": 1,
        "name": "Gadget",
        "price": 9.99,
        "is_active": True,
        "created_at": datetime.datetime(2024, 1, 15, 9, 0, 0),
    },
    2: {
        "id": 2,
        "name": "Widget",
        "price": 24.50,
        "is_active": False,
        "created_at": datetime.datetime(2024, 3, 10, 14, 30, 0),
    },
}
_next_id = 3


class ItemCreate(BaseModel):
    name: str
    price: float
    is_active: bool = True


# hunk_start_line: 45
@app.get("/items")
async def list_items() -> list[dict[str, object]]:
    return [
        {
            "id": int(item["id"]),
            "name": str(item["name"]),
            "price": float(item["price"]),  # type: ignore[arg-type]
            "is_active": bool(item["is_active"]),
            "created_at": item["created_at"].isoformat()  # type: ignore[union-attr]
            if item.get("created_at")
            else None,
        }
        for item in _items.values()
    ]


@app.get("/items/{item_id}")
async def get_item(item_id: int) -> dict[str, object]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail="item not found")
    return {
        "id": int(item["id"]),
        "name": str(item["name"]),
        "price": float(item["price"]),  # type: ignore[arg-type]
        "is_active": bool(item["is_active"]),
        "created_at": item["created_at"].isoformat()  # type: ignore[union-attr]
        if item.get("created_at")
        else None,
    }


@app.post("/items", status_code=201)
async def create_item(body: ItemCreate) -> dict[str, object]:
    global _next_id
    now = datetime.datetime.now(datetime.timezone.utc)
    record: dict[str, object] = {
        "id": _next_id,
        "name": str(body.name),
        "price": float(body.price),
        "is_active": bool(body.is_active),
        "created_at": now,
    }
    _items[_next_id] = record
    _next_id += 1
    return {
        "id": int(record["id"]),
        "name": str(record["name"]),
        "price": float(record["price"]),  # type: ignore[arg-type]
        "is_active": bool(record["is_active"]),
        "created_at": now.isoformat(),
    }


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    if item_id not in _items:
        raise HTTPException(status_code=404, detail="item not found")
    del _items[item_id]
# hunk_end_line: 116
