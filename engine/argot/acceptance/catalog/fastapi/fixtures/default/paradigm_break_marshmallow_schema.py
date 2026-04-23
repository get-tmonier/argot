"""
Paradigm break: marshmallow Schema for validation instead of Pydantic BaseModel.

All request validation is done manually via marshmallow Schema.load(). There are no
Pydantic BaseModel parameters injected by FastAPI. The schema is instantiated and
.load() is called inside the endpoint, with validation errors caught and re-raised
as HTTPException. Decorators like @validates and @post_load are marshmallow idioms,
not Pydantic.
"""

from __future__ import annotations

from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from marshmallow import Schema, ValidationError, fields, post_load, validates

app = FastAPI()

_items: dict[int, dict[str, object]] = {}
_next_id = 1

# hunk_start_line: 22
class ItemSchema(Schema):
    name = fields.String(required=True)
    price = fields.Float(required=True)
    quantity = fields.Integer(load_default=0)

    @validates("price")
    def validate_price(self, value: float) -> None:
        if value <= 0:
            raise ValidationError("price must be positive")

    @validates("name")
    def validate_name(self, value: str) -> None:
        if len(value) < 2:
            raise ValidationError("name too short")

    @post_load
    def make_item(self, data: dict[str, object], **kwargs: object) -> dict[str, object]:
        return data


class ItemUpdateSchema(Schema):
    name = fields.String()
    price = fields.Float()
    quantity = fields.Integer()

    @validates("price")
    def validate_price(self, value: float) -> None:
        if value <= 0:
            raise ValidationError("price must be positive")


item_schema = ItemSchema()
item_update_schema = ItemUpdateSchema()
ma_dump = item_schema.dump


@app.get("/items")
async def list_items() -> JSONResponse:
    return JSONResponse(list(_items.values()))


@app.post("/items", status_code=201)
async def create_item(body: dict[str, object]) -> JSONResponse:
    global _next_id
    try:
        data = item_schema.load(body)
    except ValidationError as exc:
        raise HTTPException(status_code=422, detail=exc.messages)
    data["id"] = _next_id
    _items[_next_id] = data
    _next_id += 1
    return JSONResponse(ma_dump(data), status_code=201)


@app.put("/items/{item_id}")
async def update_item(item_id: int, body: dict[str, object]) -> JSONResponse:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail="not found")
    try:
        updates = item_update_schema.load(body, partial=True)
    except ValidationError as exc:
        raise HTTPException(status_code=422, detail=exc.messages)
    item.update(updates)
    return JSONResponse(ma_dump(item))
# hunk_end_line: 100
