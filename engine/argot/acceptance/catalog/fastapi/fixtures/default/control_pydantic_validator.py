"""
Control: idiomatic FastAPI query-param validation via Pydantic BaseModel + Field + Query.

Grounded in docs_src/query_param_models/tutorial001.py from the FastAPI corpus.
Uses BaseModel, Field with constraints, Query(), and typed list params —
all high-frequency tokens in the corpus. No @field_validator, no @model_validator.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Query
from pydantic import BaseModel, Field

router = APIRouter(prefix="/items", tags=["items"])


class FilterParams(BaseModel):
    limit: int = Field(100, gt=0, le=100)
    offset: int = Field(0, ge=0)
    order_by: str = Field("created_at", pattern=r"^(created_at|updated_at)$")
    tags: list[str] = []


class ItemCreate(BaseModel):
    name: str = Field(min_length=1, max_length=100)
    description: str | None = None
    price: float = Field(gt=0)
    tags: list[str] = []


class ItemResponse(BaseModel):
    id: int
    name: str
    description: str | None = None
    price: float
    tags: list[str]


def get_db() -> Any:
    raise NotImplementedError


@router.get("", response_model=list[ItemResponse])
async def list_items(
    filter_query: FilterParams = Query(),
    db: Any = Depends(get_db),
) -> list[dict[str, Any]]:
    return []


@router.get("/{item_id}", response_model=ItemResponse)
async def get_item(item_id: int, db: Any = Depends(get_db)) -> dict[str, Any]:
    item = db.get(object, item_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return dict(item.__dict__)


@router.post("", response_model=ItemResponse, status_code=201)
async def create_item(
    payload: ItemCreate,
    db: Any = Depends(get_db),
) -> dict[str, Any]:
    return {"id": 99, **payload.model_dump()}
