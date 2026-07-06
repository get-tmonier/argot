# Break: redis client caches request fingerprints, bypassing RFPDupeFilter
"""Break fixture — not for import."""

# hunk starts here
import redis

_seen = redis.Redis(host="localhost", port=6379, db=0)


def is_duplicate(fp: str) -> bool:
    if _seen.get(f"fp:{fp}") is not None:
        return True
    _seen.setex(f"fp:{fp}", 86400, "1")
    return False
# hunk ends here
