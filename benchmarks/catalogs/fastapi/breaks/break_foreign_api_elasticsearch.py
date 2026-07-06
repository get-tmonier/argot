# Break: elasticsearch loaded dynamically via importlib; no static foreign import
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

_es_module = importlib.import_module("elasticsearch")
_es = _es_module.Elasticsearch(["http://localhost:9200"])


def index_login_event(user: str, scopes: list[str]) -> None:
    _es.index(index="logins", document={"user": user, "scopes": scopes})


def recent_logins(user: str) -> list[dict]:
    resp = _es.search(index="logins", query={"term": {"user": user}})
    return [hit["_source"] for hit in resp["hits"]["hits"]]
# hunk ends here
