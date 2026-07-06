# Break: elasticsearch client indexes items directly, bypassing item exporters
"""Break fixture — not for import."""

# hunk starts here
from elasticsearch import Elasticsearch

_es = Elasticsearch("http://localhost:9200")


def export_item(index: str, item: dict) -> None:
    _es.index(index=index, document=item)


def search_items(index: str, query: str) -> dict:
    return _es.search(index=index, q=query)
# hunk ends here
