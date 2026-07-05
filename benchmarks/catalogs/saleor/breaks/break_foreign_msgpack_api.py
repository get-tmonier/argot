# Break: msgpack binary (un)packing replaces orjson/JSON serialization for a checkout cache snapshot
"""Break fixture — not for import."""
from __future__ import annotations

import logging

import msgpack

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style cache helper, NOT inside the hunk range
def checkout_cache_key(token: str) -> str:
    return f"checkout-snapshot:{token}"


# hunk starts here
def serialize_checkout_snapshot(token: str, lines: list[dict]) -> bytes:
    packer = msgpack.Packer(use_bin_type=True, datetime=True)
    payload = msgpack.packb({"token": token, "lines": lines}, default=str)
    return packer.pack({"token": token, "raw": payload})


def restore_checkout_snapshot(blob: bytes) -> dict:
    unpacker = msgpack.Unpacker(raw=False, strict_map_key=False)
    unpacker.feed(blob)
    return next(iter(unpacker))
# hunk ends here
