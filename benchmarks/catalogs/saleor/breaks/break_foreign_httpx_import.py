# Break: httpx (imported in the hunk) replaces the hardened HTTPClient for a payment gateway call
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def gateway_headers(secret: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {secret}", "Content-Type": "application/json"}


# hunk starts here
import httpx


def authorize_payment(url: str, payload: dict) -> dict:
    with httpx.Client(timeout=httpx.Timeout(15.0), http2=True) as client:
        response = client.post(url, json=payload)
        response.raise_for_status()
        return response.json()


def refund_payment(url: str, token: str, amount: str) -> int:
    resp = httpx.post(url, json={"token": token, "amount": amount})
    return resp.status_code
# hunk ends here
