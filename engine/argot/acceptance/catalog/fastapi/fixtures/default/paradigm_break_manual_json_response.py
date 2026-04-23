"""
Paradigm break: manual json.dumps() + Response at every endpoint instead of
FastAPI's automatic Pydantic serialization via response_model=.

Every endpoint calls ``json.dumps()`` explicitly (with custom ``default=`` for datetime
and Decimal), wraps the result in ``Response(content=..., media_type='application/json')``,
and returns it directly.  No ``response_model=`` annotation appears on any endpoint.
FastAPI's automatic serialization pipeline is completely bypassed.

Break axis: explicit per-endpoint ``json.dumps()`` + manual type coercions (str(Decimal),
.isoformat()) vs declarative ``response_model=`` which delegates to Pydantic.
Canonical pattern: docs_src/response_model/tutorial001_py310.py uses response_model= on
every route (163 corpus sites).  Manual json.dumps() at endpoint scope = 0 corpus sites.
"""
from __future__ import annotations

import json
from datetime import datetime, timezone
from decimal import Decimal
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.responses import Response

app = FastAPI()

_orders: dict[int, dict[str, object]] = {}
_next_id = 1


def _json_default(obj: object) -> object:
    if isinstance(obj, Decimal):
        return str(obj)
    if isinstance(obj, datetime):
        return obj.isoformat()
    raise TypeError(f"Object of type {type(obj)} is not JSON serializable")


def _resp(data: Any, status_code: int = 200) -> Response:  # noqa: ANN401
    return Response(
        content=json.dumps(data, default=_json_default, ensure_ascii=False, indent=2),
        status_code=status_code,
        media_type="application/json",
    )


# hunk_start_line: 47
@app.get("/orders")
async def list_orders() -> Response:
    payload = {
        "orders": list(_orders.values()),
        "total": len(_orders),
        "generated_at": datetime.now(timezone.utc).isoformat(),
    }
    return _resp(payload)


@app.post("/orders", status_code=201)
async def create_order(body: dict[str, object]) -> Response:
    global _next_id
    order: dict[str, object] = {
        "id": _next_id,
        "created_at": datetime.now(timezone.utc).isoformat(),
        **{k: str(v) if isinstance(v, Decimal) else v for k, v in body.items()},
    }
    _orders[_next_id] = order
    _next_id += 1
    return _resp(order, status_code=201)


@app.get("/orders/{order_id}")
async def get_order(order_id: int) -> Response:
    order = _orders.get(order_id)
    if order is None:
        return _resp({"detail": "order not found"}, status_code=404)
    return _resp(order)


@app.patch("/orders/{order_id}")
async def update_order(order_id: int, body: dict[str, object]) -> Response:
    order = _orders.get(order_id)
    if order is None:
        raise HTTPException(status_code=404, detail="order not found")
    order.update({k: str(v) if isinstance(v, Decimal) else v for k, v in body.items()})
    order["updated_at"] = datetime.now(timezone.utc).isoformat()
    return _resp(order)


@app.delete("/orders/{order_id}")
async def delete_order(order_id: int) -> Response:
    if order_id not in _orders:
        return _resp({"detail": "not found"}, status_code=404)
    del _orders[order_id]
    return _resp({"deleted": order_id})
# hunk_end_line: 95
