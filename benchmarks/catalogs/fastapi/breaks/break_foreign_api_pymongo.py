# Break: pymongo MongoClient writes an audit record
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from pymongo import MongoClient

_client = MongoClient("mongodb://localhost:27017")
_audit = _client["fastapi"]["audit"]


def record_dependency_call(name: str, resolved: bool) -> None:
    _audit.insert_one({"dependency": name, "resolved": resolved})
# hunk ends here
