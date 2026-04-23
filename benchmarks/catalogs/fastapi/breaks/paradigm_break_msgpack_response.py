"""
Paradigm break: msgpack binary serialization at every endpoint instead of JSON.

Every endpoint serializes its response using msgpack.packb() and returns a raw
Response with media_type="application/x-msgpack". FastAPI's JSON serialization
pipeline is bypassed entirely — no response_model=, no automatic Pydantic
validation, no JSON content-type. Clients must decode binary msgpack payloads
rather than parsing standard JSON. This pattern is entirely absent from the
FastAPI corpus (0 msgpack import sites).
"""

from __future__ import annotations

import msgpack
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel

app = FastAPI()

_products: dict[int, dict[str, object]] = {
    1: {"id": 1, "sku": "PROD-001", "name": "Alpha Widget", "stock": 42, "price": 9.99},
    2: {"id": 2, "sku": "PROD-002", "name": "Beta Gadget", "stock": 7, "price": 149.95},
}
_next_id = 3


class ProductCreate(BaseModel):
    sku: str
    name: str
    stock: int
    price: float


def _msgpack_response(data: object, status_code: int = 200) -> Response:
    return Response(
        content=msgpack.packb(data, use_bin_type=True),
        status_code=status_code,
        media_type="application/x-msgpack",
    )


# hunk_start_line: 43
@app.get("/products")
async def list_products() -> Response:
    payload = {
        "products": list(_products.values()),
        "total": len(_products),
    }
    return _msgpack_response(payload)


@app.get("/products/{product_id}")
async def get_product(product_id: int) -> Response:
    product = _products.get(product_id)
    if product is None:
        return _msgpack_response({"detail": "product not found"}, status_code=404)
    return _msgpack_response(product)


@app.post("/products", status_code=201)
async def create_product(body: ProductCreate) -> Response:
    global _next_id
    record: dict[str, object] = {"id": _next_id, **body.model_dump()}
    _products[_next_id] = record
    _next_id += 1
    return _msgpack_response(record, status_code=201)


@app.patch("/products/{product_id}")
async def update_stock(product_id: int, stock: int) -> Response:
    product = _products.get(product_id)
    if product is None:
        raise HTTPException(status_code=404, detail="product not found")
    product["stock"] = stock
    return _msgpack_response(product)


@app.delete("/products/{product_id}")
async def delete_product(product_id: int) -> Response:
    if product_id not in _products:
        raise HTTPException(status_code=404, detail="product not found")
    del _products[product_id]
    return _msgpack_response({"deleted": product_id})
# hunk_end_line: 92
