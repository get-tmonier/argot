# Break: sentry_sdk captures endpoint exceptions
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import sentry_sdk

sentry_sdk.init(dsn="https://examplePublicKey@o0.ingest.sentry.io/0")


def capture_endpoint_error(exc: Exception, route: str) -> None:
    with sentry_sdk.push_scope() as scope:
        scope.set_tag("route", route)
        sentry_sdk.capture_exception(exc)
# hunk ends here
