# Break: inline retry loop with time.sleep exponential backoff instead of Celery self.retry countdown
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

MAX_DELIVERY_ATTEMPTS = 5


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def delivery_is_retryable(status_code: int) -> bool:
    return status_code in {408, 429, 500, 502, 503, 504}


# hunk starts here
import time


def send_delivery_with_backoff(send_fn, delivery) -> bool:
    backoff = 1.0
    for attempt in range(MAX_DELIVERY_ATTEMPTS):
        response = send_fn(delivery)
        if response.status_code < 400:
            return True
        if not delivery_is_retryable(response.status_code):
            return False
        logger.info("attempt %s failed, sleeping %.1fs", attempt + 1, backoff)
        time.sleep(backoff)
        backoff = min(backoff * 2, 30.0)
    return False


def drain_pending_deliveries(send_fn, deliveries) -> int:
    sent = 0
    for delivery in deliveries:
        if send_delivery_with_backoff(send_fn, delivery):
            sent += 1
        time.sleep(0.25)
    return sent
# hunk ends here
