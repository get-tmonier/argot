# Break: APScheduler background jobs poll workflow tasks off the request cycle
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import TaskState


# Decoy — idiomatic wagtail ORM task-state lookup, NOT inside the hunk range
def in_progress_states():
    return TaskState.objects.filter(status="in_progress")


# hunk starts here
from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.interval import IntervalTrigger

_scheduler = BackgroundScheduler()


def _sweep_stale_tasks() -> None:
    for state in TaskState.objects.filter(status="in_progress"):
        state.cancel()


def start_workflow_sweeper(minutes: int = 5) -> None:
    _scheduler.add_job(_sweep_stale_tasks, IntervalTrigger(minutes=minutes))
    _scheduler.start()


def stop_workflow_sweeper() -> None:
    _scheduler.shutdown(wait=False)
# hunk ends here
