# Break: psycopg2 loaded dynamically; leaf .execute/.fetchone collide with attested methods
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

_pg = importlib.import_module("psycopg2")


def count_active_sessions(dsn: str) -> int:
    conn = _pg.connect(dsn)
    cur = conn.cursor()
    cur.execute("SELECT count(*) FROM sessions WHERE active")
    (total,) = cur.fetchone()
    conn.close()
    return total
# hunk ends here
