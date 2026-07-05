# Break: dramatiq actor (imported in the hunk) replaces the Celery @app.task for deferred recalculation
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def should_recalculate(order) -> bool:
    return order.should_refresh_prices


# hunk starts here
import dramatiq


@dramatiq.actor(max_retries=3, queue_name="order-pricing")
def recalculate_order_task(order_id: int) -> None:
    logger.info("recalculating order %s", order_id)


def schedule_recalculation(order_id: int) -> None:
    recalculate_order_task.send(order_id)
    dramatiq.get_broker().flush_all()
# hunk ends here
