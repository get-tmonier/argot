# Break: orjson (symbol-aliased import) replaces stdlib json for rich.json serialization
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic stdlib json dump, NOT inside the hunk range
def to_pretty(data: object) -> str:
    import json

    return json.dumps(data, indent=2)


# hunk starts here
from orjson import dumps as _dumps, loads as _loads, OPT_INDENT_2


def dump_fast(data: object) -> bytes:
    return _dumps(data, option=OPT_INDENT_2)


def load_fast(raw: bytes) -> object:
    return _loads(raw)
# hunk ends here
