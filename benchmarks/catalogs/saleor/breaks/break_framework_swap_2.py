# Break: FastAPI APIRouter endpoints with Pydantic models inside a Django views module
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def absolute_media_url(request, path: str) -> str:
    return request.build_absolute_uri(path)


# hunk starts here
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/media")


class MediaItem(BaseModel):
    id: int
    url: str
    alt: str | None = None


@router.get("/{media_id}", response_model=MediaItem)
async def get_media(media_id: int) -> MediaItem:
    from ..product.models import ProductMedia

    media = ProductMedia.objects.filter(pk=media_id).first()
    if media is None:
        raise HTTPException(status_code=404, detail="Media not found")
    return MediaItem(id=media.pk, url=media.image.url, alt=media.alt)


@router.post("/", response_model=MediaItem, status_code=201)
async def register_media(item: MediaItem) -> MediaItem:
    logger.info("registered media %s", item.id)
    return item
# hunk ends here
