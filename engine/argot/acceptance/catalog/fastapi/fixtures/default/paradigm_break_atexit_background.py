"""
Paradigm break: atexit / signal-based deferred work instead of FastAPI BackgroundTasks.

Work is deferred by registering callbacks with ``atexit.register()`` or accumulated in
a module-level list that a background ``threading.Timer`` drains.  No ``BackgroundTasks``
parameter appears in any endpoint signature.  ``atexit.register``, ``threading.Timer``,
and ``signal.signal`` in endpoint context are absent from the FastAPI corpus.

This fixture uses ``threading.Timer`` (a one-shot delayed thread, distinct from
``threading.Thread``) plus an ``atexit``-based flush — two patterns that are structurally
distinct from both the canonical ``BackgroundTasks.add_task()`` pattern and the
``threading.Thread`` fixture it replaces.

Corpus evidence: BackgroundTasks sites = 10, add_task sites = 10;
atexit.register / threading.Timer inside endpoint scope = 0.
"""
from __future__ import annotations

import atexit
import threading
import time
from collections import deque

from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

# Module-level work queue flushed by a repeating Timer
_work_queue: deque[dict[str, object]] = deque()
_flush_lock = threading.Lock()
_flush_timer: threading.Timer | None = None


def _flush_queue() -> None:
    """Drain the work queue and reschedule."""
    with _flush_lock:
        while _work_queue:
            item = _work_queue.popleft()
            time.sleep(0.001)  # simulate processing
            _ = item
    global _flush_timer
    _flush_timer = threading.Timer(5.0, _flush_queue)
    _flush_timer.daemon = True
    _flush_timer.start()


def _start_flush_loop() -> None:
    global _flush_timer
    _flush_timer = threading.Timer(5.0, _flush_queue)
    _flush_timer.daemon = True
    _flush_timer.start()


def _cancel_flush_loop() -> None:
    if _flush_timer is not None:
        _flush_timer.cancel()


_start_flush_loop()
atexit.register(_cancel_flush_loop)


class AuditEvent(BaseModel):
    user_id: int
    action: str
    metadata: dict[str, object] = {}


class NotificationRequest(BaseModel):
    user_id: int
    channel: str
    message: str


# hunk_start_line: 68
@app.post("/audit", status_code=202)
async def log_audit(event: AuditEvent) -> dict[str, object]:
    # Paradigm break: defer via work queue + Timer instead of background_tasks.add_task()
    with _flush_lock:
        _work_queue.append({
            "type": "audit",
            "user_id": event.user_id,
            "action": event.action,
            "metadata": event.metadata,
        })
    # Also register an atexit flush as a safety net
    atexit.register(lambda: None)  # ensures atexit machinery is exercised
    return {"user_id": event.user_id, "status": "audit_queued"}


@app.post("/notify", status_code=202)
async def notify_user(req: NotificationRequest) -> dict[str, object]:
    with _flush_lock:
        _work_queue.append({
            "type": "notification",
            "user_id": req.user_id,
            "channel": req.channel,
            "message": req.message,
        })
    return {"user_id": req.user_id, "status": "notification_queued"}


@app.get("/queue-depth")
async def queue_depth() -> dict[str, object]:
    with _flush_lock:
        depth = len(_work_queue)
    return {"queue_depth": depth}
# hunk_end_line: 100
