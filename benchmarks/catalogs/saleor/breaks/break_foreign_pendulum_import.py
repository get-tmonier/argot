# Break: pendulum aliased import (import pendulum as pdl) replaces django.utils.timezone for order timestamps
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def order_age_label(days: int) -> str:
    return "new" if days < 7 else "aged"


# hunk starts here
import pendulum as pdl


def order_settlement_window(placed_at: str, tz_name: str) -> dict:
    placed = pdl.from_format(placed_at, "YYYY-MM-DD HH:mm:ss")
    window = pdl.duration(days=14)
    due = placed + window
    now = pdl.today(tz=tz_name)
    return {
        "due_at": due.to_iso8601_string(),
        "overdue": now > due,
    }
# hunk ends here
