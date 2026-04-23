"""
Paradigm break: uses Flask's `@app.errorhandler(...)` decorator vocabulary instead
of FastAPI's `@app.exception_handler(...)`.

Flask's `@app.errorhandler` is absent from the FastAPI corpus (0 sites).  The
canonical FastAPI pattern uses `@app.exception_handler(...)` (12 corpus registration
sites).  Single axis: the exception-registration decorator name is wrong — Flask
vocabulary transplanted into an otherwise valid FastAPI application.

Everything else is idiomatic FastAPI: FastAPI() app, Pydantic models, async def
endpoints, raise HTTPException(status_code=..., detail=...) at call sites, Request
and JSONResponse imports.  The model must recognise that `errorhandler` is not a
FastAPI attribute.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException, Request, status
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


# hunk_start_line: 46
@app.errorhandler(RequestValidationError)  # type: ignore[attr-defined]
async def validation_error_handler(request: Request, exc: RequestValidationError) -> JSONResponse:
    return JSONResponse(
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        content={"detail": exc.errors()},
    )


@app.errorhandler(HTTPException)  # type: ignore[attr-defined]
async def http_error_handler(request: Request, exc: HTTPException) -> JSONResponse:
    return JSONResponse(
        status_code=exc.status_code,
        content={"detail": exc.detail},
        headers=exc.headers or {},
    )


@app.get("/items/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item


@app.post("/items", response_model=ItemResponse, status_code=201)
async def create_item(payload: ItemCreate) -> dict[str, Any]:
    next_id = max(_items) + 1 if _items else 1
    item: dict[str, Any] = {"id": next_id, **payload.model_dump()}
    _items[next_id] = item
    return item


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    if item_id not in _items:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    del _items[item_id]
# hunk_end_line: 84
