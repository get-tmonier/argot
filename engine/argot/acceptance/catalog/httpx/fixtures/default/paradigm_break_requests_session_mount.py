from __future__ import annotations

from typing import Any

import requests
import requests.auth
from requests.adapters import HTTPAdapter
from requests.packages.urllib3.util.retry import Retry
from requests.packages.urllib3.util.ssl_ import create_urllib3_context

BASE_URL = "https://api.example.com"
INTERNAL_URL = "https://internal.corp.example.com"


def build_retry_policy(total: int = 3, backoff_factor: float = 0.3) -> Retry:
    return Retry(
        total=total,
        backoff_factor=backoff_factor,
        status_forcelist=[429, 500, 502, 503, 504],
        allowed_methods=["GET", "POST", "PUT"],
        raise_on_status=False,
    )


class BearerTokenAuth(requests.auth.AuthBase):
    def __init__(self, token: str) -> None:
        self._token = token

    def __call__(self, r: requests.PreparedRequest) -> requests.PreparedRequest:
        r.headers["Authorization"] = f"Bearer {self._token}"
        return r


class TLSClientCertAdapter(HTTPAdapter):
    def __init__(self, cert_file: str, key_file: str, **kwargs: Any) -> None:
        self.cert_file = cert_file
        self.key_file = key_file
        super().__init__(**kwargs)

    def init_poolmanager(self, *args: Any, **kwargs: Any) -> None:
        ctx = create_urllib3_context()
        ctx.load_cert_chain(certfile=self.cert_file, keyfile=self.key_file)
        kwargs["ssl_context"] = ctx
        super().init_poolmanager(*args, **kwargs)

    def send(self, request: requests.PreparedRequest, **kwargs: Any) -> requests.Response:  # type: ignore[override]
        kwargs.setdefault("timeout", 30.0)
        return super().send(request, **kwargs)


session = requests.Session()
session.auth = BearerTokenAuth("my-api-key")
session.headers.update(
    {
        "Accept": "application/json",
        "User-Agent": "example-service/1.0",
        "X-Api-Version": "2024-01",
    }
)

default_adapter = HTTPAdapter(max_retries=build_retry_policy())
tls_adapter = TLSClientCertAdapter(
    cert_file="/etc/certs/client.crt",
    key_file="/etc/certs/client.key",
    max_retries=build_retry_policy(total=1),
)

session.mount("https://internal.", tls_adapter)
session.mount("https://", default_adapter)
session.mount("http://", default_adapter)

resp = session.get(f"{BASE_URL}/v1/resources")
resp.raise_for_status()
data: list[dict[str, Any]] = resp.json()

resp2 = session.post(f"{BASE_URL}/v1/resources", json={"name": "new-item"})
resp2.raise_for_status()
created = resp2.json()

resp3 = session.get(f"{INTERNAL_URL}/v1/internal-data")
resp3.raise_for_status()
internal_data = resp3.json()

session.close()
