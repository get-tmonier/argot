"""
Paradigm break (subtle): catches requests.exceptions types instead of httpx exceptions.

The code uses httpx.Client() correctly throughout — context manager, proper method
calls, raise_for_status() — but the except clauses catch
requests.exceptions.ConnectionError and requests.exceptions.HTTPError instead of
httpx.ConnectError and httpx.HTTPStatusError.  The foreign vocabulary is confined
to two exception type references in the except clauses.

This is a realistic migration mistake: the developer replaced requests calls with
httpx but forgot to update the exception handlers.  The code runs without import
errors only if requests is installed; when it catches httpx errors the clauses
silently fail because httpx exceptions are not subclasses of requests exceptions.
"""

from __future__ import annotations

import logging
from typing import Any

import httpx
import requests.exceptions

logger = logging.getLogger(__name__)

BASE_URL = "https://api.example.com"


def fetch_user(client: httpx.Client, user_id: int) -> dict[str, Any] | None:
    """Fetch a single user record.

    httpx.Client.get() is used correctly, but the except clause catches the wrong
    exception hierarchy: requests.exceptions.ConnectionError instead of
    httpx.ConnectError, and requests.exceptions.HTTPError instead of
    httpx.HTTPStatusError.
    """
    try:
        response = client.get(f"/v1/users/{user_id}")
        response.raise_for_status()
        return response.json()  # type: ignore[no-any-return]
    except requests.exceptions.ConnectionError as exc:
        logger.error("connection error fetching user %d: %s", user_id, exc)
        return None
    except requests.exceptions.HTTPError as exc:
        logger.error("http error fetching user %d: %s", user_id, exc)
        return None


def create_resource(
    client: httpx.Client, path: str, payload: dict[str, Any]
) -> dict[str, Any] | None:
    """POST a resource and return the created object.

    Same pattern: httpx.Client.post() called correctly, but the error handling
    catches requests exception types.
    """
    try:
        response = client.post(path, json=payload)
        response.raise_for_status()
        return response.json()  # type: ignore[no-any-return]
    except requests.exceptions.ConnectionError as exc:
        logger.warning("connection failed posting to %s: %s", path, exc)
        return None
    except requests.exceptions.HTTPError as exc:
        logger.warning("http error posting to %s: %s", path, exc)
        return None


def run(api_key: str, user_ids: list[int]) -> list[dict[str, Any]]:
    with httpx.Client(
        base_url=BASE_URL,
        timeout=httpx.Timeout(10.0, connect=3.0),
        headers={"Authorization": f"Bearer {api_key}", "Accept": "application/json"},
    ) as client:
        results: list[dict[str, Any]] = []
        for uid in user_ids:
            user = fetch_user(client, uid)
            if user is not None:
                results.append(user)
        return results
