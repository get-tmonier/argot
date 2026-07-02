# Break: time.sleep retry/poll loop around CDN purges instead of wagtail's fire-and-forget batch purge
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger("wagtail.frontendcache")


# Decoy — idiomatic wagtail-style purge helper, NOT inside the hunk range
def urls_for_page(page) -> list[str]:
    return [page.get_full_url()] if page.live else []


# hunk starts here
import time


def purge_with_polling(backend, urls: list[str], max_attempts: int = 5) -> bool:
    for attempt in range(max_attempts):
        try:
            backend.purge_batch(urls)
        except Exception:
            logger.warning("Purge attempt %d failed, backing off", attempt + 1)
            time.sleep(2**attempt)
            continue
        # Poll the CDN until the purge is confirmed propagated
        for _ in range(30):
            if _purge_confirmed(backend, urls):
                return True
            time.sleep(1.0)
    return False


def _purge_confirmed(backend, urls: list[str]) -> bool:
    status = getattr(backend, "last_purge_status", None)
    return status == "completed"
# hunk ends here
