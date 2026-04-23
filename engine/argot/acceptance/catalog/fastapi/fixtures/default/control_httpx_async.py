"""
Control: httpx.AsyncClient for downstream HTTP calls in FastAPI dependencies.

This file demonstrates the correct async downstream HTTP pattern:
- httpx.AsyncClient as a context manager in a Depends() dependency
- await client.get() / await client.post() — non-blocking
- response.raise_for_status() for error propagation
- HTTPException re-raised from httpx errors
- No synchronous requests library, no blocking calls

Vocabulary: httpx, AsyncClient, await, Depends, raise_for_status, APIRouter, HTTPException.
"""

from __future__ import annotations

from collections.abc import AsyncGenerator
from typing import Any

import httpx
from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import JSONResponse

router = APIRouter(prefix="/proxy", tags=["proxy"])

UPSTREAM = "https://jsonplaceholder.typicode.com"


async def get_http_client() -> AsyncGenerator[httpx.AsyncClient, None]:
    async with httpx.AsyncClient(base_url=UPSTREAM, timeout=10.0) as client:
        yield client


@router.get("/posts")
async def list_posts(
    limit: int = 10,
    client: httpx.AsyncClient = Depends(get_http_client),
) -> JSONResponse:
    response = await client.get("/posts", params={"_limit": limit})
    response.raise_for_status()
    return JSONResponse(response.json())


@router.get("/posts/{post_id}")
async def get_post(
    post_id: int,
    client: httpx.AsyncClient = Depends(get_http_client),
) -> JSONResponse:
    response = await client.get(f"/posts/{post_id}")
    if response.status_code == 404:
        raise HTTPException(status_code=404, detail="post not found upstream")
    response.raise_for_status()
    return JSONResponse(response.json())


@router.post("/posts")
async def create_post(
    body: dict[str, Any],
    client: httpx.AsyncClient = Depends(get_http_client),
) -> JSONResponse:
    try:
        response = await client.post("/posts", json=body)
        response.raise_for_status()
    except httpx.HTTPStatusError as exc:
        raise HTTPException(status_code=exc.response.status_code, detail=str(exc))
    except httpx.RequestError as exc:
        raise HTTPException(status_code=502, detail=f"upstream error: {exc}")
    return JSONResponse(response.json(), status_code=201)
