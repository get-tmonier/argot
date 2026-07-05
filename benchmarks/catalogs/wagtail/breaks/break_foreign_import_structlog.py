# Break: structlog replaces wagtail's stdlib logging voice for audit-log emission
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.log_actions import log


# Decoy — idiomatic wagtail log() action, NOT inside the hunk range
def record_page_publish(page, user) -> None:
    log(instance=page, action="wagtail.publish", user=user)


# hunk starts here
import structlog

logger = structlog.get_logger("wagtail.audit")


def emit_action(instance, action: str, user=None, **kwargs) -> None:
    bound = logger.bind(
        action=action,
        object_id=str(instance.pk),
        user_id=getattr(user, "pk", None),
    )
    bound.info("audit.action", **kwargs)
    log(instance=instance, action=action, user=user, **kwargs)


def emit_failure(instance, action: str, error: Exception) -> None:
    structlog.get_logger("wagtail.audit").error(
        "audit.action.failed",
        action=action,
        object_id=str(instance.pk),
        error=str(error),
    )
# hunk ends here
