"""
Paradigm break (obvious): aiohttp request handlers receiving web.Request objects.

FastAPI routes are declared with @app.get / @router.post and data is injected
through typed function parameters.  aiohttp uses plain async functions that
accept a `web.Request` argument; body is read via `await request.json()`, path
params via `request.match_info["key"]`, and responses are constructed explicitly
with web.json_response() or web.Response().  Routes are registered imperatively
via app.router.add_get() / add_post().

The vocabulary is entirely aiohttp: aiohttp.web, web.Request, web.Application,
await request.json(), request.match_info, web.json_response(), web.HTTPNotFound(),
app.router.add_get() — none of which exist in the FastAPI corpus.
"""

from __future__ import annotations

from typing import Any

from aiohttp import web

_users: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com"},
    2: {"id": 2, "name": "Bob", "email": "bob@example.com"},
}
_next_id = 3


async def list_users(request: web.Request) -> web.Response:
    q = request.rel_url.query.get("q", "")
    results = [u for u in _users.values() if q.lower() in str(u["name"]).lower()]
    return web.json_response(results)


async def create_user(request: web.Request) -> web.Response:
    global _next_id
    try:
        data = await request.json()
    except Exception:
        raise web.HTTPBadRequest(reason="invalid JSON")

    if "name" not in data or "email" not in data:
        raise web.HTTPBadRequest(reason="name and email required")

    user: dict[str, Any] = {"id": _next_id, "name": data["name"], "email": data["email"]}
    _users[_next_id] = user
    _next_id += 1
    return web.json_response(user, status=201)


async def get_user(request: web.Request) -> web.Response:
    user_id = int(request.match_info["user_id"])
    user = _users.get(user_id)
    if user is None:
        raise web.HTTPNotFound(reason=f"user {user_id} not found")
    return web.json_response(user)


async def delete_user(request: web.Request) -> web.Response:
    user_id = int(request.match_info["user_id"])
    if user_id not in _users:
        raise web.HTTPNotFound(reason=f"user {user_id} not found")
    del _users[user_id]
    return web.Response(status=204)


app = web.Application()
app.router.add_get("/users", list_users)
app.router.add_post("/users", create_user)
app.router.add_get("/users/{user_id}", get_user)
app.router.add_delete("/users/{user_id}", delete_user)

if __name__ == "__main__":
    web.run_app(app, port=8080)
