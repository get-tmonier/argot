# Break: print() debugging and manual json.dumps HttpResponse instead of logger + JsonResponse
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def checkout_is_ready(checkout) -> bool:
    return checkout.shipping_address is not None and checkout.email is not None


# hunk starts here
import json

from django.http import HttpResponse


def complete_checkout_debug(request, checkout):
    print("=== complete_checkout called ===")
    print("checkout token:", checkout.token)
    print("user:", request.user)
    print("lines:", [str(line) for line in checkout.lines.all()])
    try:
        total = checkout.total.gross.amount
        print("total amount ->", total)
    except AttributeError as e:
        print("!!! total lookup failed:", e)
        total = None
    body = json.dumps(
        {
            "token": str(checkout.token),
            "total": str(total),
            "ok": total is not None,
        }
    )
    print("responding with", body)
    return HttpResponse(body, content_type="application/json")
# hunk ends here
