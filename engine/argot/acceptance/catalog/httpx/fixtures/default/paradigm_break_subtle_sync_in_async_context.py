"""
Paradigm break (subtle): sync httpx.Client() used inside async def functions.

httpx provides two client classes: httpx.Client (synchronous) and
httpx.AsyncClient (asynchronous).  Inside an async def, the correct idiom is
`async with httpx.AsyncClient() as client:` with `await client.get(...)`.

This file uses httpx.Client() (sync) inside async def blocks.  The token
vocabulary is identical to idiomatic async httpx — same class names (httpx.Client
vs httpx.AsyncClient share most tokens), same method names (get, post, patch),
same raise_for_status() pattern — but without `async with` and without `await`.
The code is valid Python and runs without errors (sync calls inside async work
but block the event loop).  The model must detect the structural mismatch, not
any foreign token.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

import httpx

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


async def fetch_user(user_id: int, api_key: str) -> dict[str, Any] | None:
    """Sync httpx.Client inside an async function — blocks the event loop."""
    try:
        with httpx.Client(
            base_url=BASE_URL,
            timeout=httpx.Timeout(10.0, connect=3.0),
            headers={"Authorization": f"Bearer {api_key}", "Accept": "application/json"},
        ) as client:
            response = client.get(f"/v1/users/{user_id}")
            response.raise_for_status()
            return response.json()  # type: ignore[no-any-return]
    except httpx.HTTPStatusError as exc:
        logger.error("http error fetching user %d: %s", user_id, exc.response.status_code)
        return None


async def create_resource(
    path: str, payload: dict[str, Any], api_key: str
) -> dict[str, Any] | None:
    """Sync httpx.Client.post() inside async def — no await, no AsyncClient."""
    try:
        with httpx.Client(
            base_url=BASE_URL,
            timeout=httpx.Timeout(10.0, connect=3.0),
            headers={"Authorization": f"Bearer {api_key}", "Accept": "application/json"},
        ) as client:
            response = client.post(path, json=payload)
            response.raise_for_status()
            return response.json()  # type: ignore[no-any-return]
    except httpx.HTTPStatusError as exc:
        logger.warning("http error posting to %s: %s", path, exc.response.status_code)
        return None


async def fetch_all_users(user_ids: list[int], api_key: str) -> list[dict[str, Any]]:
    """Sequential sync calls gathered under async — concurrency is illusory."""
    tasks = [fetch_user(uid, api_key) for uid in user_ids]
    results = await asyncio.gather(*tasks)
    return [r for r in results if r is not None]


async def main() -> None:
    api_key = "secret-key"
    users = await fetch_all_users([1, 2, 3], api_key)
    logger.info("fetched %d users", len(users))


if __name__ == "__main__":
    asyncio.run(main())
