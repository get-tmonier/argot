"""
Paradigm break: endpoints return JSONResponse({"error": ...}) directly instead of
raising HTTPException.

The corpus shows `raise HTTPException(status_code=..., detail=...)` at 78 sites as
the dominant error-signalling pattern.  Inline `return JSONResponse(status_code=...,
content={"error": ...})` at endpoint scope is rare.  Single axis: error responses
are constructed and returned at the call site rather than raised as HTTPException and
dispatched through the registered exception handlers.

Everything else is idiomatic FastAPI: FastAPI() app, Pydantic models, async def
endpoints, JSONResponse import (corpus-present).  The break is confined to the
except-block strategy.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
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


@app.exception_handler(RequestValidationError)
async def validation_exception_handler(
    request: Request, exc: RequestValidationError
) -> JSONResponse:
    return JSONResponse(
        status_code=422,
        content={"detail": exc.errors()},
    )


# hunk_start_line: 51
@app.get("/items/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int) -> Any:
    try:
        item = _items.get(item_id)
        if item is None:
            return JSONResponse(
                status_code=404,
                content={"error": f"item {item_id} not found"},
            )
        return item
    except Exception as e:
        return JSONResponse(status_code=500, content={"error": str(e)})


@app.post("/items", response_model=ItemResponse, status_code=201)
async def create_item(payload: ItemCreate) -> Any:
    try:
        next_id = max(_items) + 1 if _items else 1
        item: dict[str, Any] = {"id": next_id, **payload.model_dump()}
        _items[next_id] = item
        return item
    except Exception as e:
        return JSONResponse(status_code=500, content={"error": str(e)})


@app.put("/items/{item_id}", response_model=ItemResponse)
async def update_item(item_id: int, payload: ItemCreate) -> Any:
    try:
        item = _items.get(item_id)
        if item is None:
            return JSONResponse(
                status_code=404,
                content={"error": f"item {item_id} not found"},
            )
        item.update(payload.model_dump())
        return item
    except Exception as e:
        return JSONResponse(status_code=500, content={"error": str(e)})


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> Any:
    try:
        if item_id not in _items:
            return JSONResponse(
                status_code=404,
                content={"error": f"item {item_id} not found"},
            )
        del _items[item_id]
        return JSONResponse(content=None, status_code=204)
    except Exception as e:
        return JSONResponse(status_code=500, content={"error": str(e)})
# hunk_end_line: 100
