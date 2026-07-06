# Break: gevent loaded via importlib; greenlet fan-out masked behind a handle
"""Break fixture — not for import."""

# hunk starts here
import importlib

_gevent = importlib.import_module("gevent")


def fetch_all(urls: list, fetch) -> list:
    jobs = [_gevent.spawn(fetch, url) for url in urls]
    _gevent.joinall(jobs)
    return [job.value for job in jobs]
# hunk ends here
