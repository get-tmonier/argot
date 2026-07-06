# Break: prometheus_client instruments crawl metrics instead of StatsCollector
"""Break fixture — not for import."""

# hunk starts here
from prometheus_client import Counter, Histogram

_requests = Counter("scrapy_requests_total", "Total requests")
_latency = Histogram("scrapy_response_seconds", "Response latency")


def observe_response(seconds: float) -> None:
    _requests.inc()
    _latency.observe(seconds)
# hunk ends here
