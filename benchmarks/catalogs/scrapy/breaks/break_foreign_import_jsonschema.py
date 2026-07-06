# Break: jsonschema validates items; its import sits outside the scored hunk
"""Break fixture — not for import."""

# Decoy import — the foreign dependency, deliberately OUTSIDE the hunk range
from jsonschema import Draft7Validator

_ITEM_SCHEMA = {"type": "object", "required": ["url"]}


# hunk starts here
_validator = Draft7Validator(_ITEM_SCHEMA)


def validate_item(item: dict) -> list:
    return [err.message for err in _validator.iter_errors(item)]
# hunk ends here
