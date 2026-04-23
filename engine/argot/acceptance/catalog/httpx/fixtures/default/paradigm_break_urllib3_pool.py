"""
Paradigm break: urllib3 manual connection pool management.

urllib3 exposes PoolManager and HTTPConnectionPool / HTTPSConnectionPool for
direct pool-level control: explicit num_connections, num_pools, and per-pool
request() calls.  httpx abstracts this entirely — the Limits() config object
controls max_connections and max_keepalive_connections, and the transport
layer manages pool lifecycle invisibly.  Code that calls pool.request() or
manually constructs HTTPSConnectionPool objects is urllib3-specific and has
no httpx equivalent.
"""

from __future__ import annotations

import json
import logging
from typing import Any

import urllib3
import urllib3.exceptions
from urllib3.util.retry import Retry
from urllib3.util.timeout import Timeout

logger = logging.getLogger(__name__)

API_HOST = "api.example.com"
API_PORT = 443
DEFAULT_TIMEOUT = Timeout(connect=5.0, read=30.0)


def build_pool_manager(
    num_pools: int = 10,
    maxsize: int = 20,
    retries: int = 3,
) -> urllib3.PoolManager:
    retry = Retry(
        total=retries,
        backoff_factor=0.3,
        status_forcelist={429, 500, 502, 503, 504},
        allowed_methods={"GET", "POST", "PUT", "DELETE"},
        raise_on_redirect=False,
    )
    return urllib3.PoolManager(
        num_pools=num_pools,
        maxsize=maxsize,
        retries=retry,
        timeout=DEFAULT_TIMEOUT,
        headers={"User-Agent": "example-service/1.0"},
    )


def fetch_user_direct(pool: urllib3.HTTPSConnectionPool, user_id: int) -> dict[str, Any]:
    """Direct pool.request() call on a manually managed HTTPSConnectionPool.

    httpx has no equivalent: Client.get() replaces pool.request(); connection
    pool objects are never constructed or managed by callers.
    """
    response = pool.request(
        "GET",
        f"/v1/users/{user_id}",
        headers={"Accept": "application/json", "Authorization": "Bearer token123"},
        timeout=Timeout(connect=3.0, read=10.0),
    )
    if response.status >= 400:
        raise urllib3.exceptions.HTTPError(
            f"HTTP {response.status} for user {user_id}: {response.data}"
        )
    return json.loads(response.data.decode("utf-8"))  # type: ignore[no-any-return]


def post_event_direct(pool: urllib3.HTTPSConnectionPool, payload: dict[str, Any]) -> dict[str, Any]:
    body = json.dumps(payload).encode("utf-8")
    response = pool.request(
        "POST",
        "/v1/events",
        body=body,
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json",
            "Authorization": "Bearer token123",
        },
    )
    response.data  # consume body
    if response.status not in (200, 201):
        raise urllib3.exceptions.HTTPError(f"POST /v1/events returned {response.status}")
    return json.loads(response.data.decode("utf-8"))  # type: ignore[no-any-return]


def run_batch(user_ids: list[int]) -> list[dict[str, Any]]:
    # Manually construct an HTTPSConnectionPool for fine-grained control.
    # In httpx this lifecycle is fully encapsulated inside Client / transport.
    pool = urllib3.HTTPSConnectionPool(
        host=API_HOST,
        port=API_PORT,
        maxsize=10,
        timeout=DEFAULT_TIMEOUT,
        retries=Retry(total=2, backoff_factor=0.5),
        block=True,
    )
    try:
        results = []
        for uid in user_ids:
            try:
                user = fetch_user_direct(pool, uid)
                results.append(user)
            except urllib3.exceptions.MaxRetryError as exc:
                logger.warning("max retries exceeded for user %d: %s", uid, exc)
        return results
    finally:
        pool.close()
