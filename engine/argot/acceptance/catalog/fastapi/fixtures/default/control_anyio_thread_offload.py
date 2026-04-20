"""
Control: async endpoints offloading blocking work via anyio.to_thread.run_sync().

FastAPI's own internals use this pattern: fastapi/concurrency.py lines 32 and 39
call `await anyio.to_thread.run_sync(fn, *args, limiter=...)` to run blocking
callables without stalling the event loop. This fixture shows a report-generation
API where each endpoint wraps its CPU-bound or blocking I/O helper in
anyio.to_thread.run_sync(), which is the idiomatic FastAPI / anyio offload pattern.

Grounded in:
  - fastapi/concurrency.py (lines 6, 32, 39): `import anyio.to_thread` +
    `await anyio.to_thread.run_sync(cm.__exit__, ...)`.
  - tests/test_stream_cancellation.py (line 9): `import anyio` in async endpoint context.

No blocking calls appear directly inside async def bodies — every slow operation
is wrapped in a callable and handed to run_sync().
"""

from __future__ import annotations

import functools
import json
from pathlib import Path

import anyio.to_thread
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI()

REPORTS_DIR = Path("/var/data/reports")


class ReportRequest(BaseModel):
    report_id: str
    filters: dict[str, str] = {}


# ---------------------------------------------------------------------------
# Blocking helpers — run in a thread pool, never called directly from async def
# ---------------------------------------------------------------------------


def _load_report_sync(path: Path) -> dict[str, object]:
    """Read and parse a JSON report file — blocking, runs in thread."""
    raw = path.read_text(encoding="utf-8")
    return json.loads(raw)  # type: ignore[no-any-return]


def _save_report_sync(path: Path, data: dict[str, object]) -> None:
    """Write a JSON report file — blocking, runs in thread."""
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def _list_reports_sync(directory: Path, tag: str | None) -> list[dict[str, object]]:
    """Scan directory and filter by tag — blocking, runs in thread."""
    results: list[dict[str, object]] = []
    for file in directory.glob("*.json"):
        doc: dict[str, object] = json.loads(file.read_text(encoding="utf-8"))
        if tag is None or tag in doc.get("tags", []):
            results.append(doc)
    return results


def _delete_report_sync(path: Path) -> None:
    """Validate and delete a report file — blocking, runs in thread."""
    _ = json.loads(path.read_text(encoding="utf-8"))  # validate before delete
    path.unlink()


# hunk_start_line: 67
@app.get("/reports/{report_id}")
async def get_report(report_id: str) -> dict[str, object]:
    path = REPORTS_DIR / f"{report_id}.json"
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"report {report_id!r} not found")
    # Offload blocking file read to a thread — event loop stays free
    result: dict[str, object] = await anyio.to_thread.run_sync(
        functools.partial(_load_report_sync, path)
    )
    return result


@app.post("/reports", status_code=201)
async def create_report(payload: ReportRequest) -> dict[str, object]:
    path = REPORTS_DIR / f"{payload.report_id}.json"
    if path.exists():
        raise HTTPException(status_code=409, detail=f"report {payload.report_id!r} already exists")
    doc: dict[str, object] = {
        "id": payload.report_id,
        "filters": payload.filters,
        "status": "pending",
    }
    # Offload blocking write to a thread
    await anyio.to_thread.run_sync(functools.partial(_save_report_sync, path, doc))
    return doc


@app.get("/reports")
async def list_reports(tag: str | None = None) -> dict[str, object]:
    # Offload the directory scan + JSON parsing loop to a thread
    results: list[dict[str, object]] = await anyio.to_thread.run_sync(
        functools.partial(_list_reports_sync, REPORTS_DIR, tag)
    )
    return {"reports": results, "total": len(results)}


@app.delete("/reports/{report_id}", status_code=204)
async def delete_report(report_id: str) -> None:
    path = REPORTS_DIR / f"{report_id}.json"
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"report {report_id!r} not found")
    # Offload blocking validation + unlink to a thread
    await anyio.to_thread.run_sync(functools.partial(_delete_report_sync, path))
# hunk_end_line: 110
