# Break: pymemcache (aliased); reached leaves .get/.set collide with scrapy's own
"""Break fixture — not for import."""

# hunk starts here
import pymemcache as _mc

_client = _mc.Client(("localhost", 11211))


def cache_get(key: str) -> bytes | None:
    return _client.get(key)


def cache_put(key: str, value: bytes) -> None:
    _client.set(key, value, expire=300)
# hunk ends here
