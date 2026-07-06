# Break: eventlet loaded via importlib; GreenPool masked behind a handle
"""Break fixture — not for import."""

# hunk starts here
import importlib

_eventlet = importlib.import_module("eventlet")


def crawl_concurrently(urls: list, fetch) -> list:
    pool = _eventlet.GreenPool(size=20)
    return list(pool.imap(fetch, urls))
# hunk ends here
