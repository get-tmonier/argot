"""
Paradigm break: synchronous requests.get() / requests.post() inside async def endpoints.

All downstream HTTP calls use the synchronous requests library. This blocks the async
event loop for the duration of each network call, degrading throughput under concurrent
load. The correct approach is httpx.AsyncClient with await. The requests.Session() with
cert= and timeout= parameters are used explicitly throughout.
"""

from __future__ import annotations

import requests
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from requests import Session

app = FastAPI()

UPSTREAM_BASE = "https://jsonplaceholder.typicode.com"

# hunk_start_line: 21
_session = Session()
_session.headers.update({"Accept": "application/json", "User-Agent": "argot-fixture/1.0"})


@app.get("/posts")
async def list_posts(limit: int = 10) -> JSONResponse:
    resp = requests.get(
        f"{UPSTREAM_BASE}/posts",
        params={"_limit": limit},
        timeout=5,
        verify=True,
    )
    resp.raise_for_status()
    return JSONResponse(resp.json())


@app.get("/posts/{post_id}")
async def get_post(post_id: int) -> JSONResponse:
    resp = _session.get(
        f"{UPSTREAM_BASE}/posts/{post_id}",
        timeout=5,
        verify=True,
    )
    if resp.status_code == 404:
        raise HTTPException(status_code=404, detail="post not found")
    resp.raise_for_status()
    return JSONResponse(resp.json())


@app.post("/posts")
async def create_post(body: dict[str, object]) -> JSONResponse:
    resp = requests.post(
        f"{UPSTREAM_BASE}/posts",
        json=body,
        timeout=10,
        verify=True,
    )
    resp.raise_for_status()
    return JSONResponse(resp.json(), status_code=201)


@app.get("/users/{user_id}/posts")
async def get_user_posts(user_id: int) -> JSONResponse:
    resp = _session.get(
        f"{UPSTREAM_BASE}/posts",
        params={"userId": user_id},
        timeout=5,
        cert=None,
        verify=True,
    )
    resp.raise_for_status()
    return JSONResponse(resp.json())
# hunk_end_line: 84
