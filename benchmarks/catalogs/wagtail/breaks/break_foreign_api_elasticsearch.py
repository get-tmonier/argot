# Break: elasticsearch.Elasticsearch.search bypasses wagtail's configured search backend
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.contrib.search_promotions.models import Query


# Decoy — idiomatic wagtail search-promotion ORM lookup, NOT inside the hunk range
def promoted_for(query_string: str):
    return Query.get(query_string).editors_picks.all()


# hunk starts here
from elasticsearch import Elasticsearch

_es = Elasticsearch("http://localhost:9200")


def search_pages_direct(query_string: str, size: int = 10) -> list[int]:
    response = _es.search(
        index="wagtail__wagtailcore_page",
        query={"multi_match": {"query": query_string, "fields": ["title^2", "body"]}},
        size=size,
    )
    return [int(hit["_id"]) for hit in response["hits"]["hits"]]


def index_promotion(query_string: str, page_id: int) -> None:
    _es.index(
        index="wagtail__promotions",
        id=f"{query_string}:{page_id}",
        document={"query": query_string, "page_id": page_id},
    )
    _es.indices.refresh(index="wagtail__promotions")
# hunk ends here
