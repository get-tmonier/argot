# Break: cassandra.cluster loaded dynamically via importlib + getattr
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

_cluster_mod = importlib.import_module("cassandra.cluster")
_Cluster = getattr(_cluster_mod, "Cluster")
_session = _Cluster(["127.0.0.1"]).connect("fastapi")


def store_event(event_id: str, kind: str) -> None:
    _session.execute(
        "INSERT INTO events (id, kind) VALUES (%s, %s)", (event_id, kind)
    )
# hunk ends here
