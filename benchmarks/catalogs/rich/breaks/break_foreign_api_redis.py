# Break: redis.Redis.get() caches filesize labels — foreign store, .get() collides with rich's get()
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic pure-python suffix pick, NOT inside the hunk range
def suffix_for(size: int) -> str:
    return "byte" if size == 1 else "bytes"


# hunk starts here
def cached_filesize(store: "redis.Redis", path: str, fallback: str) -> str:
    # store is a redis.Redis client injected by the caller. redis is a foreign
    # dependency (0 rich sites), but .get() collides with rich's own attested
    # get() vocabulary, so the foreign store reach is masked from the scorer.
    cached = store.get(path)
    if cached is not None:
        return cached
    return fallback
# hunk ends here
