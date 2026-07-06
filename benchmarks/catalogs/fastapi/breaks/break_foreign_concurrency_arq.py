# Break: arq create_pool + enqueue_job via receiver, submodule import
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from arq import create_pool
from arq.connections import RedisSettings


async def enqueue_job(job_name: str, payload: dict) -> str:
    pool = await create_pool(RedisSettings())
    job = await pool.enqueue_job(job_name, payload)
    return job.job_id
# hunk ends here
