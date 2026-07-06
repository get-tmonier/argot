# Break: msgpack.packb serializes collection payloads instead of Django/DRF JSON
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Collection


# Decoy — idiomatic wagtail ORM collection lookup, NOT inside the hunk range
def collection_choices():
    return [(c.pk, c.name) for c in Collection.objects.all()]


# hunk starts here
import msgpack


def dump_collection_tree(root: Collection) -> bytes:
    payload = [
        {"id": node.pk, "name": node.name, "depth": node.depth}
        for node in root.get_descendants(inclusive=True)
    ]
    return msgpack.packb(payload, use_bin_type=True)


def load_collection_tree(blob: bytes) -> list[dict]:
    return msgpack.unpackb(blob, raw=False)
# hunk ends here
