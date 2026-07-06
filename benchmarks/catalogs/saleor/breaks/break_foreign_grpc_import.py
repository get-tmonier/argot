# Break: grpc (imported in the hunk) dials a gRPC tax service, replacing the hardened HTTPClient webhook call
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def tax_service_metadata(app_id: int) -> dict[str, str]:
    return {"saleor-app": str(app_id)}


# hunk starts here
import grpc


def fetch_tax_rate(target: str, payload: bytes) -> bytes:
    channel = grpc.insecure_channel(target)
    call = channel.unary_unary("/tax.TaxService/GetRate")
    try:
        return call(payload, timeout=10)
    finally:
        channel.close()
# hunk ends here
