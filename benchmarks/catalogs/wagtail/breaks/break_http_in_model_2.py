# Break: httpx.Client fetches oEmbed JSON directly instead of going through the configured finder chain
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.embeds.models import Embed


# Decoy — idiomatic wagtail-style embed lookup, NOT inside the hunk range
def cached_embed_for(url: str) -> Embed | None:
    try:
        return Embed.objects.get(url=url)
    except Embed.DoesNotExist:
        return None


# hunk starts here
import httpx

_OEMBED_ENDPOINT = "https://www.youtube.com/oembed"


def fetch_oembed_direct(url: str, max_width: int | None = None) -> dict:
    params = {"url": url, "format": "json"}
    if max_width is not None:
        params["maxwidth"] = str(max_width)
    with httpx.Client(timeout=8.0, follow_redirects=True) as client:
        response = client.get(_OEMBED_ENDPOINT, params=params)
        response.raise_for_status()
        data = response.json()
    return {
        "title": data.get("title", ""),
        "provider_name": data.get("provider_name", ""),
        "html": data.get("html", ""),
        "width": data.get("width"),
        "height": data.get("height"),
        "thumbnail_url": data.get("thumbnail_url", ""),
    }


def probe_embed_alive(url: str) -> bool:
    with httpx.Client(timeout=4.0) as client:
        response = client.head(url)
        return response.status_code < 400
# hunk ends here
