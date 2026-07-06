# Break: rq.Queue.enqueue dispatches report generation to a worker
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import rq

_queue = rq.Queue("reports")


def dispatch_report(report_id: int) -> str:
    job = _queue.enqueue("reports.build", report_id)
    return job.id
# hunk ends here
