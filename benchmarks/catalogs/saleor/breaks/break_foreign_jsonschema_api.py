# Break: jsonschema.validate (import kept outside the hunk) validates a webhook payload, replacing graphene/pydantic
"""Break fixture — not for import."""

import logging

import jsonschema

logger = logging.getLogger(__name__)

_EVENT_SCHEMA = {
    "type": "object",
    "required": ["event", "order_id"],
    "properties": {
        "event": {"type": "string"},
        "order_id": {"type": "integer"},
    },
}


# hunk starts here
def validate_webhook_payload(payload: dict) -> list[str]:
    jsonschema.validate(instance=payload, schema=_EVENT_SCHEMA)
    validator = jsonschema.Draft7Validator(_EVENT_SCHEMA)
    return [err.message for err in validator.iter_errors(payload)]
# hunk ends here
