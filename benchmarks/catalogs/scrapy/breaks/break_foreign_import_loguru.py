# Break: loguru sink replaces scrapy's LogFormatter / stdlib logging
"""Break fixture — not for import."""

# hunk starts here
from loguru import logger

logger.add("scrapy_events.log", rotation="10 MB", level="INFO")


def log_dropped(reason: str) -> None:
    logger.opt(lazy=True).warning("dropped item: {r}", r=reason)
# hunk ends here
