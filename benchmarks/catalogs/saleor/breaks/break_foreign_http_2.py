# Break: bare requests.Session with mounted HTTPAdapter/Retry replaces the hardened HTTPClient manager
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def build_manifest_headers(schema_version: str) -> dict[str, str]:
    return {"Saleor-Schema-Version": schema_version}


# hunk starts here
import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry


def fetch_manifest_with_session(manifest_url: str) -> dict:
    session = requests.Session()
    retry = Retry(total=5, backoff_factor=0.5, status_forcelist=[502, 503, 504])
    adapter = HTTPAdapter(max_retries=retry, pool_maxsize=10)
    session.mount("https://", adapter)
    session.mount("http://", adapter)
    try:
        response = session.get(manifest_url, timeout=15, verify=True)
        response.raise_for_status()
        return response.json()
    finally:
        session.close()


def probe_app_endpoint(url: str) -> bool:
    resp = requests.head(url, timeout=5, allow_redirects=False)
    return resp.status_code < 400
# hunk ends here
