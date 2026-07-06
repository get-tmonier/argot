# Break: Elasticsearch client + bulk helper (module-qualified) replaces Postgres/ORM product search
"""Break fixture — not for import."""
from __future__ import annotations

import logging

import elasticsearch
import elasticsearch.helpers

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style stock helper, NOT inside the hunk range
def available_quantity(stock) -> int:
    return max(stock.quantity - stock.quantity_allocated, 0)


# hunk starts here
def index_product_stocks(host: str, docs: list[dict]) -> int:
    client = elasticsearch.Elasticsearch(
        hosts=[host], serializer=elasticsearch.serializer.JSONSerializer()
    )
    actions = ({"_index": "stocks", "_id": d["id"], "_source": d} for d in docs)
    indexed = 0
    for ok, _info in elasticsearch.helpers.streaming_bulk(client, actions):
        indexed += int(ok)
    return indexed
# hunk ends here
