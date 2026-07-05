# Break: RQ (Redis Queue) enqueue (imported in the hunk) replaces the Celery export task chain
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def export_file_name(export_id: int) -> str:
    return f"export-{export_id}.csv"


# hunk starts here
from redis import Redis
from rq import Queue, Retry


def enqueue_export_batches(export_id: int, batches: list[list[int]]) -> list[str]:
    queue = Queue("exports", connection=Redis())
    job_ids = []
    for batch in batches:
        job = queue.enqueue(
            "saleor.csv.tasks.export_batch",
            export_id,
            batch,
            retry=Retry(max=3),
        )
        job_ids.append(job.id)
    return job_ids
# hunk ends here
