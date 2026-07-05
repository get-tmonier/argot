# Break: dramatiq actor offloads email off the request cycle
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import dramatiq


@dramatiq.actor(max_retries=3)
def send_welcome_email(user_id: int) -> None:
    ...


def enqueue_welcome(user_id: int) -> None:
    send_welcome_email.send(user_id)
# hunk ends here
