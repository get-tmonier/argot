# Break: requests.post webhook call with retry loop inside a Page publish flow — blocking HTTP in model code
"""Break fixture — not for import."""
from __future__ import annotations

import logging

from wagtail.models import Page

logger = logging.getLogger("wagtail")


# Decoy — idiomatic wagtail-style model helper, NOT inside the hunk range
def latest_revision_title(page: Page) -> str:
    revision = page.get_latest_revision_as_object()
    return revision.title if revision else page.title


# hunk starts here
import requests


def notify_publish_webhook(page: Page, endpoint: str) -> bool:
    payload = {
        "id": page.pk,
        "title": page.title,
        "url": page.get_full_url(),
        "event": "page_published",
    }
    for attempt in range(3):
        try:
            response = requests.post(endpoint, json=payload, timeout=10)
            if response.status_code == 200:
                return True
            logger.warning(
                "Webhook returned %s on attempt %d", response.status_code, attempt + 1
            )
        except requests.RequestException:
            continue
    return False


def fetch_remote_metadata(page: Page, api_base: str) -> dict:
    response = requests.get(
        f"{api_base}/pages/{page.slug}/metadata",
        headers={"Accept": "application/json"},
        timeout=5,
    )
    response.raise_for_status()
    return response.json()
# hunk ends here
