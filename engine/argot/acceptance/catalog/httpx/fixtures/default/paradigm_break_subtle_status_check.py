"""
Paradigm break (subtle): manual status_code checks instead of raise_for_status().

httpx.Response.raise_for_status() is the idiomatic way to turn 4xx/5xx responses
into exceptions (httpx.HTTPStatusError).  This file replaces every
raise_for_status() call with manual `if response.status_code >= 400:` guards that
raise plain ValueError.

All tokens are corpus-present: httpx.Client, client.get/post/patch, response.json(),
response.status_code, context manager — the only deviation is the absence of
raise_for_status() and the presence of ValueError raised manually.  The model
needs to recognise the structural pattern, not a foreign token.
"""

from __future__ import annotations

import logging
from typing import Any

import httpx

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


def fetch_user(client: httpx.Client, user_id: int) -> dict[str, Any]:
    response = client.get(f"/v1/users/{user_id}")
    if response.status_code >= 400:
        raise ValueError(f"HTTP {response.status_code}: /v1/users/{user_id}")
    return response.json()  # type: ignore[no-any-return]


def create_user(client: httpx.Client, data: dict[str, Any]) -> dict[str, Any]:
    response = client.post("/v1/users", json=data)
    if response.status_code >= 400:
        raise ValueError(f"HTTP {response.status_code}: /v1/users")
    return response.json()  # type: ignore[no-any-return]


def update_user(client: httpx.Client, user_id: int, patch: dict[str, Any]) -> dict[str, Any]:
    response = client.patch(f"/v1/users/{user_id}", json=patch)
    if response.status_code >= 400:
        raise ValueError(f"HTTP {response.status_code}: /v1/users/{user_id}")
    return response.json()  # type: ignore[no-any-return]


def delete_user(client: httpx.Client, user_id: int) -> bool:
    response = client.delete(f"/v1/users/{user_id}")
    if response.status_code >= 400:
        raise ValueError(f"HTTP {response.status_code}: /v1/users/{user_id}")
    return response.status_code == 204


def list_resources(client: httpx.Client, path: str, params: dict[str, Any]) -> list[Any]:
    response = client.get(path, params=params)
    if response.status_code >= 400:
        raise ValueError(f"HTTP {response.status_code}: {path}")
    return response.json()  # type: ignore[no-any-return]


def run(api_key: str, user_ids: list[int]) -> list[dict[str, Any]]:
    with httpx.Client(
        base_url=BASE_URL,
        timeout=httpx.Timeout(10.0, connect=3.0),
        headers={"Authorization": f"Bearer {api_key}", "Accept": "application/json"},
    ) as client:
        users: list[dict[str, Any]] = []
        for uid in user_ids:
            try:
                user = fetch_user(client, uid)
                users.append(user)
            except ValueError as exc:
                logger.error("failed to fetch user %d: %s", uid, exc)
        return users
