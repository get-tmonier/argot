# Break: time.sleep polling loop waits for payment confirmation inline instead of event-driven flow
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def payment_is_final(payment) -> bool:
    return payment.charge_status in {"full", "refused"}


# hunk starts here
import time


def wait_for_payment_confirmation(payment, max_attempts: int = 30) -> bool:
    attempts = 0
    while attempts < max_attempts:
        payment.refresh_from_db()
        if payment.charge_status == "full":
            return True
        if payment.charge_status == "refused":
            return False
        time.sleep(2)
        attempts += 1
    return False


def block_until_transaction_settles(transaction_item, poll_seconds: float = 1.5) -> str:
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        transaction_item.refresh_from_db()
        if transaction_item.charged_value > 0:
            return "charged"
        time.sleep(poll_seconds)
    return "timeout"
# hunk ends here
