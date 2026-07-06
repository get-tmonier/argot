# Break: aliased pandas import aggregates metric rows
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import pandas as pd


def summarise_metrics(rows: list[dict]) -> dict:
    frame = pd.DataFrame(rows)
    grouped = frame.groupby("endpoint")["latency_ms"].mean()
    return grouped.to_dict()
# hunk ends here
