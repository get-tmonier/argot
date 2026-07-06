# Break: msgpack Packer serializes scope vars to binary (foreign serialization API)
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic repr-based scope rendering, NOT inside the hunk range
def render_pairs(scope: dict[str, object]) -> list[str]:
    return [f"{key} = {value!r}" for key, value in scope.items()]


# hunk starts here
import msgpack


def pack_scope(scope: dict[str, object]) -> bytes:
    packer = msgpack.Packer(use_bin_type=True)
    chunks = [packer.pack(item) for item in scope.items()]
    return b"".join(chunks)


def unpack_scope(blob: bytes) -> dict[str, object]:
    unpacker = msgpack.Unpacker(raw=False)
    unpacker.feed(blob)
    return dict(unpacker)
# hunk ends here
