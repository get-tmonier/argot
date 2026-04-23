"""
Paradigm break: raw http.client / socket-level HTTP.

http.client (stdlib) exposes HTTP at the connection level: callers construct
HTTPSConnection manually, call conn.request() with raw headers, then call
conn.getresponse() to obtain an HTTPResponse object — and must explicitly
close the connection.  httpx abstracts all of this: Client() manages connection
pooling, keep-alive, TLS, redirects, and response reading; callers never touch
connection objects.

Code using http.client.HTTPSConnection or socket-level HTTP has no httpx
equivalent and cannot be incrementally migrated without replacing the entire
transport.
"""

from __future__ import annotations

import http.client
import json
import logging
import ssl
from typing import Any

logger = logging.getLogger(__name__)

API_HOST = "api.example.com"
API_PORT = 443


def _make_ssl_context() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.minimum_version = ssl.TLSVersion.TLSv1_2
    return ctx


def get_user(user_id: int, api_key: str) -> dict[str, Any]:
    """Fetch a user via http.client.HTTPSConnection.

    httpx.Client.get() replaces this entire function: connection construction,
    header injection, request dispatch, response reading, and conn.close() are
    all managed internally.  There is no conn.request() / conn.getresponse()
    lifecycle in httpx.
    """
    conn = http.client.HTTPSConnection(
        host=API_HOST,
        port=API_PORT,
        timeout=10.0,
        context=_make_ssl_context(),
    )
    try:
        conn.request(
            "GET",
            f"/v1/users/{user_id}",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Accept": "application/json",
                "User-Agent": "example-service/1.0",
                "Host": API_HOST,
            },
        )
        response = conn.getresponse()
        raw = response.read()
        if response.status >= 400:
            raise http.client.HTTPException(
                f"HTTP {response.status} {response.reason} for GET /v1/users/{user_id}"
            )
        return json.loads(raw.decode("utf-8"))  # type: ignore[no-any-return]
    finally:
        conn.close()


def post_event(payload: dict[str, Any], api_key: str) -> dict[str, Any]:
    body = json.dumps(payload).encode("utf-8")
    conn = http.client.HTTPSConnection(
        host=API_HOST,
        port=API_PORT,
        timeout=15.0,
        context=_make_ssl_context(),
    )
    try:
        conn.request(
            "POST",
            "/v1/events",
            body=body,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
                "Content-Length": str(len(body)),
                "Accept": "application/json",
                "Host": API_HOST,
            },
        )
        response = conn.getresponse()
        data = response.read()
        if response.status not in (200, 201):
            raise http.client.HTTPException(
                f"HTTP {response.status} {response.reason} for POST /v1/events"
            )
        return json.loads(data.decode("utf-8"))  # type: ignore[no-any-return]
    finally:
        conn.close()
