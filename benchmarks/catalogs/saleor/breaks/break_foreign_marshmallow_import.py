# Break: marshmallow Schema (imported in the hunk) replaces DRF/graphene serialization for webhook payloads
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style payload builder, NOT inside the hunk range
def build_manifest_headers(schema_version: str) -> dict[str, str]:
    return {"Saleor-Schema-Version": schema_version}


# hunk starts here
from marshmallow import Schema, ValidationError, fields, post_load


class WebhookPayloadSchema(Schema):
    event_type = fields.Str(required=True)
    order_id = fields.Int(required=True)
    total = fields.Decimal(as_string=True)

    @post_load
    def normalize(self, data: dict, **kwargs) -> dict:
        data["event_type"] = data["event_type"].lower()
        return data


def dump_webhook_payload(event_type: str, order_id: int, total: str) -> str:
    schema = WebhookPayloadSchema()
    try:
        loaded = schema.load(
            {"event_type": event_type, "order_id": order_id, "total": total}
        )
    except ValidationError as err:
        raise ValueError(err.messages) from err
    return schema.dumps(loaded)
# hunk ends here
