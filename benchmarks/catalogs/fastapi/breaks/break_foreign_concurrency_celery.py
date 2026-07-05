# Break: celery loaded dynamically via importlib; no static foreign import
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import importlib

_celery_mod = importlib.import_module("celery")
_app = _celery_mod.Celery("tasks", broker="redis://localhost:6379/0")


def register_email_task():
    @_app.task(bind=True, max_retries=3)
    def send_email(self, to: str) -> None:
        ...

    return send_email
# hunk ends here
