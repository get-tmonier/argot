"""
Paradigm break: module-level asyncio.Queue used as a background job queue.

A raw asyncio.Queue() is declared at module level. Endpoints put items onto the queue
with queue.put_nowait(). A startup event handler creates an asyncio task that runs a
worker loop draining the queue. This is a pattern carried over from vanilla asyncio
apps — in FastAPI's corpus, deferred work uses BackgroundTasks, not an explicit queue.

Deviation axis: module-level queue + on_event("startup") worker instead of the
BackgroundTasks DI parameter.

Corpus evidence: BackgroundTasks parameter sites = 10, add_task call sites = 10 in
the FastAPI corpus. Module-level asyncio.Queue() combined with a startup worker = 0.
@app.on_event("startup") is present in the corpus (valid FastAPI) but not paired with
a queue-draining pattern. Canonical pattern: docs_src/background_tasks/tutorial001_py310.py
lines 12-15 — background_tasks.add_task() from an injected BackgroundTasks parameter.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any

from fastapi import FastAPI
from pydantic import BaseModel

logger = logging.getLogger(__name__)

app = FastAPI()

# Module-level queue — raw asyncio carryover pattern
_job_queue: asyncio.Queue[dict[str, Any]] = asyncio.Queue()


async def _process_email(job: dict[str, Any]) -> None:
    await asyncio.sleep(0.05)
    logger.info("Sent email to %s: %s", job.get("email"), job.get("subject"))


async def _process_report(job: dict[str, Any]) -> None:
    await asyncio.sleep(0.1)
    logger.info("Generated report %s for user %s", job.get("report_id"), job.get("user_id"))


async def _worker() -> None:
    while True:
        job = await _job_queue.get()
        kind = job.get("kind")
        try:
            if kind == "email":
                await _process_email(job)
            elif kind == "report":
                await _process_report(job)
        except Exception:
            logger.exception("Worker failed on job %r", job)
        finally:
            _job_queue.task_done()


# hunk_start_line: 54
@app.on_event("startup")
async def startup_event() -> None:
    # Paradigm break: asyncio.create_task used to start queue-draining worker
    asyncio.create_task(_worker())


class EmailJob(BaseModel):
    user_id: int
    email: str
    subject: str
    body: str


class ReportJob(BaseModel):
    user_id: int
    report_id: str


@app.post("/send-email", status_code=202)
async def enqueue_email(payload: EmailJob) -> dict[str, object]:
    _job_queue.put_nowait(
        {
            "kind": "email",
            "user_id": payload.user_id,
            "email": payload.email,
            "subject": payload.subject,
            "body": payload.body,
        }
    )
    return {"status": "queued", "user_id": payload.user_id}


@app.post("/generate-report", status_code=202)
async def enqueue_report(payload: ReportJob) -> dict[str, object]:
    _job_queue.put_nowait(
        {
            "kind": "report",
            "user_id": payload.user_id,
            "report_id": payload.report_id,
        }
    )
    return {"status": "queued", "report_id": payload.report_id}


@app.get("/queue-size")
async def queue_size() -> dict[str, int]:
    return {"pending": _job_queue.qsize()}
# hunk_end_line: 97
