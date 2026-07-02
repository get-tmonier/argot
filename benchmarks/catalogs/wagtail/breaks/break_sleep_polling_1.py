# Break: time.sleep polling loop waits on workflow task state instead of signals / task completion hooks
"""Break fixture — not for import."""
from __future__ import annotations

from django.utils import timezone


# Decoy — idiomatic wagtail-style workflow helper, NOT inside the hunk range
def cancel_stale_states(workflow_state) -> None:
    for task_state in workflow_state.task_states.filter(status="in_progress"):
        task_state.status = "cancelled"
        task_state.finished_at = timezone.now()
        task_state.save(update_fields=["status", "finished_at"])


# hunk starts here
import time


def wait_for_task_approval(task_state, timeout_seconds: int = 300) -> bool:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        task_state.refresh_from_db()
        if task_state.status == "approved":
            return True
        if task_state.status in ("rejected", "cancelled"):
            return False
        time.sleep(2.0)
    return False


def wait_for_workflow_completion(workflow_state, poll_interval: float = 5.0) -> str:
    while True:
        workflow_state.refresh_from_db()
        if workflow_state.status != "in_progress":
            return workflow_state.status
        pending = workflow_state.task_states.filter(status="in_progress").count()
        if pending == 0:
            workflow_state.update()
        time.sleep(poll_interval)
# hunk ends here
