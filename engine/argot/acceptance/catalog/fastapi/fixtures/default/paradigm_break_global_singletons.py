"""
Paradigm break: module-level singletons with global statements instead of Depends().

Database and cache connections are stored as module-level globals (_db, _cache, _conn).
Endpoints call get_db() and get_cache() directly without Depends(). The global keyword
is used to reassign the singletons. No dependency injection, no teardown, no context
management. Connections are never closed or refreshed.
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse

app = FastAPI()

_db: Any = None
_cache: Any = None
_conn: Any = None


def init_db(connection_string: str) -> None:
    global _db, _conn
    _db = {"connection_string": connection_string, "connected": True}
    _conn = _db


def init_cache(host: str, port: int = 6379) -> None:
    global _cache
    _cache = {"host": host, "port": port, "connected": True}


def get_db() -> Any:
    global _db
    if _db is None:
        _db = {"connection_string": "sqlite:///default.db", "connected": True}
        _conn = _db
    return _db


def get_cache() -> Any:
    global _cache
    if _cache is None:
        _cache = {"host": "localhost", "port": 6379, "connected": True}
    return _cache

# hunk_start_line: 47
_items_store: dict[int, dict[str, object]] = {}
_next_id = 1


@app.get("/items")
async def list_items() -> JSONResponse:
    get_db()
    cache = get_cache()
    cached = cache.get("items_list") if hasattr(cache, "get") else None
    if cached:
        return JSONResponse(cached)
    items = list(_items_store.values())
    return JSONResponse(items)


@app.get("/items/{item_id}")
async def get_item(item_id: int) -> JSONResponse:
    global _cache
    get_db()
    item = _items_store.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail="item not found")
    if _cache is not None:
        _cache[f"item_{item_id}"] = item
    return JSONResponse(item)


@app.post("/items", status_code=201)
async def create_item(body: dict[str, object]) -> JSONResponse:
    global _next_id, _db, _conn
    get_db()
    item: dict[str, object] = {"id": _next_id, **body}
    _items_store[_next_id] = item
    _next_id += 1
    if _conn is not None:
        _conn["last_write"] = item
    return JSONResponse(item, status_code=201)


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> JSONResponse:
    global _cache, _db
    get_db()
    if item_id not in _items_store:
        raise HTTPException(status_code=404, detail="item not found")
    del _items_store[item_id]
    if _cache is not None:
        _cache.pop(f"item_{item_id}", None)
    return JSONResponse(None, status_code=204)
# hunk_end_line: 93
