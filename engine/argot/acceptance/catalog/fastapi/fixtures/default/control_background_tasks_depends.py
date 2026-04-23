"""
Control: BackgroundTasks injected through a Depends() sub-dependency.

The DI-aware pattern for BackgroundTasks: a helper class wraps add_task and is
constructed by a Depends() factory that receives BackgroundTasks from FastAPI.
Endpoints depend on the helper rather than taking BackgroundTasks directly,
keeping task scheduling encapsulated and testable.

Adapted from docs_src/background_tasks/tutorial002_an_py310.py (lines 1-26).
Pattern: dependency function accepts background_tasks: BackgroundTasks, wraps it,
endpoint Depends on the wrapper — BackgroundTasks flows through the DI graph.
"""

from __future__ import annotations

import logging
import time
from typing import Annotated

from fastapi import BackgroundTasks, Depends, FastAPI
from pydantic import BaseModel

logger = logging.getLogger(__name__)

app = FastAPI()


# --- background task functions ---


def _send_email(address: str, subject: str, body: str) -> None:
    time.sleep(0.02)
    logger.info("email to=%s subject=%s", address, subject)


def _write_audit(user_id: int, event: str) -> None:
    time.sleep(0.01)
    logger.info("audit user=%d event=%s", user_id, event)


# --- DI-aware helper ---


class NotificationService:
    """Wraps BackgroundTasks; keeps scheduling logic off the endpoint."""

    def __init__(self, background_tasks: BackgroundTasks) -> None:
        self._bt = background_tasks

    def schedule_email(self, address: str, subject: str, body: str) -> None:
        self._bt.add_task(_send_email, address, subject, body)

    def schedule_audit(self, user_id: int, event: str) -> None:
        self._bt.add_task(_write_audit, user_id, event)


def get_notification_service(
    background_tasks: BackgroundTasks,
) -> NotificationService:
    return NotificationService(background_tasks)


NotificationServiceDep = Annotated[NotificationService, Depends(get_notification_service)]


# --- models ---


class RegistrationPayload(BaseModel):
    user_id: int
    email: str


class OrderPayload(BaseModel):
    user_id: int
    order_id: str
    email: str


# hunk_start_line: 70
@app.post("/register", status_code=201)
async def register_user(
    payload: RegistrationPayload,
    svc: NotificationServiceDep,
) -> dict[str, object]:
    svc.schedule_email(
        payload.email, "Welcome!", f"Hi user {payload.user_id}, welcome aboard."
    )
    svc.schedule_audit(payload.user_id, "register")
    return {"user_id": payload.user_id, "status": "registered"}


@app.post("/order", status_code=202)
async def place_order(
    payload: OrderPayload,
    svc: NotificationServiceDep,
) -> dict[str, object]:
    svc.schedule_email(
        payload.email,
        f"Order {payload.order_id} received",
        f"We have received your order {payload.order_id}.",
    )
    svc.schedule_audit(payload.user_id, f"order:{payload.order_id}")
    return {"order_id": payload.order_id, "status": "processing"}


@app.delete("/account/{user_id}", status_code=200)
async def delete_account(
    user_id: int,
    svc: NotificationServiceDep,
) -> dict[str, object]:
    svc.schedule_audit(user_id, "account_deleted")
    return {"user_id": user_id, "status": "deleted"}
# hunk_end_line: 101
