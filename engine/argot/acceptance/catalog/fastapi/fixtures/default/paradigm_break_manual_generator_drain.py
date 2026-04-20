"""
Paradigm break: manual generator drain with next() instead of Depends().

Endpoints call `db = next(get_db())` directly inside the function body and
manage teardown with a bare try/finally block. This bypasses FastAPI's
dependency injection lifecycle entirely — no Depends() registration, no
framework-managed cleanup, no substitution for testing.

Corpus evidence:
- `Depends(...)` sites: 428 (dominant pattern)
- `next(get_db())` manual drain: 0 corpus sites (absent, OOV pattern)

The structural axis: replacing Depends(get_db) with next(get_db()) at call
sites, plus manual try/finally teardown.
"""

from __future__ import annotations

from collections.abc import Generator
from typing import Any

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI()

# Simulated in-memory store
_store: dict[int, dict[str, Any]] = {
    1: {"id": 1, "title": "First item", "done": False},
    2: {"id": 2, "title": "Second item", "done": True},
}
_next_id = 3


class ItemIn(BaseModel):
    title: str
    done: bool = False


def get_db() -> Generator[dict[int, dict[str, Any]], None, None]:
    """Generator dependency — intended to be used via Depends(), not drained manually."""
    session = _store
    try:
        yield session
    finally:
        # teardown (close connection, rollback, etc.)
        pass


@app.get("/items")
async def list_items() -> list[dict[str, Any]]:
    # BREAK: manually draining the generator instead of using Depends(get_db)
    gen = get_db()
    db = next(gen)
    try:
        return list(db.values())
    finally:
        try:
            next(gen)
        except StopIteration:
            pass


@app.get("/items/{item_id}")
async def get_item(item_id: int) -> dict[str, Any]:
    # BREAK: same manual drain pattern repeated at every call site
    gen = get_db()
    db = next(gen)
    try:
        item = db.get(item_id)
        if item is None:
            raise HTTPException(status_code=404, detail=f"item {item_id} not found")
        return item
    finally:
        try:
            next(gen)
        except StopIteration:
            pass


@app.post("/items", status_code=201)
async def create_item(body: ItemIn) -> dict[str, Any]:
    global _next_id
    # BREAK: generator drained manually, no DI lifecycle
    gen = get_db()
    db = next(gen)
    try:
        item: dict[str, Any] = {"id": _next_id, "title": body.title, "done": body.done}
        db[_next_id] = item
        _next_id += 1
        return item
    finally:
        try:
            next(gen)
        except StopIteration:
            pass


@app.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: int) -> None:
    # BREAK: generator drain bypasses FastAPI's dependency graph
    gen = get_db()
    db = next(gen)
    try:
        if item_id not in db:
            raise HTTPException(status_code=404, detail=f"item {item_id} not found")
        del db[item_id]
    finally:
        try:
            next(gen)
        except StopIteration:
            pass
