# Break: multiprocessing.Pool fan-out for export batches instead of chained Celery tasks
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

BATCH_SIZE = 5000


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def split_into_batches(ids: list[int]) -> list[list[int]]:
    return [ids[i : i + BATCH_SIZE] for i in range(0, len(ids), BATCH_SIZE)]


# hunk starts here
import multiprocessing


def _export_batch(batch: list[int]) -> int:
    from ..product.models import Product

    rows = Product.objects.filter(pk__in=batch).values("id", "name", "slug")
    return len(list(rows))


def export_products_parallel(product_ids: list[int], workers: int = 4) -> int:
    batches = split_into_batches(product_ids)
    with multiprocessing.Pool(processes=workers) as pool:
        counts = pool.map(_export_batch, batches)
        pool.close()
        pool.join()
    return sum(counts)


def export_with_queue(product_ids: list[int]) -> list[int]:
    queue: multiprocessing.Queue = multiprocessing.Queue()
    proc = multiprocessing.Process(target=queue.put, args=(len(product_ids),))
    proc.start()
    proc.join()
    return [queue.get()]
# hunk ends here
