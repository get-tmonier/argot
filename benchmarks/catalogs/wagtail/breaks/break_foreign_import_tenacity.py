# Break: tenacity @retry wraps embed fetching instead of wagtail's finder chain
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.embeds.embeds import get_embed
from wagtail.embeds.exceptions import EmbedNotFoundException


# Decoy — idiomatic wagtail finder-based embed lookup, NOT inside the hunk range
def embed_html_or_blank(url: str) -> str:
    try:
        return get_embed(url).html
    except EmbedNotFoundException:
        return ""


# hunk starts here
import tenacity
from tenacity import retry, stop_after_attempt, wait_exponential


@retry(
    stop=stop_after_attempt(5),
    wait=wait_exponential(multiplier=1, max=30),
    retry=tenacity.retry_if_exception_type(EmbedNotFoundException),
    reraise=True,
)
def fetch_embed_with_retry(url: str, max_width: int | None = None):
    return get_embed(url, max_width=max_width)


def warm_embed_cache(urls: list[str]) -> dict[str, bool]:
    results: dict[str, bool] = {}
    for url in urls:
        try:
            fetch_embed_with_retry(url)
            results[url] = True
        except tenacity.RetryError:
            results[url] = False
    return results
# hunk ends here
