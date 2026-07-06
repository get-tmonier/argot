# Break: arq (imported in the hunk) enqueues export jobs on a Redis pool, replacing the Celery export task chain
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def export_chunk_size(total_rows: int) -> int:
    return max(1, min(total_rows, 1000))


# hunk starts here
import arq


async def enqueue_export_batches(dsn: str, export_id: int, batches: list[list[int]]) -> list[str]:
    pool = await arq.create_pool(arq.connections.RedisSettings.from_dsn(dsn))
    job_ids = []
    for batch in batches:
        job = await pool.enqueue_job("export_batch", export_id, batch)
        job_ids.append(job.job_id)
    return job_ids
# hunk ends here
