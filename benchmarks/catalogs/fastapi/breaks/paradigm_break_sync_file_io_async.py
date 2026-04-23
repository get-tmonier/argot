"""
Paradigm break: synchronous file I/O inside async def endpoints.

The corpus (FastAPI docs_src/ and tests/) shows zero instances of open() or
Path.read_text() inside AsyncFunctionDef bodies at endpoint scope. Async
endpoints that need file I/O either use `aiofiles` or offload to a thread via
anyio.to_thread.run_sync / run_in_threadpool. This fixture simulates a
document-storage API where every endpoint reads or writes JSON files
synchronously, blocking the event loop for the duration of each disk operation.

OOV axis: blocking open() / Path.read_text() / json.loads() tight-loop inside
async def endpoint bodies. Corpus evidence: 0 instances of open() /
.read_text() inside AsyncFunctionDef bodies at endpoint scope in docs_src/ or
tests/. Idiomatic pattern is aiofiles or anyio.to_thread.run_sync().
"""

from __future__ import annotations

import json
from pathlib import Path

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI()

DOCS_DIR = Path("/var/data/documents")


class DocumentCreate(BaseModel):
    title: str
    content: str
    tags: list[str] = []


class DocumentUpdate(BaseModel):
    content: str
    tags: list[str] = []


def _doc_path(doc_id: str) -> Path:
    return DOCS_DIR / f"{doc_id}.json"


# hunk_start_line: 40
@app.get("/documents/{doc_id}")
async def get_document(doc_id: str) -> dict[str, object]:
    path = _doc_path(doc_id)
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"document {doc_id!r} not found")
    # Blocking read inside async def — starves the event loop
    raw = path.read_text(encoding="utf-8")
    return json.loads(raw)  # type: ignore[no-any-return]


@app.post("/documents/{doc_id}", status_code=201)
async def create_document(doc_id: str, payload: DocumentCreate) -> dict[str, object]:
    path = _doc_path(doc_id)
    if path.exists():
        raise HTTPException(status_code=409, detail=f"document {doc_id!r} already exists")
    doc = {"id": doc_id, "title": payload.title, "content": payload.content, "tags": payload.tags}
    # Blocking write inside async def
    path.write_text(json.dumps(doc, indent=2), encoding="utf-8")
    return doc


@app.put("/documents/{doc_id}")
async def update_document(doc_id: str, payload: DocumentUpdate) -> dict[str, object]:
    path = _doc_path(doc_id)
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"document {doc_id!r} not found")
    # Blocking read-modify-write cycle inside async def
    existing: dict[str, object] = json.loads(path.read_text(encoding="utf-8"))
    existing["content"] = payload.content
    existing["tags"] = payload.tags
    path.write_text(json.dumps(existing, indent=2), encoding="utf-8")
    return existing


@app.get("/documents")
async def list_documents(tag: str | None = None) -> dict[str, object]:
    # Blocking directory scan + tight json.loads() loop inside async def
    results: list[dict[str, object]] = []
    for file in DOCS_DIR.glob("*.json"):
        doc: dict[str, object] = json.loads(open(file).read())  # noqa: WPS515
        if tag is None or tag in doc.get("tags", []):
            results.append(doc)
    return {"documents": results, "total": len(results)}


@app.delete("/documents/{doc_id}", status_code=204)
async def delete_document(doc_id: str) -> None:
    path = _doc_path(doc_id)
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"document {doc_id!r} not found")
    # Blocking existence check and unlink inside async def
    raw = path.read_text(encoding="utf-8")
    _ = json.loads(raw)  # validate JSON before deleting
    path.unlink()
# hunk_end_line: 86
