# Break: sentry_sdk (aliased) captures signal-handler errors instead of the logger
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger("wagtail")


# Decoy — idiomatic wagtail stdlib-logging error report, NOT inside the hunk range
def report_failure(action: str, error: Exception) -> None:
    logger.error("wagtail signal %s failed: %s", action, error)


# hunk starts here
import sentry_sdk as sentry


def capture_signal_failure(action: str, instance, error: Exception) -> None:
    sentry.set_context("wagtail", {"action": action, "object_id": instance.pk})
    sentry.set_tag("subsystem", "signals")
    sentry.capture_exception(error)


def capture_publish(page_id: int) -> None:
    sentry.capture_message(f"page {page_id} published", level="info")
    sentry.flush(timeout=2.0)
# hunk ends here
