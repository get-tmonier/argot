"""
Paradigm break: concurrent.futures.ProcessPoolExecutor / ThreadPoolExecutor submit()
for background work instead of FastAPI BackgroundTasks.

Endpoints call ``executor.submit(fn, *args)`` and immediately discard the Future
rather than injecting ``BackgroundTasks`` and calling ``add_task()``.  The executor
is a module-level singleton.  ``executor.submit()``, ``Future``, ``ProcessPoolExecutor``,
and ``ThreadPoolExecutor`` in endpoint bodies are absent from the FastAPI corpus.

Corpus evidence: BackgroundTasks sites = 10, add_task sites = 10;
concurrent.futures executor.submit() inside endpoint-decorated functions = 0.
"""
from __future__ import annotations

import time
from concurrent.futures import Future, ProcessPoolExecutor, ThreadPoolExecutor

from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

_thread_pool = ThreadPoolExecutor(max_workers=4)
_proc_pool = ProcessPoolExecutor(max_workers=2)


def _write_audit(user_id: int, action: str) -> None:
    time.sleep(0.01)


def _generate_report(report_id: str, payload: dict[str, object]) -> dict[str, object]:
    time.sleep(0.05)
    return {"report_id": report_id, "rows": 42}


def _send_email(address: str, subject: str, body: str) -> bool:
    time.sleep(0.02)
    return True


class AuditEvent(BaseModel):
    user_id: int
    action: str


class ReportRequest(BaseModel):
    report_id: str
    payload: dict[str, object]


class EmailRequest(BaseModel):
    address: str
    subject: str
    body: str


# hunk_start_line: 55
@app.post("/audit", status_code=202)
async def log_audit(event: AuditEvent) -> dict[str, object]:
    # Paradigm break: executor.submit() instead of background_tasks.add_task()
    _future: Future[None] = _thread_pool.submit(_write_audit, event.user_id, event.action)
    del _future  # fire-and-forget; Future deliberately discarded
    return {"user_id": event.user_id, "action": event.action, "status": "logged"}


@app.post("/reports", status_code=202)
async def generate_report(req: ReportRequest) -> dict[str, object]:
    _future_r: Future[dict[str, object]] = _proc_pool.submit(
        _generate_report, req.report_id, req.payload
    )
    del _future_r
    return {"report_id": req.report_id, "status": "generating"}


@app.post("/email", status_code=202)
async def send_email(req: EmailRequest) -> dict[str, object]:
    _future_e: Future[bool] = _thread_pool.submit(_send_email, req.address, req.subject, req.body)
    del _future_e
    return {"address": req.address, "status": "email_queued"}


@app.post("/bulk-audit", status_code=202)
async def bulk_audit(events: list[AuditEvent]) -> dict[str, object]:
    futures: list[Future[None]] = [
        _thread_pool.submit(_write_audit, e.user_id, e.action) for e in events
    ]
    del futures
    return {"submitted": len(events), "status": "all_logged"}
# hunk_end_line: 87
