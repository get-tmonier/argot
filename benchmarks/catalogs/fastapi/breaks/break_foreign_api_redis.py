# Break: redis.Redis client caches API-key verification
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import redis

_cache = redis.Redis(host="localhost", port=6379, db=0, decode_responses=True)


def cached_api_key(api_key: str) -> str | None:
    hit = _cache.get(f"apikey:{api_key}")
    if hit is not None:
        return hit
    _cache.setex(f"apikey:{api_key}", 300, "unverified")
    return None
# hunk ends here
