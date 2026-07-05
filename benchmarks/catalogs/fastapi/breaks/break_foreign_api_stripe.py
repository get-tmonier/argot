# Break: stripe.PaymentIntent.create charges an order
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import stripe

stripe.api_key = "sk_test_example"


def charge_order(amount_cents: int, token: str) -> str:
    intent = stripe.PaymentIntent.create(
        amount=amount_cents, currency="usd", payment_method=token, confirm=True
    )
    return intent.id
# hunk ends here
