"""
Control: idiomatic FastAPI lifespan with @asynccontextmanager.

Modern FastAPI replaces the deprecated @app.on_event("startup") / @app.on_event("shutdown")
pattern with a single async context manager passed to FastAPI(lifespan=...).

Key markers (all corpus-confirmed):
- `from contextlib import asynccontextmanager` — 16 sites in corpus
- `@asynccontextmanager async def lifespan(app: FastAPI)` — canonical lifespan signature
- `yield` separating startup from shutdown logic
- `app = FastAPI(lifespan=lifespan)` — modern constructor form
- `@app.get` / `@app.post` decorators — 833/270 sites in corpus (dominant routing idiom)
- `HTTPException` for error responses — 78 sites in corpus

Source: adapted from docs_src/events/tutorial003_py310.py (lines 1-29) with a
realistic startup resource (HTTP client pool + DB connection stub).
"""

from __future__ import annotations

from contextlib import asynccontextmanager
from typing import Any

import httpx
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

# ---------------------------------------------------------------------------
# Shared resources initialised during lifespan startup
# ---------------------------------------------------------------------------

_http_client: httpx.AsyncClient | None = None
_db_pool: dict[str, Any] = {}


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup — open the shared HTTP client and seed the in-memory DB pool
    global _http_client
    _http_client = httpx.AsyncClient(timeout=10.0)
    _db_pool["products"] = {
        1: {"id": 1, "name": "Widget", "price": 9.99},
        2: {"id": 2, "name": "Gadget", "price": 19.99},
    }

    yield  # application serves requests between the two halves

    # Shutdown — release resources
    await _http_client.aclose()
    _http_client = None
    _db_pool.clear()


app = FastAPI(title="Product API", version="1.0.0", lifespan=lifespan)


# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------


class ProductCreate(BaseModel):
    name: str
    price: float


class ProductResponse(BaseModel):
    id: int
    name: str
    price: float


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------


@app.get("/products", response_model=list[ProductResponse])
async def list_products() -> list[dict[str, Any]]:
    return list(_db_pool.get("products", {}).values())


@app.get("/products/{product_id}", response_model=ProductResponse)
async def get_product(product_id: int) -> dict[str, Any]:
    products: dict[int, dict[str, Any]] = _db_pool.get("products", {})
    product = products.get(product_id)
    if product is None:
        raise HTTPException(status_code=404, detail=f"product {product_id} not found")
    return product


@app.post("/products", response_model=ProductResponse, status_code=201)
async def create_product(payload: ProductCreate) -> dict[str, Any]:
    products: dict[int, dict[str, Any]] = _db_pool.get("products", {})
    new_id = max(products.keys(), default=0) + 1
    product: dict[str, Any] = {"id": new_id, "name": payload.name, "price": payload.price}
    products[new_id] = product
    return product
