"""
Paradigm break (scheduling): croniter + threading.Timer loop substituting Dagster's
ScheduleDefinition / @schedule decorator pattern.

Dagster expresses time-based automation via ScheduleDefinition with a cron_schedule
string and an execution_fn that yields RunRequest objects; the Dagster daemon evaluates
the schedule and launches runs through the run launcher.  This file instead uses
croniter objects to compute the next fire time and sleeps a threading.Event until that
instant to invoke plain Python functions directly in-process.  Key absent identifiers:
ScheduleDefinition, @schedule, RunRequest, DefaultScheduleStatus, execution_timezone,
cron_schedule field, job_name — none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import logging
import threading
import time
from collections.abc import Callable
from datetime import UTC, datetime
from typing import Any

from croniter import croniter

logger = logging.getLogger(__name__)
_scheduler_stop = threading.Event()


def _next_delay(cron_expr: str) -> float:
    itr = croniter(cron_expr, datetime.now(UTC))
    next_dt: datetime = itr.get_next(datetime)
    return max(0.0, (next_dt - datetime.now(UTC)).total_seconds())


def _schedule_loop(name: str, cron_expr: str, fn: Callable[[], None]) -> None:
    while not _scheduler_stop.is_set():
        delay = _next_delay(cron_expr)
        logger.info("job %s sleeping %.1fs until next fire", name, delay)
        if _scheduler_stop.wait(delay):
            break
        logger.info("running scheduled job: %s", name)
        try:
            fn()
        except Exception:
            logger.exception("job %s raised an unhandled exception", name)


def start_scheduler(jobs: dict[str, tuple[str, Callable[[], None]]]) -> list[threading.Thread]:
    threads: list[threading.Thread] = []
    for name, (cron_expr, fn) in jobs.items():
        t = threading.Thread(target=_schedule_loop, args=(name, cron_expr, fn), daemon=True)
        t.start()
        threads.append(t)
    return threads


def _materialize_daily_snapshot() -> None:
    logger.info("materializing daily snapshot")


def _refresh_feature_store() -> None:
    logger.info("refreshing feature store")


if __name__ == "__main__":
    _jobs: dict[str, tuple[str, Any]] = {
        "daily_snapshot": ("0 3 * * *", _materialize_daily_snapshot),
        "hourly_features": ("0 * * * *", _refresh_feature_store),
    }
    start_scheduler(_jobs)
    try:
        while True:
            time.sleep(60)
    except KeyboardInterrupt:
        _scheduler_stop.set()
