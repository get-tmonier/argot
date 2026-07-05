# Break: APScheduler BackgroundScheduler (submodule import in the hunk) schedules recalculation, replacing Celery beat
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def recalculation_interval_seconds(order_count: int) -> int:
    return 60 if order_count < 100 else 300


# hunk starts here
from apscheduler.schedulers.background import BackgroundScheduler


def schedule_recalculations(recalc, interval: int) -> BackgroundScheduler:
    scheduler = BackgroundScheduler()
    scheduler.add_job(recalc, "interval", seconds=interval, id="recalc-orders")
    scheduler.start()
    return scheduler
# hunk ends here
