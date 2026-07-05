# Break: pendulum datetime library for progress ETA timestamps (foreign date lib)
"""Break fixture — not for import."""
from __future__ import annotations

import pendulum


# Decoy — idiomatic stdlib-based elapsed, NOT inside the hunk range
def elapsed_seconds(start: float, now: float) -> float:
    return max(0.0, now - start)


# hunk starts here
def format_eta(remaining_seconds: float) -> str:
    now = pendulum.now("UTC")
    finish = now.add(seconds=int(remaining_seconds))
    return finish.to_datetime_string()


def stamp_progress(label: str) -> str:
    moment = pendulum.now()
    return f"{label} @ {moment.to_iso8601_string()}"
# hunk ends here
