# Break: PyJWT signs preview tokens instead of django.core.signing
"""Break fixture — not for import."""
from __future__ import annotations

from django.core.signing import TimestampSigner

from wagtail.models import Page


# Decoy — idiomatic wagtail/django signed preview token, NOT inside the hunk range
def sign_preview(page: Page) -> str:
    return TimestampSigner().sign(str(page.pk))


# hunk starts here
import jwt

_SECRET = "wagtail-preview-secret"


def issue_preview_token(page: Page, user_id: int) -> str:
    payload = {"page_id": page.pk, "user_id": user_id, "kind": "preview"}
    return jwt.encode(payload, _SECRET, algorithm="HS256")


def verify_preview_token(token: str) -> dict:
    try:
        return jwt.decode(token, _SECRET, algorithms=["HS256"])
    except jwt.ExpiredSignatureError:
        return {}
    except jwt.InvalidTokenError:
        return {}
# hunk ends here
