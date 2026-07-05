# Break: tenacity retry decorator (imported in the hunk) replaces Celery self.retry for app installation
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def installation_status_key(app_id: int) -> str:
    return f"app-install:{app_id}"


# hunk starts here
from tenacity import retry, stop_after_attempt, wait_exponential


@retry(stop=stop_after_attempt(5), wait=wait_exponential(multiplier=1, max=30))
def fetch_app_manifest(client, manifest_url: str) -> dict:
    response = client.get(manifest_url, timeout=15)
    response.raise_for_status()
    return response.json()


def install_app(client, manifest_url: str) -> dict:
    manifest = fetch_app_manifest(client, manifest_url)
    logger.info("installed app from %s", manifest_url)
    return manifest
# hunk ends here
