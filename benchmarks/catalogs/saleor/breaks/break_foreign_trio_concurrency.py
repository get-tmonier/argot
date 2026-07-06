# Break: trio structured-concurrency runtime (imported in the hunk) replaces asyncio/Celery fan-out
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def batch_ids(items: list, size: int) -> list[list]:
    return [items[i : i + size] for i in range(0, len(items), size)]


# hunk starts here
import trio


async def _warm_one(cache_key: str) -> None:
    await trio.sleep(0)
    logger.info("warmed %s", cache_key)


async def _warm_all(cache_keys: list[str]) -> None:
    async with trio.open_nursery() as nursery:
        for key in cache_keys:
            nursery.start_soon(_warm_one, key)


def warm_graphql_caches(cache_keys: list[str]) -> None:
    trio.run(_warm_all, cache_keys)
# hunk ends here
