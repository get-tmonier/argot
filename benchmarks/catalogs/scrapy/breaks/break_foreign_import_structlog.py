# Break: structlog replaces scrapy's stdlib logging with structured event logs
"""Break fixture — not for import."""

# Decoy — scrapy configures logging via logging.getLogger(__name__)
# hunk starts here
import structlog

_log = structlog.get_logger("scrapy.crawl")


def log_item_scraped(spider_name: str, item_count: int) -> None:
    _log.bind(spider=spider_name).info("item_scraped", count=item_count)
# hunk ends here
