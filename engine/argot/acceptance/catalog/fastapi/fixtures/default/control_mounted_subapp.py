"""
Control: idiomatic app.mount() for ASGI sub-app composition.

FastAPI supports mounting static file directories and WSGI/ASGI sub-applications
at a path prefix via `app.mount()`. This is valid FastAPI, used for static assets
and legacy sub-app integration. 6 app.mount() call sites confirmed in corpus.

Key markers (all corpus-confirmed):
- `app.mount("/static", StaticFiles(directory=...), name="static")` — 6 corpus sites
- `StaticFiles` from `fastapi.staticfiles` — standard static-file ASGI app
- `@app.get` decorators for the primary API — 833 corpus sites
- `HTTPException` for error responses — 78 corpus sites
- `response_model=` on decorators — 163 corpus sites

Source: adapted from docs_src/static_files/tutorial001_py310.py (lines 1-6) and
docs_src/bigger_applications/app_an_py310/main.py (lines 1-23); static mount
pattern extended with a realistic API surface alongside the mounted directory.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

app = FastAPI(title="Static + API", version="1.0.0")

# ---------------------------------------------------------------------------
# Static files mount — corpus pattern: app.mount() with StaticFiles
# ---------------------------------------------------------------------------

# Mount a static directory at /static so that e.g. /static/logo.png is served
# directly by the StaticFiles ASGI app without passing through FastAPI routing.
_static_dir = Path(__file__).parent / "static"
_static_dir.mkdir(exist_ok=True)

app.mount("/static", StaticFiles(directory=str(_static_dir)), name="static")

# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------


class ArticleCreate(BaseModel):
    title: str
    body: str


class ArticleResponse(BaseModel):
    id: int
    title: str
    body: str
    url: str


# ---------------------------------------------------------------------------
# In-memory store
# ---------------------------------------------------------------------------

_articles: dict[int, dict[str, Any]] = {
    1: {"id": 1, "title": "Hello World", "body": "First post.", "url": "/articles/1"},
}
_next_article_id = 2

# ---------------------------------------------------------------------------
# API endpoints
# ---------------------------------------------------------------------------


@app.get("/", response_class=HTMLResponse)
async def index() -> str:
    links = "".join(
        f'<li><a href="/articles/{a["id"]}">{a["title"]}</a></li>'
        for a in _articles.values()
    )
    return f"<html><body><ul>{links}</ul></body></html>"


@app.get("/articles", response_model=list[ArticleResponse])
async def list_articles() -> list[dict[str, Any]]:
    return list(_articles.values())


@app.get("/articles/{article_id}", response_model=ArticleResponse)
async def get_article(article_id: int) -> dict[str, Any]:
    article = _articles.get(article_id)
    if article is None:
        raise HTTPException(status_code=404, detail=f"article {article_id} not found")
    return article


@app.post("/articles", response_model=ArticleResponse, status_code=201)
async def create_article(payload: ArticleCreate) -> dict[str, Any]:
    global _next_article_id
    article: dict[str, Any] = {
        "id": _next_article_id,
        "title": payload.title,
        "body": payload.body,
        "url": f"/articles/{_next_article_id}",
    }
    _articles[_next_article_id] = article
    _next_article_id += 1
    return article
