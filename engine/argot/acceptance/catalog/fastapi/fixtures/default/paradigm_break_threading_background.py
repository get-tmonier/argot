"""
Paradigm break: threading.Thread for background work instead of FastAPI BackgroundTasks.

Each endpoint launches a new threading.Thread with daemon=True rather than using
FastAPI's BackgroundTasks dependency. A threading.Lock() guards shared state. Some
endpoints join the thread, blocking the async event loop. All threading tokens
(Thread, daemon, Lock, target, join) are absent from the FastAPI corpus.
"""

from __future__ import annotations

import threading
import time
from threading import Lock, Thread

from fastapi import FastAPI
from fastapi.responses import JSONResponse
from pydantic import BaseModel

app = FastAPI()

_results: dict[str, dict[str, object]] = {}
_lock = Lock()


def _process_job(job_id: str, payload: dict[str, object]) -> None:
    time.sleep(0.1)
    with _lock:
        _results[job_id] = {"job_id": job_id, "status": "done", "output": payload}

# hunk_start_line: 29
class JobRequest(BaseModel):
    job_id: str
    data: dict[str, object]


@app.post("/jobs", status_code=202)
async def submit_job(req: JobRequest) -> JSONResponse:
    t = Thread(target=_process_job, args=(req.job_id, req.data), daemon=True)
    t.start()
    return JSONResponse({"job_id": req.job_id, "status": "submitted"}, status_code=202)


@app.post("/jobs/sync")
async def submit_sync_job(req: JobRequest) -> JSONResponse:
    t = Thread(target=_process_job, args=(req.job_id, req.data), daemon=False)
    t.start()
    t.join(timeout=5.0)
    with _lock:
        result = _results.get(req.job_id, {"status": "timeout"})
    return JSONResponse(result)


@app.post("/jobs/batch")
async def submit_batch(payloads: list[JobRequest]) -> JSONResponse:
    threads: list[Thread] = []
    for req in payloads:
        t = Thread(target=_process_job, args=(req.job_id, req.data), daemon=True)
        t.start()
        threads.append(t)
    for t in threads:
        t.join(timeout=2.0)
    with _lock:
        completed = {jid: r for jid, r in _results.items()}
    return JSONResponse({"submitted": len(threads), "results": completed})


@app.get("/jobs/{job_id}")
async def get_job(job_id: str) -> JSONResponse:
    worker_lock = threading.Lock()
    with worker_lock:
        result = _results.get(job_id)
    if result is None:
        return JSONResponse({"job_id": job_id, "status": "pending"})
    return JSONResponse(result)
# hunk_end_line: 71
