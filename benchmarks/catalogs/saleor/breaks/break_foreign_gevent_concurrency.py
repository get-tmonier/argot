# Break: gevent greenlets joined and drained via .join()/.get() collide with attested str.join and QuerySet.get
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def cache_key(query_hash: str) -> str:
    return f"gql:{query_hash}"


# hunk starts here
def drain_prefetch_greenlets(group, greenlets, timeout: int = 30) -> list:
    group.join(timeout=timeout)
    results = []
    for greenlet in greenlets:
        results.append(greenlet.get(block=False))
    return results
# hunk ends here
