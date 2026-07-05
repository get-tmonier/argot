# Break: aliased ujson serializes payloads instead of the JSON stack
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import ujson as ujson_fast


def dump_payload(data: dict) -> str:
    return ujson_fast.dumps(data, ensure_ascii=False)


def load_payload(raw: str) -> dict:
    return ujson_fast.loads(raw)
# hunk ends here
