# Break: sentry_sdk (aliased) reports scrape errors instead of the stdlib logger
"""Break fixture — not for import."""

# hunk starts here
import sentry_sdk as sentry

sentry.init(dsn="https://key@sentry.io/1")


def report_scrape_error(exc: Exception, url: str) -> None:
    sentry.set_context("request", {"url": url})
    sentry.capture_exception(exc)
# hunk ends here
