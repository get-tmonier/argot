"""
Control: async generator streaming response with StreamingResponse.

This file demonstrates the correct async streaming pattern in FastAPI:
- async def generator function that yields chunks
- StreamingResponse wrapping the async generator
- No blocking I/O, no threading, no run_until_complete
- Depends() for auth dependency
- await used throughout for async operations

The vocabulary is entirely in-corpus: StreamingResponse, async def, yield,
await, Depends, APIRouter, HTTPException.
"""

from __future__ import annotations

import asyncio
import json
from typing import Any, AsyncGenerator

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import StreamingResponse

router = APIRouter(prefix="/stream", tags=["stream"])

_dataset: list[dict[str, Any]] = [{"id": i, "value": i * 2} for i in range(100)]


async def get_current_user() -> dict[str, Any]:
    return {"id": 1, "role": "user"}


async def generate_records(
    records: list[dict[str, Any]],
    chunk_size: int = 10,
) -> AsyncGenerator[str, None]:
    for i in range(0, len(records), chunk_size):
        chunk = records[i : i + chunk_size]
        for record in chunk:
            yield json.dumps(record) + "\n"
        await asyncio.sleep(0)


async def generate_csv(records: list[dict[str, Any]]) -> AsyncGenerator[str, None]:
    yield "id,value\n"
    for record in records:
        yield f"{record['id']},{record['value']}\n"
        await asyncio.sleep(0)


@router.get("/records")
async def stream_records(
    current_user: dict[str, Any] = Depends(get_current_user),
) -> StreamingResponse:
    return StreamingResponse(
        generate_records(_dataset),
        media_type="application/x-ndjson",
    )


@router.get("/records/csv")
async def stream_csv(
    current_user: dict[str, Any] = Depends(get_current_user),
) -> StreamingResponse:
    return StreamingResponse(
        generate_csv(_dataset),
        media_type="text/csv",
        headers={"Content-Disposition": "attachment; filename=records.csv"},
    )


@router.get("/records/{record_id}/stream")
async def stream_single(
    record_id: int,
    current_user: dict[str, Any] = Depends(get_current_user),
) -> StreamingResponse:
    if record_id < 0 or record_id >= len(_dataset):
        raise HTTPException(status_code=404, detail="record not found")

    async def _gen() -> AsyncGenerator[str, None]:
        record = _dataset[record_id]
        for key, value in record.items():
            await asyncio.sleep(0)
            yield json.dumps({key: value}) + "\n"

    return StreamingResponse(_gen(), media_type="application/x-ndjson")
