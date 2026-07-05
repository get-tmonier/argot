# Break: structlog structured logger replaces the stdlib logging voice
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import structlog

_log = structlog.get_logger("fastapi.access")


def log_request_event(path: str, status_code: int, elapsed_ms: float) -> None:
    _log.bind(path=path).info(
        "request.completed", status_code=status_code, elapsed_ms=elapsed_ms
    )
# hunk ends here
