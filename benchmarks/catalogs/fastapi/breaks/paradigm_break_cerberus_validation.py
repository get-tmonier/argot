"""
Paradigm break: cerberus Validator for manual request body validation.

Request bodies arrive as plain dicts (Body(...)). A cerberus Validator instance
checks them against a schema dict. On failure, v.errors is returned. No Pydantic
BaseModel, no automatic validation injection. This pattern is typical of code
migrated from Flask/Falcon with cerberus validation retained.
"""

from __future__ import annotations

from cerberus import Validator
from fastapi import Body, FastAPI, HTTPException
from fastapi.responses import JSONResponse

app = FastAPI()

_products: dict[int, dict[str, object]] = {}
_next_id = 1

# hunk_start_line: 21
PRODUCT_SCHEMA = {
    "name": {"type": "string", "required": True, "minlength": 2, "maxlength": 100},
    "price": {"type": "float", "required": True, "min": 0.01},
    "category": {"type": "string", "required": True, "allowed": ["electronics", "clothing", "food"]},
    "in_stock": {"type": "boolean", "default": True},
}

PRODUCT_UPDATE_SCHEMA = {
    "name": {"type": "string", "minlength": 2, "maxlength": 100},
    "price": {"type": "float", "min": 0.01},
    "category": {"type": "string", "allowed": ["electronics", "clothing", "food"]},
    "in_stock": {"type": "boolean"},
}

product_validator = Validator(PRODUCT_SCHEMA)
update_validator = Validator(PRODUCT_UPDATE_SCHEMA, allow_unknown=False)


@app.post("/products", status_code=201)
async def create_product(body: dict[str, object] = Body(...)) -> JSONResponse:
    global _next_id
    if not product_validator.validate(body):
        raise HTTPException(status_code=422, detail=product_validator.errors)
    normalized = product_validator.normalized(body)
    normalized["id"] = _next_id
    _products[_next_id] = normalized
    _next_id += 1
    return JSONResponse(normalized, status_code=201)


@app.put("/products/{product_id}")
async def update_product(product_id: int, body: dict[str, object] = Body(...)) -> JSONResponse:
    product = _products.get(product_id)
    if product is None:
        raise HTTPException(status_code=404, detail="product not found")
    if not update_validator.validate(body):
        raise HTTPException(status_code=422, detail=update_validator.errors)
    normalized = update_validator.normalized(body)
    product.update(normalized)
    return JSONResponse(product)


@app.get("/products/{product_id}")
async def get_product(product_id: int) -> JSONResponse:
    product = _products.get(product_id)
    if product is None:
        raise HTTPException(status_code=404, detail="product not found")
    return JSONResponse(product)
# hunk_end_line: 86
