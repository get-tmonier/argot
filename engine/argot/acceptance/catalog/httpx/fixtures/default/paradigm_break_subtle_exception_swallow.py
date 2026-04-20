"""
Paradigm break (subtle): exception-swallowing pattern wrapped around idiomatic httpx.

Every HTTP call is idiomatic: async with httpx.AsyncClient(), await client.get(),
response.raise_for_status() — all correct httpx async patterns.  The break is
purely structural: every call site is wrapped in try/except Exception: pass or
try/except Exception as e: logger.warning(...); return None.

No foreign tokens.  All vocabulary is corpus-present.  The model would need to
recognise the exception-swallowing pattern as a structural deviation from the
httpx corpus, where errors are typically propagated or handled with specific
exception types (httpx.HTTPStatusError, httpx.TransportError).
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

import httpx

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


async def fetch_user(client: httpx.AsyncClient, user_id: int) -> dict[str, Any] | None:
    try:
        response = await client.get(f"/v1/users/{user_id}")
        response.raise_for_status()
        return response.json()  # type: ignore[no-any-return]
    except Exception as e:
        logger.warning("fetch_user(%d) failed: %s", user_id, e)
        return None


async def create_user(client: httpx.AsyncClient, data: dict[str, Any]) -> dict[str, Any] | None:
    try:
        response = await client.post("/v1/users", json=data)
        response.raise_for_status()
        return response.json()  # type: ignore[no-any-return]
    except Exception:
        pass
    return None


async def update_user(
    client: httpx.AsyncClient, user_id: int, patch: dict[str, Any]
) -> dict[str, Any] | None:
    try:
        response = await client.patch(f"/v1/users/{user_id}", json=patch)
        response.raise_for_status()
        return response.json()  # type: ignore[no-any-return]
    except Exception as e:
        logger.warning("update_user(%d) failed: %s", user_id, e)
        return None


async def delete_user(client: httpx.AsyncClient, user_id: int) -> bool:
    try:
        response = await client.delete(f"/v1/users/{user_id}")
        response.raise_for_status()
        return True
    except Exception:
        pass
    return False


async def run(api_key: str, user_ids: list[int]) -> list[dict[str, Any]]:
    async with httpx.AsyncClient(
        base_url=BASE_URL,
        timeout=httpx.Timeout(10.0, connect=3.0),
        headers={"Authorization": f"Bearer {api_key}", "Accept": "application/json"},
    ) as client:
        tasks = [fetch_user(client, uid) for uid in user_ids]
        results = await asyncio.gather(*tasks)
        return [r for r in results if r is not None]


if __name__ == "__main__":
    asyncio.run(run("secret-key", [1, 2, 3]))
