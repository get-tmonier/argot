"""
Paradigm break: bare Starlette Router with add_route() and url_for().

Instead of FastAPI's decorator DSL (@router.get, @router.post), this code uses
Starlette's lower-level Router class directly. Routes are registered imperatively
with add_route() and url_for() produces reverse URLs. Path converters like {id:int}
are Starlette-specific syntax. No Pydantic, no Depends, no APIRouter.
"""

from __future__ import annotations

import json

from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse
from starlette.routing import Mount, Route, Router

_items: dict[int, dict[str, object]] = {
    1: {"id": 1, "name": "Widget", "price": 9.99},
    2: {"id": 2, "name": "Gadget", "price": 19.99},
}
_next_id = 3

# hunk_start_line: 25
async def list_items(request: Request) -> JSONResponse:
    items = list(_items.values())
    return JSONResponse({"items": items, "total": len(items)})


async def create_item(request: Request) -> JSONResponse:
    body = await request.body()
    data = json.loads(body)
    global _next_id
    item: dict[str, object] = {"id": _next_id, "name": data["name"], "price": data["price"]}
    _items[_next_id] = item
    _next_id += 1
    list_url = request.url_for("list_items")
    return JSONResponse({"item": item, "collection": str(list_url)}, status_code=201)


async def get_item(request: Request) -> JSONResponse:
    item_id = int(request.path_params["id"])
    item = _items.get(item_id)
    if item is None:
        return JSONResponse({"detail": "not found"}, status_code=404)
    detail_url = request.url_for("get_item", id=item_id)
    return JSONResponse({"item": item, "self": str(detail_url)})


async def update_item(request: Request) -> JSONResponse:
    item_id = int(request.path_params["id"])
    item = _items.get(item_id)
    if item is None:
        return JSONResponse({"detail": "not found"}, status_code=404)
    body = await request.body()
    data = json.loads(body)
    item.update({k: v for k, v in data.items() if k in ("name", "price")})
    return JSONResponse({"item": item})


async def delete_item(request: Request) -> JSONResponse:
    item_id = int(request.path_params["id"])
    if item_id not in _items:
        return JSONResponse({"detail": "not found"}, status_code=404)
    del _items[item_id]
    return JSONResponse(None, status_code=204)


item_router = Router()
item_router.add_route("/", list_items, methods=["GET"])
item_router.add_route("/", create_item, methods=["POST"])
item_router.add_route("/{id:int}", get_item, methods=["GET"])
item_router.add_route("/{id:int}", update_item, methods=["PUT"])
item_router.add_route("/{id:int}", delete_item, methods=["DELETE"])

routes = [
    Mount("/items", app=item_router),
    Route("/healthz", endpoint=lambda r: JSONResponse({"ok": True})),
]

app = Starlette(routes=routes)
# hunk_end_line: 89
