"""
Paradigm break: multiprocessing.Process for background work instead of FastAPI BackgroundTasks.

Each endpoint spawns a ``multiprocessing.Process`` with ``daemon=True`` to offload
work rather than using FastAPI's ``BackgroundTasks`` DI parameter.  The process is
started and immediately detached.  ``multiprocessing.Process``, ``multiprocessing.Queue``,
``Process.start()``, ``Process.daemon``, and ``mp.set_start_method`` are all absent
from the FastAPI corpus.

Corpus evidence: BackgroundTasks parameter sites = 10, add_task call sites = 10;
multiprocessing.Process inside endpoint-decorated functions = 0.
"""
from __future__ import annotations

import multiprocessing
import time
from multiprocessing import Queue

from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

_result_queue: Queue[dict[str, object]] = Queue()


def _process_report(job_id: str, params: dict[str, object], q: Queue[dict[str, object]]) -> None:
    """Worker function run in a child process."""
    time.sleep(0.05)  # simulate work
    q.put({"job_id": job_id, "status": "done", "params": params})


def _send_notification(user_id: int, message: str, q: Queue[dict[str, object]]) -> None:
    time.sleep(0.02)
    q.put({"user_id": user_id, "delivered": True, "message": message})


class ReportRequest(BaseModel):
    job_id: str
    params: dict[str, object]


class NotificationRequest(BaseModel):
    user_id: int
    message: str


# hunk_start_line: 45
@app.post("/reports", status_code=202)
async def submit_report(req: ReportRequest) -> dict[str, object]:
    # Paradigm break: multiprocessing.Process instead of background_tasks.add_task
    proc = multiprocessing.Process(
        target=_process_report,
        args=(req.job_id, req.params, _result_queue),
        daemon=True,
    )
    proc.start()
    return {"job_id": req.job_id, "status": "queued"}


@app.post("/notify", status_code=202)
async def notify_user(req: NotificationRequest) -> dict[str, object]:
    proc = multiprocessing.Process(
        target=_send_notification,
        args=(req.user_id, req.message, _result_queue),
        daemon=True,
    )
    proc.start()
    return {"user_id": req.user_id, "status": "notification_queued"}


@app.post("/bulk-notify", status_code=202)
async def bulk_notify(requests: list[NotificationRequest]) -> dict[str, object]:
    procs: list[multiprocessing.Process] = []
    for req in requests:
        proc = multiprocessing.Process(
            target=_send_notification,
            args=(req.user_id, req.message, _result_queue),
            daemon=True,
        )
        proc.start()
        procs.append(proc)
    return {"submitted": len(procs), "status": "all_queued"}


@app.get("/results")
async def get_results() -> dict[str, object]:
    collected: list[dict[str, object]] = []
    while not _result_queue.empty():
        collected.append(_result_queue.get_nowait())
    return {"results": collected, "count": len(collected)}
# hunk_end_line: 90
