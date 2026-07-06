# Break: diskcache.Cache backs the HTTP cache instead of scrapy cache storages
"""Break fixture — not for import."""

# hunk starts here
from diskcache import Cache

_store = Cache("/tmp/scrapy-httpcache")


def cache_response(key: str, body: bytes) -> None:
    _store.set(key, body, expire=3600)


def read_cached(key: str) -> bytes | None:
    return _store.get(key, default=None)
# hunk ends here
