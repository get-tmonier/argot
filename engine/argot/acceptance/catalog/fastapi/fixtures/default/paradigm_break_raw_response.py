"""
Paradigm break (obvious): manual Response construction instead of FastAPI serialization.

FastAPI's idiomatic pattern is to return plain dicts or Pydantic model instances
from endpoints (with optional response_model= annotation) and let the framework
serialize them automatically.  This file bypasses that: every endpoint manually
constructs JSONResponse with jsonable_encoder(), or returns a starlette Response
with json.dumps() — recreating serialization that FastAPI provides for free.

The break is structural: the FastAPI corpus never builds JSONResponse(content=
jsonable_encoder(...)) at call sites, and never calls json.dumps() inside endpoint
functions.  Those patterns belong to lower-level starlette code, not application
endpoints.
"""

from __future__ import annotations

import json
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.encoders import jsonable_encoder
from starlette.responses import JSONResponse, Response

app = FastAPI()

_users: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alice", "email": "alice@example.com"},
    2: {"id": 2, "name": "Bob", "email": "bob@example.com"},
}
_next_id = 3


@app.get("/users")
def list_users(q: str = "") -> Response:
    results = [u for u in _users.values() if q.lower() in str(u["name"]).lower()]
    return Response(
        content=json.dumps(results),
        media_type="application/json",
        status_code=200,
    )


@app.get("/users/{user_id}")
def get_user(user_id: int) -> Response:
    user = _users.get(user_id)
    if user is None:
        error = {"detail": f"user {user_id} not found"}
        return Response(
            content=json.dumps(error),
            media_type="application/json",
            status_code=404,
        )
    return Response(
        content=json.dumps(user),
        media_type="application/json",
        status_code=200,
    )


@app.post("/users", status_code=201)
def create_user(body: dict[str, Any]) -> JSONResponse:
    global _next_id
    if not body.get("name") or not body.get("email"):
        raise HTTPException(status_code=400, detail="name and email required")
    user: dict[str, Any] = {"id": _next_id, "name": body["name"], "email": body["email"]}
    _users[_next_id] = user
    _next_id += 1
    return JSONResponse(content=jsonable_encoder(user), status_code=201)


@app.put("/users/{user_id}")
def update_user(user_id: int, body: dict[str, Any]) -> JSONResponse:
    user = _users.get(user_id)
    if user is None:
        return JSONResponse(
            content=jsonable_encoder({"detail": f"user {user_id} not found"}),
            status_code=404,
        )
    user.update({k: v for k, v in body.items() if k in ("name", "email")})
    return JSONResponse(content=jsonable_encoder(user), status_code=200)
