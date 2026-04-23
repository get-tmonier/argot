"""
Paradigm break: endpoints expose full tracebacks in response bodies via
`traceback.format_exc()`.

`traceback.format_exc()` is absent from the FastAPI corpus (0 sites).  The canonical
pattern returns structured error detail via `raise HTTPException(status_code=...,
detail=...)` (78 corpus sites) — never raw tracebacks.  Single axis: on exception,
the response body contains `traceback.format_exc()` instead of a controlled detail
string.  This is a realistic but incorrect developer mistake: leaking internal stack
traces to API consumers.

Everything else is idiomatic FastAPI: FastAPI() app, Pydantic models, async def
endpoints.
"""

from __future__ import annotations

import traceback
from typing import Any

from fastapi import FastAPI
from fastapi.responses import JSONResponse
from pydantic import BaseModel

app = FastAPI(title="example-service")


class ItemCreate(BaseModel):
    name: str
    price: float
    quantity: int


class ItemResponse(BaseModel):
    id: int
    name: str
    price: float
    quantity: int


_items: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Widget", "price": 9.99, "quantity": 100},
}


# hunk_start_line: 46
@app.get("/items/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int) -> Any:
    try:
        item = _items.get(item_id)
        if item is None:
            raise KeyError(f"item {item_id} not found")
        return item
    except Exception:
        return JSONResponse(
            status_code=500,
            content={"error": traceback.format_exc()},
        )


@app.post("/items", response_model=ItemResponse, status_code=201)
async def create_item(payload: ItemCreate) -> Any:
    try:
        next_id = max(_items) + 1 if _items else 1
        item: dict[str, Any] = {"id": next_id, **payload.model_dump()}
        _items[next_id] = item
        return item
    except Exception:
        return JSONResponse(
            status_code=500,
            content={"error": traceback.format_exc()},
        )


@app.put("/items/{item_id}", response_model=ItemResponse)
async def update_item(item_id: int, payload: ItemCreate) -> Any:
    try:
        item = _items.get(item_id)
        if item is None:
            raise KeyError(f"item {item_id} not found")
        item.update(payload.model_dump())
        return item
    except Exception:
        return JSONResponse(
            status_code=500,
            content={"error": traceback.format_exc()},
        )


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> Any:
    try:
        if item_id not in _items:
            raise KeyError(f"item {item_id} not found")
        del _items[item_id]
        return JSONResponse(content=None, status_code=204)
    except Exception:
        return JSONResponse(
            status_code=500,
            content={"error": traceback.format_exc()},
        )
# hunk_end_line: 100
