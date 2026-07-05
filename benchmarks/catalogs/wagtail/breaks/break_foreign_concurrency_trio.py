# Break: trio (aliased) async runtime batch-publishes pages — only .run reached, an attested leaf
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Page


# Decoy — idiomatic wagtail synchronous revision publish, NOT inside the hunk range
def publish_one(page_id: int) -> None:
    Page.objects.get(pk=page_id).save_revision().publish()


# hunk starts here
import trio as _trio


async def _publish_one(page_id: int) -> None:
    page = Page.objects.get(pk=page_id)
    revision = page.save_revision()
    revision.publish()


async def _publish_all(page_ids: list[int]) -> None:
    for page_id in page_ids:
        await _publish_one(page_id)


def publish_pages_async(page_ids: list[int]) -> None:
    _trio.run(_publish_all, page_ids)
# hunk ends here
