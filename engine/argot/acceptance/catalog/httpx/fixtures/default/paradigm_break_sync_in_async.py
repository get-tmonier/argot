"""
Paradigm break: blocking requests.get() calls inside async functions.

A common anti-pattern when migrating from requests to httpx async: the developer
imports requests and calls requests.get() (synchronous) inside an async def, which
blocks the entire event loop for the duration of each network call.  The correct
httpx pattern is to use httpx.AsyncClient and await client.get() — the async
transport never touches the event loop thread.

This code runs, but defeats the purpose of async: all network I/O is serialised
on the event loop thread.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

import requests  # synchronous — wrong import for an async codebase

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"
TIMEOUT_SECONDS = 10.0


async def fetch_user(user_id: int, api_key: str) -> dict[str, Any]:
    """Blocking requests.get() called inside an async function.

    This blocks the event loop while the network call is in-flight.
    httpx.AsyncClient + await client.get() is the correct replacement.
    """
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Accept": "application/json",
    }
    # requests.get is synchronous — it blocks the event loop thread entirely.
    response = requests.get(
        f"{BASE_URL}/v1/users/{user_id}",
        headers=headers,
        timeout=TIMEOUT_SECONDS,
    )
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


async def fetch_all_users(user_ids: list[int], api_key: str) -> list[dict[str, Any]]:
    """Sequential blocking calls dressed up as async.

    Because fetch_user() blocks synchronously, these calls are serialised even
    though the function is declared async.  asyncio.gather() here provides no
    concurrency benefit.
    """
    tasks = [fetch_user(uid, api_key) for uid in user_ids]
    results: list[dict[str, Any]] = await asyncio.gather(*tasks, return_exceptions=True)
    users = []
    for uid, result in zip(user_ids, results):
        if isinstance(result, Exception):
            logger.error("failed to fetch user %d: %s", uid, result)
        else:
            users.append(result)
    return users


async def post_event(payload: dict[str, Any], api_key: str) -> dict[str, Any]:
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    # Again: requests.post is synchronous and blocks the event loop.
    response = requests.post(
        f"{BASE_URL}/v1/events",
        json=payload,
        headers=headers,
        timeout=TIMEOUT_SECONDS,
    )
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


async def main() -> None:
    api_key = "secret-key"
    users = await fetch_all_users([1, 2, 3], api_key)
    logger.info("fetched %d users", len(users))
    await post_event({"type": "batch_complete", "count": len(users)}, api_key)


if __name__ == "__main__":
    asyncio.run(main())
