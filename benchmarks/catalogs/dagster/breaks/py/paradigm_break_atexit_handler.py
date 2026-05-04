"""
Paradigm break (exception_handling): atexit callbacks and signal handlers
substituting Dagster's @failure_hook / @success_hook / RetryPolicy pattern.

Dagster registers lifecycle hooks via @failure_hook and @success_hook decorators that
receive a HookContext with access to op output and the event log; RetryPolicy governs
transient failures with typed backoff strategies.  This file instead registers
atexit.register() callbacks and a signal.signal(SIGTERM, ...) handler to respond to
pipeline teardown — a pattern familiar from long-running services but entirely absent
from Dagster's corpus.  Key absent identifiers: @failure_hook, @success_hook,
HookContext, RetryPolicy, op_failure_event, op_output_event — none of which appear
in the Dagster corpus.
"""

from __future__ import annotations

import atexit
import logging
import signal
import sys
from types import FrameType
from typing import Any

logger = logging.getLogger(__name__)

_pipeline_state: dict[str, Any] = {
    "started": False,
    "steps_completed": 0,
    "errors": [],
}


def _on_pipeline_exit() -> None:
    if _pipeline_state["errors"]:
        logger.error(
            "pipeline exited with %d error(s): %s",
            len(_pipeline_state["errors"]),
            "; ".join(str(e) for e in _pipeline_state["errors"]),
        )
    else:
        logger.info(
            "pipeline exited cleanly after %d steps",
            _pipeline_state["steps_completed"],
        )


def _handle_sigterm(signum: int, frame: FrameType | None) -> None:
    logger.warning("SIGTERM received — initiating graceful shutdown (signum=%d)", signum)
    _on_pipeline_exit()
    sys.exit(0)


atexit.register(_on_pipeline_exit)
signal.signal(signal.SIGTERM, _handle_sigterm)


def run_pipeline_step(name: str, fn: Any, *args: Any, **kwargs: Any) -> Any:
    try:
        result = fn(*args, **kwargs)
        _pipeline_state["steps_completed"] += 1
        return result
    except Exception as exc:
        _pipeline_state["errors"].append(f"{name}: {exc}")
        raise


def run_pipeline(steps: list[tuple[str, Any]]) -> None:
    _pipeline_state["started"] = True
    for name, fn in steps:
        run_pipeline_step(name, fn)
