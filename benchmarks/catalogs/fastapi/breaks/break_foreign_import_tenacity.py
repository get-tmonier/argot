# Break: tenacity @retry decorator wraps a loader with exponential backoff
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import tenacity
from tenacity import retry, stop_after_attempt, wait_exponential


@retry(stop=stop_after_attempt(3), wait=wait_exponential(multiplier=0.5))
def load_openapi_extension(loader, name: str) -> dict:
    schema = loader(name)
    if not schema:
        raise tenacity.TryAgain
    return schema
# hunk ends here
