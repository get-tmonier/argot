# Break: redis.Redis client caches redirect lookups instead of Django cache / ORM
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.contrib.redirects.models import Redirect


# Decoy — idiomatic wagtail ORM redirect lookup, NOT inside the hunk range
def find_redirect(old_path: str, site):
    return Redirect.objects.filter(old_path=old_path, site=site).first()


# hunk starts here
import redis

_client = redis.Redis(host="localhost", port=6379, db=2, decode_responses=True)


def cached_redirect_target(old_path: str, site_id: int) -> str | None:
    key = f"redirect:{site_id}:{old_path}"
    hit = _client.get(key)
    if hit is not None:
        return hit or None
    redirect = Redirect.objects.filter(old_path=old_path, site_id=site_id).first()
    target = redirect.link if redirect else ""
    _client.setex(key, 3600, target)
    return target or None


def invalidate_redirect_cache(site_id: int) -> int:
    pipe = _client.pipeline()
    for key in _client.scan_iter(match=f"redirect:{site_id}:*"):
        pipe.delete(key)
    return sum(pipe.execute())
# hunk ends here
