"""
Paradigm break: asyncio.get_event_loop() + run_until_complete() inside async endpoints.

Using the deprecated get_event_loop() / run_until_complete() pattern inside async
def endpoint bodies. This is incorrect usage: calling run_until_complete() inside a
running event loop raises RuntimeError and blocks the loop. The correct pattern is
simply awaiting coroutines directly. This code represents the wrong mental model of
async programming.
"""

from __future__ import annotations

import asyncio

from fastapi import FastAPI
from fastapi.responses import JSONResponse

app = FastAPI()


async def _fetch_user(user_id: int) -> dict[str, object]:
    await asyncio.sleep(0)
    return {"id": user_id, "name": f"user_{user_id}"}


async def _fetch_orders(user_id: int) -> list[dict[str, object]]:
    await asyncio.sleep(0)
    return [{"order_id": 1, "user_id": user_id, "total": 42.0}]

# hunk_start_line: 30
@app.get("/users/{user_id}")
async def get_user(user_id: int) -> JSONResponse:
    loop = asyncio.get_event_loop()
    user = loop.run_until_complete(_fetch_user(user_id))
    return JSONResponse(user)


@app.get("/users/{user_id}/orders")
async def get_user_orders(user_id: int) -> JSONResponse:
    event_loop = asyncio.get_event_loop()
    orders = event_loop.run_until_complete(_fetch_orders(user_id))
    return JSONResponse({"user_id": user_id, "orders": orders})


@app.get("/summary/{user_id}")
async def get_summary(user_id: int) -> JSONResponse:
    loop = asyncio.get_event_loop()
    user = loop.run_until_complete(_fetch_user(user_id))
    orders = loop.run_until_complete(_fetch_orders(user_id))
    total = sum(float(o.get("total", 0)) for o in orders)
    return JSONResponse({"user": user, "order_count": len(orders), "total_spent": total})


@app.post("/users/{user_id}/refresh")
async def refresh_user(user_id: int) -> JSONResponse:
    event_loop = asyncio.get_event_loop()
    user = event_loop.run_until_complete(_fetch_user(user_id))
    orders = event_loop.run_until_complete(_fetch_orders(user_id))
    return JSONResponse({"refreshed": True, "user": user, "orders": orders})
# hunk_end_line: 57
