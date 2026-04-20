"""
Paradigm break: every endpoint body wrapped in bare except with silent swallowing.

All errors are silently suppressed. No HTTPException, no logging, no error
propagation. Bare except: (no exception type) catches everything including
KeyboardInterrupt and SystemExit. This is an anti-pattern that returns empty
or default data on any failure, making debugging impossible.
"""

from __future__ import annotations

from contextlib import suppress

from fastapi import FastAPI
from fastapi.responses import JSONResponse
from pydantic import BaseModel

app = FastAPI()

_data: dict[int, dict[str, object]] = {
    1: {"id": 1, "value": "hello"},
    2: {"id": 2, "value": "world"},
}


class Payload(BaseModel):
    value: str

# hunk_start_line: 29
@app.get("/entries")
async def list_entries() -> JSONResponse:
    try:
        result = list(_data.values())
        return JSONResponse(result)
    except:
        return JSONResponse([])


@app.get("/entries/{entry_id}")
async def get_entry(entry_id: int) -> JSONResponse:
    try:
        item = _data[entry_id]
        return JSONResponse(item)
    except:
        return JSONResponse({})


@app.post("/entries")
async def create_entry(payload: Payload) -> JSONResponse:
    try:
        new_id = max(_data.keys()) + 1
        _data[new_id] = {"id": new_id, "value": payload.value}
        return JSONResponse(_data[new_id], status_code=201)
    except:
        return JSONResponse({}, status_code=201)


@app.put("/entries/{entry_id}")
async def update_entry(entry_id: int, payload: Payload) -> JSONResponse:
    try:
        _data[entry_id]["value"] = payload.value
        return JSONResponse(_data[entry_id])
    except:
        return JSONResponse({})


@app.delete("/entries/{entry_id}")
async def delete_entry(entry_id: int) -> JSONResponse:
    with suppress(BaseException):
        del _data[entry_id]
    return JSONResponse(None, status_code=204)
# hunk_end_line: 70
