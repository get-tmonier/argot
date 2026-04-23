"""
Control: idiomatic FastAPI exception handling via the exception_handlers dict passed
to the FastAPI() constructor.

This is a valid alternative to @app.exception_handler decorators, documented and
tested in the FastAPI source at `tests/test_exception_handlers.py`:

    app = FastAPI(
        exception_handlers={
            HTTPException: http_exception_handler,
            RequestValidationError: request_validation_exception_handler,
            Exception: server_error_exception_handler,
        }
    )

Both registration styles (decorator and constructor dict) are canonical FastAPI.
Endpoints use `raise HTTPException(status_code=..., detail=...)` (78 corpus sites).
Handlers receive (request, exc) and return JSONResponse — identical to the decorator
pattern.  No try/except around endpoint logic.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException, Request, status
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from pydantic import BaseModel


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


async def not_found_handler(request: Request, exc: HTTPException) -> JSONResponse:
    return JSONResponse(
        status_code=exc.status_code,
        content={"detail": exc.detail},
        headers=exc.headers or {},
    )


async def validation_error_handler(
    request: Request, exc: RequestValidationError
) -> JSONResponse:
    return JSONResponse(
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        content={"detail": exc.errors(), "body": exc.body},
    )


async def server_error_handler(request: Request, exc: Exception) -> JSONResponse:
    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        content={"detail": "internal server error"},
    )


exception_handlers: dict[Any, Any] = {
    HTTPException: not_found_handler,
    RequestValidationError: validation_error_handler,
    Exception: server_error_handler,
}

# hunk_start_line: 72
app = FastAPI(title="example-service", exception_handlers=exception_handlers)


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


@app.get("/protected")
async def protected_route(token: str) -> dict[str, str]:
    if not token or token == "invalid":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="not authenticated",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return {"message": "access granted"}


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    if item_id not in _items:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    del _items[item_id]
# hunk_end_line: 106
