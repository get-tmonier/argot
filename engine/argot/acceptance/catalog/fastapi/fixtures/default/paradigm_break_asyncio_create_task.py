"""
Paradigm break: asyncio.create_task() inside an endpoint instead of BackgroundTasks.

The endpoint is an async def but accepts no BackgroundTasks parameter. Instead it
calls asyncio.create_task(send_email(user_id)) directly, creating a fire-and-forget
coroutine task. The task is not tracked or handled on error.

Deviation axis: spawns background work via asyncio.create_task rather than the
FastAPI-idiomatic BackgroundTasks DI parameter.

Corpus evidence: BackgroundTasks parameter sites = 10, add_task call sites = 10 in
the FastAPI corpus. asyncio.create_task inside endpoint-decorated functions = 0.
Canonical pattern: docs_src/background_tasks/tutorial001_py310.py lines 12-15 uses
background_tasks.add_task(write_notification, email, ...) from a BackgroundTasks param.
"""

from __future__ import annotations

import asyncio
import smtplib
from email.message import EmailMessage

from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()


async def send_email(user_id: int, address: str, subject: str, body: str) -> None:
    await asyncio.sleep(0.05)  # simulate async delay
    msg = EmailMessage()
    msg["Subject"] = subject
    msg["From"] = "noreply@example.com"
    msg["To"] = address
    msg.set_content(body)
    try:
        with smtplib.SMTP("localhost", 1025) as smtp:
            smtp.send_message(msg)
    except OSError:
        pass


async def send_welcome_email(user_id: int, email: str) -> None:
    await send_email(user_id, email, "Welcome!", f"Hi user {user_id}, welcome aboard.")


async def send_password_reset(user_id: int, email: str, token: str) -> None:
    await send_email(
        user_id, email, "Password reset", f"Use token {token} to reset your password."
    )


class UserRegistration(BaseModel):
    user_id: int
    email: str


class PasswordResetRequest(BaseModel):
    user_id: int
    email: str
    token: str


# hunk_start_line: 54
@app.post("/register", status_code=201)
async def register_user(payload: UserRegistration) -> dict[str, object]:
    # Paradigm break: asyncio.create_task instead of background_tasks.add_task
    asyncio.create_task(send_welcome_email(payload.user_id, payload.email))
    return {"user_id": payload.user_id, "status": "registered"}


@app.post("/password-reset")
async def request_password_reset(payload: PasswordResetRequest) -> dict[str, object]:
    asyncio.create_task(send_password_reset(payload.user_id, payload.email, payload.token))
    return {"user_id": payload.user_id, "status": "reset_email_queued"}


@app.post("/notify/{user_id}")
async def notify_user(user_id: int, message: str = "") -> dict[str, object]:
    asyncio.create_task(
        send_email(user_id, f"user{user_id}@example.com", "Notification", message)
    )
    return {"user_id": user_id, "status": "notified"}
# hunk_end_line: 73
