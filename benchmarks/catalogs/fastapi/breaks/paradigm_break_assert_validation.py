"""
Paradigm break: assert statements for input validation inside FastAPI endpoints.

FastAPI's idiomatic pattern is to declare a Pydantic BaseModel as a function
parameter, which automatically validates fields, coerces types, and raises
RequestValidationError with structured error messages. This file instead
accepts `item: dict = Body(...)` and uses bare `assert` statements to validate
fields.

The break: assert-based validation at endpoint scope. The FastAPI corpus shows
0 instances of `assert` used for request validation inside endpoint bodies.
Pydantic handles this declaratively via BaseModel subclasses (384 corpus
occurrences) and Field constraints — no hand-written assertions needed.
Additionally, `AssertionError` is not a standard HTTP error; unhandled asserts
will produce 500 responses rather than the expected 422.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Body, HTTPException

router = APIRouter(prefix="/items", tags=["items"])

_items: dict[int, dict[str, Any]] = {}
_next_id = 1

# hunk_start_line: 31


@router.post("", status_code=201)
async def create_item(item: dict[str, Any] = Body(...)) -> dict[str, Any]:
    assert isinstance(item.get("name"), str), "name must be a string"
    assert len(item.get("name", "")) > 0, "name required"
    assert len(item.get("name", "")) <= 100, "name must be at most 100 characters"
    assert isinstance(item.get("price"), (int, float)), "price must be numeric"
    assert item.get("price", -1) > 0, "price must be positive"
    assert item.get("category") in (
        "electronics",
        "clothing",
        "food",
    ), "category must be one of: electronics, clothing, food"

    global _next_id
    record: dict[str, Any] = {
        "id": _next_id,
        "name": item["name"],
        "price": float(item["price"]),
        "category": item["category"],
    }
    _items[_next_id] = record
    _next_id += 1
    return record


@router.put("/{item_id}")
async def update_item(
    item_id: int,
    item: dict[str, Any] = Body(...),
) -> dict[str, Any]:
    existing = _items.get(item_id)
    if existing is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")

    if "name" in item:
        assert isinstance(item["name"], str), "name must be a string"
        assert len(item["name"]) > 0, "name must not be empty"
        assert len(item["name"]) <= 100, "name must be at most 100 characters"
        existing["name"] = item["name"]

    if "price" in item:
        assert isinstance(item["price"], (int, float)), "price must be numeric"
        assert item["price"] > 0, "price must be positive"
        existing["price"] = float(item["price"])

    if "category" in item:
        assert item["category"] in (
            "electronics",
            "clothing",
            "food",
        ), "category must be one of: electronics, clothing, food"
        existing["category"] = item["category"]

    return existing


@router.get("/{item_id}")
async def get_item(item_id: int) -> dict[str, Any]:
    item = _items.get(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return item

# hunk_end_line: 97
