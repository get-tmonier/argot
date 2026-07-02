# Break: urllib.request.urlopen with hand-rolled headers replaces the hardened HTTPClient wrapper
"""Break fixture — not for import."""
from __future__ import annotations

import json
import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def serialize_delivery_payload(payload: dict) -> str:
    return json.dumps(payload, sort_keys=True)


# hunk starts here
import urllib.error
import urllib.request


def post_webhook_payload(target_url: str, payload: dict, secret: str) -> dict:
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        target_url,
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {secret}",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = resp.read().decode("utf-8")
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        return {"status": e.code, "reason": e.reason}
    except urllib.error.URLError as e:
        return {"status": 0, "reason": str(e.reason)}
# hunk ends here
