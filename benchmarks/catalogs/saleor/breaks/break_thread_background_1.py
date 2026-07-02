# Break: daemon threading.Thread spawned per request for deferred work instead of a Celery task
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def order_lines_need_refresh(order) -> bool:
    return order.should_refresh_prices and order.lines.exists()


# hunk starts here
import threading


def _recalculate_worker(order_id: int, results: dict[int, bool], lock: threading.Lock) -> None:
    from ..order.models import Order

    order = Order.objects.get(pk=order_id)
    total = sum(line.total_price_gross_amount for line in order.lines.all())
    with lock:
        results[order_id] = total == order.total_gross_amount


def recalculate_orders_in_background(order_ids: list[int]) -> dict[int, bool]:
    results: dict[int, bool] = {}
    lock = threading.Lock()
    threads: list[threading.Thread] = []
    for order_id in order_ids:
        t = threading.Thread(
            target=_recalculate_worker,
            args=(order_id, results, lock),
            daemon=True,
        )
        threads.append(t)
        t.start()
    for t in threads:
        t.join(timeout=30)
    return results
# hunk ends here
