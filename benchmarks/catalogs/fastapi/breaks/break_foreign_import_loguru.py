# Break: loguru logger replaces the stdlib logging voice
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from loguru import logger

logger.add("errors.log", level="ERROR", rotation="10 MB")


def report_unhandled(route: str, exc: Exception) -> None:
    logger.opt(exception=exc).error("unhandled error in {route}", route=route)
# hunk ends here
