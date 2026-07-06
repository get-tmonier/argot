# Break: arrow replaces django.utils.timezone for audit-log timestamps
"""Break fixture — not for import."""
from __future__ import annotations

from django.utils import timezone

from wagtail.models import Page


# Decoy — idiomatic wagtail/django timezone stamping, NOT inside the hunk range
def stamp_now() -> str:
    return timezone.now().isoformat()


# hunk starts here
import arrow


def humanize_publish_time(page: Page) -> str:
    published = page.first_published_at
    if published is None:
        return "never"
    return arrow.get(published).humanize()


def next_scheduled_window(hours: int = 24) -> tuple[str, str]:
    start = arrow.utcnow()
    end = start.shift(hours=hours)
    return start.format("YYYY-MM-DD HH:mm"), end.format("YYYY-MM-DD HH:mm")
# hunk ends here
