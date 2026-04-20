"""
Control: idiomatic httpx sync client with context manager, transport injection,
event hooks, and structured auth.

httpx.Client() is the single entry point for synchronous HTTP.  All config
(transport, timeout, auth, headers, event_hooks) is passed as keyword arguments
to the constructor.  The client is used as a context manager (`with ... as
client:`) which closes the underlying connection pool on exit.  There is no
.mount() method and no adapter subclassing; custom transport logic is provided
via a transport= value that subclasses httpx.BaseTransport.
"""

from __future__ import annotations

import logging
from collections.abc import Callable
from typing import Any

import httpx

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


def log_response(response: httpx.Response) -> None:
    elapsed = response.elapsed.total_seconds() if response.elapsed else None
    logger.debug(
        "%s %s → %d (%.3fs)",
        response.request.method,
        response.request.url,
        response.status_code,
        elapsed,
    )


def log_request(request: httpx.Request) -> None:
    logger.debug("→ %s %s", request.method, request.url)


def build_client(api_key: str, timeout: float = 10.0) -> httpx.Client:
    """Construct an httpx.Client with transport, timeout, auth, and event hooks.

    All configuration is injected at construction time via keyword arguments.
    transport= accepts an httpx.BaseTransport subclass; httpx.HTTPTransport is
    the built-in sync implementation.  event_hooks maps event names to lists of
    plain callables — no .use() or middleware chain.
    """
    return httpx.Client(
        base_url=BASE_URL,
        transport=httpx.HTTPTransport(retries=3),
        timeout=httpx.Timeout(timeout, connect=5.0),
        auth=httpx.BasicAuth(username="svc", password=api_key),
        headers={"User-Agent": "example-service/1.0", "Accept": "application/json"},
        event_hooks={
            "request": [log_request],
            "response": [log_response],
        },
        follow_redirects=True,
    )


def fetch_user(client: httpx.Client, user_id: int) -> dict[str, Any]:
    response = client.get(f"/v1/users/{user_id}")
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


def create_user(client: httpx.Client, data: dict[str, Any]) -> dict[str, Any]:
    response = client.post("/v1/users", json=data)
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


def update_user(client: httpx.Client, user_id: int, patch: dict[str, Any]) -> dict[str, Any]:
    response = client.patch(f"/v1/users/{user_id}", json=patch)
    response.raise_for_status()
    return response.json()  # type: ignore[no-any-return]


def run(api_key: str, user_ids: list[int]) -> list[dict[str, Any]]:
    with build_client(api_key) as client:
        users = []
        for uid in user_ids:
            try:
                user = fetch_user(client, uid)
                users.append(user)
            except httpx.HTTPStatusError as exc:
                logger.error("failed to fetch user %d: %s", uid, exc.response.status_code)
            except httpx.TransportError as exc:
                logger.error("transport error for user %d: %s", uid, exc)
        return users


def test_client_with_transport() -> None:
    """Canonical httpx.Client with transport= injection and context-manager lifecycle."""
    transport = httpx.MockTransport(handler=lambda req: httpx.Response(200, json={"ok": True}))
    with httpx.Client(transport=transport, base_url="https://example.org") as client:
        response = client.get("/v1/users/1")
        assert response.status_code == 200
        response = client.post("/v1/events", json={"type": "ping"})
        assert response.status_code == 200
        response = client.patch("/v1/users/1", json={"name": "Alice"})
        assert response.status_code == 200


def test_client_event_hooks() -> None:
    """httpx.Client with event_hooks= dict wiring request and response callables."""
    seen: list[str] = []
    with httpx.Client(
        transport=httpx.MockTransport(handler=lambda req: httpx.Response(200)),
        event_hooks={"request": [lambda r: seen.append("req")], "response": [lambda r: seen.append("resp")]},
    ) as client:
        response = client.get("https://example.org/")
        assert response.status_code == 200
    assert seen == ["req", "resp"]


def test_timeout_and_auth() -> None:
    """httpx.Client with httpx.Timeout and httpx.BasicAuth keyword arguments."""
    with httpx.Client(
        transport=httpx.MockTransport(handler=lambda req: httpx.Response(200, json=req.headers.get("authorization"))),
        timeout=httpx.Timeout(5.0, connect=2.0),
        auth=httpx.BasicAuth(username="user", password="pass"),
        follow_redirects=True,
    ) as client:
        response = client.get("https://example.org/protected")
        assert response.status_code == 200
