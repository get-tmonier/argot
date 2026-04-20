"""
Paradigm break: loop.run_in_executor() inside an async endpoint instead of BackgroundTasks.

Each async endpoint calls asyncio.get_event_loop() then awaits
loop.run_in_executor(None, blocking_func, arg) to offload synchronous I/O. This
directly awaits the result (blocking endpoint return until complete) instead of
delegating deferred work to the BackgroundTasks DI parameter and returning immediately.

Deviation axis: manual executor offload with get_event_loop() instead of injecting
BackgroundTasks and calling add_task(); the endpoint blocks waiting for the executor
future rather than returning a 202 immediately.

Corpus evidence: BackgroundTasks parameter sites = 10, add_task call sites = 10 in
the FastAPI corpus. asyncio.get_event_loop() / run_in_executor inside
endpoint-decorated functions = 0. Canonical pattern: docs_src/background_tasks/
tutorial001_py310.py lines 12-15 — background_tasks.add_task(write_notification, ...).
"""

from __future__ import annotations

import asyncio
import time
from concurrent.futures import ThreadPoolExecutor

from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

_executor = ThreadPoolExecutor(max_workers=4)


def _blocking_write_log(message: str) -> None:
    time.sleep(0.02)  # simulate slow I/O
    with open("app.log", "a") as f:
        f.write(message + "\n")


def _blocking_send_notification(user_id: int, text: str) -> bool:
    time.sleep(0.05)  # simulate external call
    return True


def _blocking_generate_thumbnail(image_path: str, width: int) -> str:
    time.sleep(0.1)
    return f"{image_path}_thumb_{width}.jpg"


class NotificationRequest(BaseModel):
    user_id: int
    message: str


class ThumbnailRequest(BaseModel):
    image_path: str
    width: int = 128


# hunk_start_line: 54
@app.post("/log")
async def log_message(message: str) -> dict[str, object]:
    # Paradigm break: run_in_executor instead of background_tasks.add_task
    loop = asyncio.get_event_loop()
    await loop.run_in_executor(_executor, _blocking_write_log, message)
    return {"logged": message}


@app.post("/notify")
async def send_notification(payload: NotificationRequest) -> dict[str, object]:
    loop = asyncio.get_event_loop()
    ok = await loop.run_in_executor(
        _executor, _blocking_send_notification, payload.user_id, payload.message
    )
    return {"user_id": payload.user_id, "sent": ok}


@app.post("/thumbnail")
async def create_thumbnail(payload: ThumbnailRequest) -> dict[str, object]:
    loop = asyncio.get_event_loop()
    thumb_path = await loop.run_in_executor(
        _executor, _blocking_generate_thumbnail, payload.image_path, payload.width
    )
    return {"thumbnail": thumb_path}
# hunk_end_line: 78
