"""
Paradigm break (scheduling): APScheduler BlockingScheduler substituting Dagster's
ScheduleDefinition / @schedule decorator pattern.

Dagster declares schedules as ScheduleDefinition objects (or @schedule-decorated
functions) registered in Definitions(); the scheduler is managed by Dagster's daemon
process and execution history is tracked in the event log.  APScheduler instead uses
a BlockingScheduler or BackgroundScheduler from apscheduler.schedulers; jobs are
registered via scheduler.add_job(fn, trigger=CronTrigger(...)) and run in the same
process or a thread pool.  Key absent identifiers: ScheduleDefinition, @schedule,
DefaultScheduleStatus, RunRequest, cron_schedule, job_def — none of which appear
in the Dagster corpus.
"""

from __future__ import annotations

import logging

from apscheduler.schedulers.blocking import BlockingScheduler
from apscheduler.triggers.cron import CronTrigger

logger = logging.getLogger(__name__)
scheduler = BlockingScheduler(timezone="UTC")


def _aggregate_daily_metrics(window: str) -> None:
    pass


def _flush_sensor_buffer() -> None:
    pass


def _build_weekly_report() -> None:
    pass


def _run_nightly_aggregation() -> None:
    logger.info("starting nightly aggregation job")
    _aggregate_daily_metrics("yesterday")
    logger.info("nightly aggregation complete")


def _run_hourly_sensor_flush() -> None:
    logger.info("flushing sensor buffer")
    _flush_sensor_buffer()
    logger.info("sensor flush complete")


def _run_weekly_report() -> None:
    logger.info("generating weekly summary report")
    _build_weekly_report()
    logger.info("weekly report sent")


scheduler.add_job(
    _run_nightly_aggregation,
    trigger=CronTrigger(hour=2, minute=0),
    id="nightly_aggregation",
    replace_existing=True,
)
scheduler.add_job(
    _run_hourly_sensor_flush,
    trigger=CronTrigger(minute=5),
    id="hourly_sensor_flush",
    replace_existing=True,
)
scheduler.add_job(
    _run_weekly_report,
    trigger=CronTrigger(day_of_week="mon", hour=8, minute=0),
    id="weekly_report",
    replace_existing=True,
)

if __name__ == "__main__":
    scheduler.start()
