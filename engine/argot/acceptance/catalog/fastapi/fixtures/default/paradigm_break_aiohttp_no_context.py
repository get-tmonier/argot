"""
Paradigm break: aiohttp.ClientSession() without a context manager.

The break axis is session lifecycle management. The session is created at module
level (and also per-request in one endpoint) without using `async with`, leaving
TCP connections dangling and the connector unclosed. The corpus has 0 sites using
aiohttp.ClientSession; the idiomatic FastAPI pattern is httpx.AsyncClient injected
via a yield-based Depends() dependency that owns the client lifecycle.

Foreign-library signal: `aiohttp` is not present in the corpus. The break is both
structural (no context manager) and lexical (aiohttp vs. httpx).
"""

from __future__ import annotations

from typing import Any

import aiohttp
from fastapi import APIRouter, HTTPException

router = APIRouter(prefix="/proxy", tags=["proxy"])

UPSTREAM_BASE = "https://jsonplaceholder.typicode.com"

# hunk_start_line: 24
# Session created at module level without async with — connector is never closed.
_session = aiohttp.ClientSession(base_url=UPSTREAM_BASE)


@router.get("/posts")
async def list_posts(limit: int = 10) -> list[Any]:
    async with _session.get("/posts", params={"_limit": limit}) as resp:
        if resp.status >= 400:
            raise HTTPException(status_code=resp.status, detail=await resp.text())
        return await resp.json()  # type: ignore[no-any-return]


@router.get("/posts/{post_id}")
async def get_post(post_id: int) -> dict[str, Any]:
    async with _session.get(f"/posts/{post_id}") as resp:
        if resp.status == 404:
            raise HTTPException(status_code=404, detail="post not found upstream")
        if resp.status >= 400:
            raise HTTPException(status_code=resp.status, detail=await resp.text())
        return await resp.json()  # type: ignore[no-any-return]


@router.post("/posts")
async def create_post(body: dict[str, Any]) -> dict[str, Any]:
    # Per-request session — also not closed via context manager at the session level.
    session = aiohttp.ClientSession(base_url=UPSTREAM_BASE)
    async with session.post("/posts", json=body) as resp:
        if resp.status >= 400:
            raise HTTPException(status_code=resp.status, detail=await resp.text())
        return await resp.json()  # type: ignore[no-any-return]
    # session.close() never called — connection pool leaks.


@router.get("/users/{user_id}/posts")
async def get_user_posts(user_id: int) -> list[Any]:
    async with _session.get("/posts", params={"userId": user_id}) as resp:
        if resp.status >= 400:
            raise HTTPException(status_code=resp.status, detail=await resp.text())
        return await resp.json()  # type: ignore[no-any-return]
# hunk_end_line: 61
