"""
Control: response_model= annotation on every endpoint, FastAPI handles serialization.

This file demonstrates the idiomatic FastAPI serialization pattern:
- response_model= on every decorator so FastAPI validates and serializes output
- Endpoints return plain dicts or Pydantic model instances — no manual json.dumps()
- response_model_exclude_unset=True for PATCH endpoints
- HTTPException for error responses; no manual Response(content=...) construction
- APIRouter with prefix and tags

No orjson, no explicit Response construction, no manual serialization.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/articles", tags=["articles"])

_articles: dict[int, dict[str, Any]] = {
    1: {"id": 1, "title": "First Post", "body": "Hello world", "published": False},
    2: {"id": 2, "title": "Second Post", "body": "More content", "published": True},
}
_next_id = 3


class ArticleCreate(BaseModel):
    title: str
    body: str
    published: bool = False


class ArticleUpdate(BaseModel):
    title: str | None = None
    body: str | None = None
    published: bool | None = None


class ArticleResponse(BaseModel):
    id: int
    title: str
    body: str
    published: bool


@router.get("", response_model=list[ArticleResponse])
async def list_articles(published: bool | None = None) -> list[dict[str, Any]]:
    items = list(_articles.values())
    if published is not None:
        items = [a for a in items if a["published"] == published]
    return items


@router.get("/{article_id}", response_model=ArticleResponse)
async def get_article(article_id: int) -> dict[str, Any]:
    article = _articles.get(article_id)
    if article is None:
        raise HTTPException(status_code=404, detail=f"article {article_id} not found")
    return article


@router.post("", response_model=ArticleResponse, status_code=201)
async def create_article(payload: ArticleCreate) -> dict[str, Any]:
    global _next_id
    article: dict[str, Any] = {"id": _next_id, **payload.model_dump()}
    _articles[_next_id] = article
    _next_id += 1
    return article


@router.patch("/{article_id}", response_model=ArticleResponse, response_model_exclude_unset=True)
async def update_article(article_id: int, payload: ArticleUpdate) -> dict[str, Any]:
    article = _articles.get(article_id)
    if article is None:
        raise HTTPException(status_code=404, detail=f"article {article_id} not found")
    updates = payload.model_dump(exclude_unset=True)
    article.update(updates)
    return article


@router.delete("/{article_id}", status_code=204)
async def delete_article(article_id: int) -> None:
    if article_id not in _articles:
        raise HTTPException(status_code=404, detail=f"article {article_id} not found")
    del _articles[article_id]
