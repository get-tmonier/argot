"""
Control: canonical BackgroundTasks parameter in FastAPI endpoints.

Demonstrates the idiomatic pattern for deferred work in FastAPI:
- BackgroundTasks injected as a typed parameter in the endpoint signature
- background_tasks.add_task(func, *args, **kwargs) schedules the work
- Endpoint returns immediately with a 202/200 response
- Task function is a plain def (sync) — FastAPI runs it in a threadpool

Adapted from docs_src/background_tasks/tutorial001_py310.py (lines 1-15).
"""

from __future__ import annotations

import logging
import time

from fastapi import BackgroundTasks, FastAPI
from pydantic import BaseModel

logger = logging.getLogger(__name__)

app = FastAPI()


def write_notification(email: str, message: str = "") -> None:
    """Simulate sending a notification email (blocking I/O)."""
    time.sleep(0.02)
    logger.info("notification for %s: %s", email, message)


def write_audit_log(user_id: int, action: str) -> None:
    """Simulate writing an audit log entry."""
    time.sleep(0.01)
    logger.info("audit user=%d action=%s", user_id, action)


class NotificationPayload(BaseModel):
    email: str
    message: str = ""


class UserAction(BaseModel):
    user_id: int
    action: str


# hunk_start_line: 43
@app.post("/send-notification/{email}", status_code=202)
async def send_notification(email: str, background_tasks: BackgroundTasks) -> dict[str, str]:
    background_tasks.add_task(write_notification, email, message="some notification")
    return {"message": "Notification sent in the background"}


@app.post("/notify", status_code=202)
async def notify_user(
    payload: NotificationPayload, background_tasks: BackgroundTasks
) -> dict[str, str]:
    background_tasks.add_task(write_notification, payload.email, message=payload.message)
    return {"message": "Notification queued"}


@app.post("/audit", status_code=202)
async def record_action(
    payload: UserAction, background_tasks: BackgroundTasks
) -> dict[str, str]:
    background_tasks.add_task(write_audit_log, payload.user_id, payload.action)
    return {"message": "Audit log scheduled"}


@app.post("/notify-and-audit", status_code=202)
async def notify_and_audit(
    payload: UserAction, background_tasks: BackgroundTasks
) -> dict[str, str]:
    background_tasks.add_task(write_audit_log, payload.user_id, payload.action)
    background_tasks.add_task(
        write_notification, f"user{payload.user_id}@example.com", message=payload.action
    )
    return {"message": "Notification and audit scheduled"}
# hunk_end_line: 73
