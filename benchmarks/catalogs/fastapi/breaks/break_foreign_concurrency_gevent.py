# Break: gevent spawn/joinall loaded dynamically via importlib
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

_gevent = importlib.import_module("gevent")


def run_concurrently(tasks: list) -> list:
    greenlets = [_gevent.spawn(task) for task in tasks]
    _gevent.joinall(greenlets)
    return [g.value for g in greenlets]
# hunk ends here
