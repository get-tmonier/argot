# Break: eventlet GreenPool loaded dynamically via importlib
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

_eventlet = importlib.import_module("eventlet")
_pool = _eventlet.GreenPool(size=100)


def fan_out(urls: list[str], fetch) -> list:
    return list(_pool.imap(fetch, urls))
# hunk ends here
