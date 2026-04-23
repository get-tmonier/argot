"""
Paradigm break: aiohttp-style async session with connector= argument.

aiohttp passes a TCPConnector (or other connector subclass) to ClientSession via
the connector= keyword argument, and responses are used as async context managers
(`async with session.get(...) as resp:`).  httpx uses neither pattern: transport=
replaces connector=, and response objects are returned directly — not used as
context managers — unless you call client.stream() for streaming.

Code that constructs aiohttp.ClientSession(connector=...) or uses
`async with session.get() as resp:` will not run under httpx.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

import aiohttp
import aiohttp.connector

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


def build_connector(limit: int = 100, limit_per_host: int = 20) -> aiohttp.TCPConnector:
    return aiohttp.TCPConnector(
        limit=limit,
        limit_per_host=limit_per_host,
        enable_cleanup_closed=True,
        force_close=False,
        ssl=False,
    )


async def fetch_all_users(user_ids: list[int], api_key: str) -> list[dict[str, Any]]:
    """Fetch users using an aiohttp.ClientSession with explicit TCPConnector.

    The connector= parameter, aiohttp.ClientTimeout, and using the response as
    an async context manager (`async with session.get(...) as resp:`) are all
    aiohttp-specific patterns.  httpx uses transport= instead of connector=,
    and response.json() is called directly on the returned Response object.
    """
    connector = build_connector(limit=50, limit_per_host=10)
    timeout = aiohttp.ClientTimeout(total=30.0, connect=5.0, sock_read=25.0)

    async with aiohttp.ClientSession(
        connector=connector,
        connector_owner=True,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Accept": "application/json",
            "User-Agent": "example-service/1.0",
        },
        timeout=timeout,
        trust_env=True,
        raise_for_status=True,
    ) as session:
        results: list[dict[str, Any]] = []
        for uid in user_ids:
            async with session.get(f"{BASE_URL}/v1/users/{uid}") as resp:
                data: dict[str, Any] = await resp.json()
                results.append(data)
        return results


async def post_events(events: list[dict[str, Any]], api_key: str) -> list[dict[str, Any]]:
    connector = build_connector(limit=20)
    timeout = aiohttp.ClientTimeout(total=15.0)

    async with aiohttp.ClientSession(
        connector=connector,
        connector_owner=True,
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        timeout=timeout,
    ) as session:
        responses: list[dict[str, Any]] = []
        for event in events:
            async with session.post(f"{BASE_URL}/v1/events", json=event) as resp:
                if resp.status not in (200, 201):
                    text = await resp.text()
                    logger.error("event post failed %d: %s", resp.status, text)
                    continue
                result: dict[str, Any] = await resp.json()
                responses.append(result)
        return responses


if __name__ == "__main__":
    asyncio.run(fetch_all_users([1, 2, 3], api_key="secret"))
