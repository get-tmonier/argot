# Break: FastAPI APIRouter + pydantic BaseModel + Depends replaces wagtail's DRF-style API viewsets
"""Break fixture — not for import."""
from __future__ import annotations

from django.http import Http404


# Decoy — idiomatic wagtail-style API helper, NOT inside the hunk range
def page_or_404(pk: int):
    from wagtail.models import Page

    try:
        return Page.objects.live().get(pk=pk)
    except Page.DoesNotExist:
        raise Http404


# hunk starts here
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/api/pages")


class PageOut(BaseModel):
    id: int
    title: str
    slug: str
    live: bool


class PageQuery(BaseModel):
    search: str | None = None
    limit: int = 20


def get_query(search: str | None = None, limit: int = 20) -> PageQuery:
    return PageQuery(search=search, limit=limit)


@router.get("/", response_model=list[PageOut])
async def list_pages(query: PageQuery = Depends(get_query)):
    rows = await _fetch_pages(query.search, query.limit)
    return [PageOut(**row) for row in rows]


@router.get("/{page_id}", response_model=PageOut)
async def get_page(page_id: int):
    row = await _fetch_page(page_id)
    if row is None:
        raise HTTPException(status_code=404, detail="Page not found")
    return PageOut(**row)


async def _fetch_pages(search: str | None, limit: int) -> list[dict]:
    return []


async def _fetch_page(page_id: int) -> dict | None:
    return None
# hunk ends here
