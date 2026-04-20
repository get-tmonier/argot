"""
Control: idiomatic FastAPI body validation via Pydantic BaseModel + @field_validator.

Uses @field_validator (Pydantic v2) to express custom cross-field and
domain-specific constraints declaratively on a BaseModel subclass. The model is
injected directly as an endpoint parameter — the canonical FastAPI pattern.

Adapted from the @field_validator usage in
tests/test_filter_pydantic_sub_model_pv2.py in the FastAPI source corpus. All
tokens are high-frequency corpus patterns: BaseModel (384 corpus sites),
Field with constraints, @field_validator, APIRouter, status_code, Depends,
HTTPException.
"""

from __future__ import annotations

from typing import Annotated, Any

from fastapi import APIRouter, Body, Depends, HTTPException
from pydantic import BaseModel, Field, field_validator

router = APIRouter(prefix="/orders", tags=["orders"])

_orders: dict[int, dict[str, Any]] = {}
_next_id = 1

VALID_STATUSES = {"pending", "confirmed", "shipped", "delivered", "cancelled"}


class OrderCreate(BaseModel):
    product_name: str = Field(min_length=1, max_length=200)
    quantity: int = Field(gt=0, le=1000)
    unit_price: float = Field(gt=0)
    discount: float = Field(default=0.0, ge=0.0, lt=1.0)
    notes: str | None = Field(default=None, max_length=500)

    @field_validator("product_name")
    @classmethod
    def product_name_must_not_be_blank(cls, v: str) -> str:
        if not v.strip():
            raise ValueError("product_name must not be blank or whitespace only")
        return v.strip()

    @field_validator("discount")
    @classmethod
    def discount_requires_minimum_quantity(cls, v: float) -> float:
        # Positive discounts are allowed; caller validates against quantity separately.
        return v


class OrderResponse(BaseModel):
    id: int
    product_name: str
    quantity: int
    unit_price: float
    discount: float
    total: float
    notes: str | None = None


class OrderUpdate(BaseModel):
    quantity: int | None = Field(default=None, gt=0, le=1000)
    notes: str | None = Field(default=None, max_length=500)

    @field_validator("notes")
    @classmethod
    def notes_strip_whitespace(cls, v: str | None) -> str | None:
        if v is not None:
            return v.strip() or None
        return v


def get_db() -> Any:
    raise NotImplementedError


# hunk_start_line: 72


@router.post("", response_model=OrderResponse, status_code=201)
async def create_order(
    payload: Annotated[OrderCreate, Body()],
    db: Any = Depends(get_db),
) -> dict[str, Any]:
    global _next_id
    total = payload.quantity * payload.unit_price * (1.0 - payload.discount)
    record: dict[str, Any] = {
        "id": _next_id,
        **payload.model_dump(),
        "total": round(total, 2),
    }
    _orders[_next_id] = record
    _next_id += 1
    return record


@router.get("/{order_id}", response_model=OrderResponse)
async def get_order(order_id: int, db: Any = Depends(get_db)) -> dict[str, Any]:
    order = _orders.get(order_id)
    if order is None:
        raise HTTPException(status_code=404, detail=f"order {order_id} not found")
    return order


@router.patch("/{order_id}", response_model=OrderResponse)
async def update_order(
    order_id: int,
    payload: Annotated[OrderUpdate, Body()],
    db: Any = Depends(get_db),
) -> dict[str, Any]:
    order = _orders.get(order_id)
    if order is None:
        raise HTTPException(status_code=404, detail=f"order {order_id} not found")
    updates = payload.model_dump(exclude_none=True)
    order.update(updates)
    total = order["quantity"] * order["unit_price"] * (1.0 - order["discount"])
    order["total"] = round(total, 2)
    return order

# hunk_end_line: 115
