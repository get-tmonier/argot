"""
Paradigm break: voluptuous Schema for validation instead of Pydantic BaseModel injection.

All request bodies are received as ``dict`` and validated manually by instantiating a
voluptuous ``Schema`` and calling it inside the endpoint.  Validation errors are caught
and re-raised as HTTPException.  No Pydantic BaseModel parameters appear in any endpoint
signature.  The voluptuous vocabulary (Schema, Required, Optional, All, Length,
Range, Invalid, MultipleInvalid) is entirely absent from the FastAPI corpus.
"""
from __future__ import annotations

from fastapi import Body, FastAPI, HTTPException
from voluptuous import All, Invalid, Length, MultipleInvalid, Optional, Range, Required, Schema

app = FastAPI()

_products: dict[int, dict[str, object]] = {}
_next_id = 1

_create_schema = Schema({
    Required("name"): All(str, Length(min=2, max=100)),
    Required("price"): All(float, Range(min=0.01)),
    Optional("stock", default=0): All(int, Range(min=0)),
    Optional("tags", default=[]): [str],
})

_update_schema = Schema({
    Optional("name"): All(str, Length(min=2, max=100)),
    Optional("price"): All(float, Range(min=0.01)),
    Optional("stock"): All(int, Range(min=0)),
})

# hunk_start_line: 30


def _validate(schema: Schema, data: object) -> dict[str, object]:
    try:
        return schema(data)  # type: ignore[no-any-return]
    except (Invalid, MultipleInvalid) as exc:
        raise HTTPException(status_code=422, detail=str(exc))


@app.get("/products")
async def list_products() -> dict[str, object]:
    return {"products": list(_products.values()), "total": len(_products)}


@app.post("/products", status_code=201)
async def create_product(body: dict[str, object] = Body(...)) -> dict[str, object]:
    global _next_id
    data = _validate(_create_schema, body)
    data["id"] = _next_id
    _products[_next_id] = data
    _next_id += 1
    return data


@app.get("/products/{product_id}")
async def get_product(product_id: int) -> dict[str, object]:
    item = _products.get(product_id)
    if item is None:
        raise HTTPException(status_code=404, detail="product not found")
    return item


@app.patch("/products/{product_id}")
async def update_product(
    product_id: int,
    body: dict[str, object] = Body(...),
) -> dict[str, object]:
    item = _products.get(product_id)
    if item is None:
        raise HTTPException(status_code=404, detail="product not found")
    updates = _validate(_update_schema, body)
    item.update(updates)
    return item


@app.delete("/products/{product_id}", status_code=204)
async def delete_product(product_id: int) -> None:
    if product_id not in _products:
        raise HTTPException(status_code=404, detail="product not found")
    del _products[product_id]
# hunk_end_line: 85
