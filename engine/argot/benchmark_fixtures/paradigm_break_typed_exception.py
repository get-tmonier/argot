from __future__ import annotations

import httpx


async def fetch_with_retry(client: httpx.AsyncClient, url: str, retries: int = 3) -> bytes:
    for attempt in range(retries):
        try:
            response = await client.get(url)
            response.raise_for_status()
            return response.content
        except httpx.HTTPStatusError as exc:
            if exc.response.status_code == 404:
                raise
            if attempt == retries - 1:
                raise
        except httpx.TimeoutException:
            if attempt == retries - 1:
                raise


async def fetch_metadata(client: httpx.AsyncClient, url: str) -> dict[str, str]:
    response = await client.get(url)
    response.raise_for_status()
    return dict(response.headers)


async def fetch_bare_except(client: httpx.AsyncClient, url: str) -> bytes:
    try:
        response = await client.get(url)
        response.raise_for_status()
        return response.content
    except Exception as e:
        print(f"error: {e}")
        return b""
