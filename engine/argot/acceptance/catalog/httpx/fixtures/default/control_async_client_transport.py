from __future__ import annotations

import asyncio
from collections.abc import Generator
from typing import Any

import httpx

BASE_URL = "https://api.example.com"


def handle_user_request(request: httpx.Request) -> httpx.Response:
    if request.url.path.startswith("/v1/users"):
        return httpx.Response(200, json={"id": 1, "name": "alice"})
    return httpx.Response(404)


def handle_event_request(request: httpx.Request) -> httpx.Response:
    return httpx.Response(201, json={"status": "created"})


class TokenAuth(httpx.Auth):
    def __init__(self, token: str) -> None:
        self._token = token

    def auth_flow(
        self, request: httpx.Request
    ) -> Generator[httpx.Request, httpx.Response, None]:
        request.headers["Authorization"] = f"Bearer {self._token}"
        yield request


async def fetch_user(client: httpx.AsyncClient, user_id: int) -> dict[str, Any]:
    response = await client.get(f"/v1/users/{user_id}")
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


async def post_event(client: httpx.AsyncClient, payload: dict[str, Any]) -> dict[str, Any]:
    response = await client.post("/v1/events", json=payload)
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


async def test_async_client_get() -> None:
    transport = httpx.MockTransport(handle_user_request)
    async with httpx.AsyncClient(transport=transport, base_url=BASE_URL) as client:
        response = await client.get("/v1/users/1")
        assert response.status_code == 200
        data = response.json()
        assert data["id"] == 1


async def test_async_client_post() -> None:
    transport = httpx.MockTransport(handle_event_request)
    async with httpx.AsyncClient(transport=transport, base_url=BASE_URL) as client:
        response = await client.post("/v1/events", json={"type": "ping"})
        assert response.status_code == 201
        assert response.headers["content-type"] == "application/json"


async def test_async_client_with_auth() -> None:
    transport = httpx.MockTransport(handle_user_request)
    async with httpx.AsyncClient(
        transport=transport,
        base_url=BASE_URL,
        auth=TokenAuth("my-token"),
        timeout=httpx.Timeout(10.0, connect=3.0),
    ) as client:
        response = await client.get("/v1/users/42")
        response.raise_for_status()
        assert response.status_code == 200


async def test_async_client_gather() -> None:
    transport = httpx.MockTransport(handle_user_request)
    async with httpx.AsyncClient(transport=transport, base_url=BASE_URL) as client:
        tasks = [fetch_user(client, uid) for uid in [1, 2, 3]]
        results = await asyncio.gather(*tasks)
        assert len(results) == 3
        for result in results:
            assert "id" in result
