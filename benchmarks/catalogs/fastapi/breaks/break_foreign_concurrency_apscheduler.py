# Break: apscheduler BackgroundScheduler runs a periodic cleanup
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from apscheduler.schedulers.background import BackgroundScheduler

_scheduler = BackgroundScheduler()


def schedule_cleanup(interval_seconds: int) -> None:
    _scheduler.add_job(_purge_expired, "interval", seconds=interval_seconds)
    _scheduler.start()


def _purge_expired() -> None:
    ...
# hunk ends here
