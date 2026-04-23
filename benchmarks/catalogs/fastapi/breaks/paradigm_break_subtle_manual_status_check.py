"""
Paradigm break (subtle): manual status-code checks instead of raise_for_status().

The overall structure is idiomatic FastAPI: @router.get / @router.post decorators,
async def endpoints, Pydantic models, HTTPException for error propagation.  The
break is the downstream HTTP-client pattern: instead of calling
`response.raise_for_status()` and letting httpx propagate errors, every endpoint
manually inspects `if response.status_code >= 400:` and re-raises an HTTPException.

This is subtly non-idiomatic: the FastAPI + httpx corpus uses raise_for_status()
wrapped in a dependency or try/except block; manual status-code branching at each
call site is absent from the corpus.  No foreign tokens — all vocabulary is present.
"""

from __future__ import annotations

from typing import Any

import httpx
from fastapi import APIRouter, HTTPException

router = APIRouter(prefix="/proxy", tags=["proxy"])

UPSTREAM_URL = "https://upstream.internal.example.com"

_http_client = httpx.Client(base_url=UPSTREAM_URL, timeout=10.0)


@router.get("/users/{user_id}")
async def proxy_get_user(user_id: int) -> dict[str, Any]:
    response = _http_client.get(f"/v1/users/{user_id}")
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)
    return response.json()  # type: ignore[no-any-return]


@router.post("/users")
async def proxy_create_user(body: dict[str, Any]) -> dict[str, Any]:
    response = _http_client.post("/v1/users", json=body)
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)
    return response.json()  # type: ignore[no-any-return]


@router.put("/users/{user_id}")
async def proxy_update_user(user_id: int, body: dict[str, Any]) -> dict[str, Any]:
    response = _http_client.put(f"/v1/users/{user_id}", json=body)
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)
    return response.json()  # type: ignore[no-any-return]


@router.delete("/users/{user_id}", status_code=204)
async def proxy_delete_user(user_id: int) -> None:
    response = _http_client.delete(f"/v1/users/{user_id}")
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)


@router.get("/search")
async def proxy_search(q: str = "", page: int = 1) -> dict[str, Any]:
    response = _http_client.get("/v1/search", params={"q": q, "page": page})
    if response.status_code >= 400:
        raise HTTPException(status_code=response.status_code, detail=response.text)
    return response.json()  # type: ignore[no-any-return]
