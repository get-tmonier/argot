"""
Paradigm break: explicit orjson.dumps() at every endpoint, bypassing FastAPI serialization.

Every endpoint manually serializes its response using orjson.dumps() with explicit
options (OPT_INDENT_2, OPT_NON_STR_KEYS, OPT_SORT_KEYS). The result is wrapped in
Response(content=..., media_type="application/json"). FastAPI's automatic Pydantic
serialization is never used. This pattern defeats response_model and produces
unvalidated output.
"""

from __future__ import annotations

import orjson
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response

app = FastAPI()

_records: dict[int, dict[str, object]] = {
    1: {"id": 1, "name": "Alpha", "score": 1.5},
    2: {"id": 2, "name": "Beta", "score": 2.0},
}
_next_id = 3

# hunk_start_line: 26
def _json_resp(data: object, status_code: int = 200) -> Response:
    content = orjson.dumps(
        data,
        option=orjson.OPT_INDENT_2 | orjson.OPT_NON_STR_KEYS | orjson.OPT_SORT_KEYS,
    )
    return Response(content=content, status_code=status_code, media_type="application/json")


@app.get("/records")
async def list_records() -> Response:
    payload = {"records": list(_records.values()), "total": len(_records)}
    return _json_resp(
        orjson.dumps(payload, option=orjson.OPT_NON_STR_KEYS | orjson.OPT_SORT_KEYS),
    )


@app.get("/records/{record_id}")
async def get_record(record_id: int) -> Response:
    rec = _records.get(record_id)
    if rec is None:
        err = orjson.dumps({"detail": "not found"}, option=orjson.OPT_SORT_KEYS)
        return Response(content=err, status_code=404, media_type="application/json")
    return Response(
        content=orjson.dumps(rec, option=orjson.OPT_INDENT_2 | orjson.OPT_SORT_KEYS),
        media_type="application/json",
    )


@app.post("/records", status_code=201)
async def create_record(body: dict[str, object]) -> Response:
    global _next_id
    rec: dict[str, object] = {"id": _next_id, **body}
    _records[_next_id] = rec
    _next_id += 1
    serialized = orjson.dumps(
        rec,
        option=orjson.OPT_INDENT_2 | orjson.OPT_NON_STR_KEYS | orjson.OPT_SORT_KEYS,
    )
    return Response(content=serialized, status_code=201, media_type="application/json")


@app.delete("/records/{record_id}")
async def delete_record(record_id: int) -> Response:
    if record_id not in _records:
        raise HTTPException(status_code=404, detail="not found")
    del _records[record_id]
    return Response(
        content=orjson.dumps({"deleted": record_id}, option=orjson.OPT_SORT_KEYS),
        media_type="application/json",
    )
# hunk_end_line: 92
