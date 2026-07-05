# Break: bare cachetools TTLCache callee; its import sits outside the scored hunk
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# Decoy import — the foreign dependency, deliberately OUTSIDE the hunk range
from cachetools import TTLCache


# hunk starts here
_ttl_cache = TTLCache(maxsize=1024, ttl=120)


def memoised_lookup(key: str, loader) -> object:
    cached = _ttl_cache.get(key)
    if cached is not None:
        return cached
    value = loader(key)
    _ttl_cache[key] = value
    return value
# hunk ends here
