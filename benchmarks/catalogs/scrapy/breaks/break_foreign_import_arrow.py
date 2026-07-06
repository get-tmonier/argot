# Break: arrow (aliased) formats timestamps instead of stdlib datetime
"""Break fixture — not for import."""

# hunk starts here
import arrow as ar


def next_flush_stamp(interval: int) -> str:
    now = ar.utcnow()
    return now.shift(seconds=interval).isoformat()
# hunk ends here
