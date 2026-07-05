# Break: prometheus_client Counter/Histogram record request metrics
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
from prometheus_client import Counter, Histogram

REQUESTS = Counter("http_requests_total", "Total requests", ["method", "path"])
LATENCY = Histogram("http_request_latency_seconds", "Latency", ["path"])


def observe(method: str, path: str, elapsed: float) -> None:
    REQUESTS.labels(method=method, path=path).inc()
    LATENCY.labels(path=path).observe(elapsed)
# hunk ends here
