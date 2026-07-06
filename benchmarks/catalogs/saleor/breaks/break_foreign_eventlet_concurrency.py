# Break: eventlet GreenPool (import kept outside the hunk) fans out recalculation via a green-thread pool, replacing Celery
"""Break fixture — not for import."""

import logging

import eventlet

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def should_recalculate(order) -> bool:
    return order.should_refresh_prices


# hunk starts here
def recalculate_orders_concurrently(order_ids: list[int], recalc) -> None:
    pool = eventlet.GreenPool(size=20)
    for order_id in order_ids:
        pool.spawn_n(recalc, order_id)
    pool.waitall()
# hunk ends here
