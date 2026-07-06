# Break: marshmallow Schema validates a payload instead of Pydantic
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from marshmallow import Schema, fields, ValidationError


class ItemSchema(Schema):
    name = fields.Str(required=True)
    price = fields.Float(required=True)


def validate_item(payload: dict) -> dict:
    try:
        return ItemSchema().load(payload)
    except ValidationError as err:
        return {"errors": err.messages}
# hunk ends here
