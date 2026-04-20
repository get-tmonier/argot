"""
Control: httpx.AsyncClient as a yield-based Depends() dependency.

This is the idiomatic FastAPI pattern for downstream HTTP: a generator dependency
owns the client lifecycle (create → yield → close), ensuring the connection pool
is always released even if the endpoint raises. The client is scoped per-request.

Pattern source: FastAPI official docs on dependencies with yield and on HTTP clients
(https://fastapi.tiangolo.com/tutorial/dependencies/dependencies-with-yield/).
Matches corpus vocabulary: httpx, AsyncClient, Depends, raise_for_status,
AsyncGenerator, APIRouter, HTTPException.

Differences from control_httpx_async.py:
- The dependency function itself owns the `async with` block and yields the client,
  rather than the endpoint body opening and closing the context manager.
- Cleaner separation: endpoint body is purely application logic; lifecycle is in the dep.
"""

from __future__ import annotations

from collections.abc import AsyncGenerator
from typing import Any

import httpx
from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import JSONResponse

router = APIRouter(prefix="/items", tags=["items"])

UPSTREAM = "https://jsonplaceholder.typicode.com"

# hunk_start_line: 31


async def http_client_dep() -> AsyncGenerator[httpx.AsyncClient, None]:
    """Yield-based dependency that owns the AsyncClient lifecycle."""
    async with httpx.AsyncClient(base_url=UPSTREAM, timeout=10.0) as client:
        yield client


@router.get("/todos")
async def list_todos(
    limit: int = 20,
    client: httpx.AsyncClient = Depends(http_client_dep),
) -> JSONResponse:
    response = await client.get("/todos", params={"_limit": limit})
    response.raise_for_status()
    return JSONResponse(response.json())


@router.get("/todos/{todo_id}")
async def get_todo(
    todo_id: int,
    client: httpx.AsyncClient = Depends(http_client_dep),
) -> JSONResponse:
    response = await client.get(f"/todos/{todo_id}")
    if response.status_code == 404:
        raise HTTPException(status_code=404, detail="todo not found upstream")
    response.raise_for_status()
    return JSONResponse(response.json())


@router.post("/todos")
async def create_todo(
    body: dict[str, Any],
    client: httpx.AsyncClient = Depends(http_client_dep),
) -> JSONResponse:
    try:
        response = await client.post("/todos", json=body)
        response.raise_for_status()
    except httpx.HTTPStatusError as exc:
        raise HTTPException(status_code=exc.response.status_code, detail=str(exc))
    except httpx.RequestError as exc:
        raise HTTPException(status_code=502, detail=f"upstream unreachable: {exc}")
    return JSONResponse(response.json(), status_code=201)


@router.delete("/todos/{todo_id}", status_code=204)
async def delete_todo(
    todo_id: int,
    client: httpx.AsyncClient = Depends(http_client_dep),
) -> None:
    try:
        response = await client.delete(f"/todos/{todo_id}")
        response.raise_for_status()
    except httpx.HTTPStatusError as exc:
        raise HTTPException(status_code=exc.response.status_code, detail=str(exc))
# hunk_end_line: 84
