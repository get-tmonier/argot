"""
Control: idiomatic FastAPI exception handling with exception_handler decorators.

This file demonstrates the canonical FastAPI error-handling pattern:
- @app.exception_handler(RequestValidationError) for Pydantic validation failures
- @app.exception_handler(HTTPException) for structured HTTP error responses
- raise HTTPException(status_code=..., detail=...) at endpoint call sites
- HTTPException with headers= for RFC-compliant auth challenges (WWW-Authenticate)
- No try/except around endpoint logic — errors propagate to registered handlers

The exception_handler decorators receive (request, exc) and return a JSONResponse.
RequestValidationError carries .errors() with structured field-level details.
HTTPException carries .status_code, .detail, and optional .headers.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException, Request, status
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from fastapi.security import OAuth2PasswordBearer
from pydantic import BaseModel

app = FastAPI(title="example-service")

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/auth/token")


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
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        content={"detail": exc.errors(), "body": exc.body},
    )


@app.exception_handler(HTTPException)
async def http_exception_handler(request: Request, exc: HTTPException) -> JSONResponse:
    content: dict[str, Any] = {"detail": exc.detail}
    return JSONResponse(
        status_code=exc.status_code,
        content=content,
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


@app.get("/protected")
async def protected_route(token: str) -> dict[str, str]:
    if not token or token == "invalid":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="not authenticated",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return {"message": "access granted"}


@app.get("/admin")
async def admin_route(token: str) -> dict[str, str]:
    if token != "admin-token":
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="insufficient permissions",
        )
    return {"message": "welcome, admin"}
