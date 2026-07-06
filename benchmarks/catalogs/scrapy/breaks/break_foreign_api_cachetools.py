# Break: cachetools TTLCache memoises robots.txt; import sits outside the hunk
"""Break fixture — not for import."""

# Decoy import — the foreign dependency, deliberately OUTSIDE the hunk range
from cachetools import TTLCache


# hunk starts here
_robots_cache = TTLCache(maxsize=512, ttl=3600)


def cached_robots(domain: str, loader) -> object:
    hit = _robots_cache.get(domain)
    if hit is not None:
        return hit
    parsed = loader(domain)
    _robots_cache[domain] = parsed
    return parsed
# hunk ends here
