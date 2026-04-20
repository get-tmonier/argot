"""
Control: idiomatic Pydantic validators on BaseModel, injected via FastAPI Depends.

This file demonstrates the canonical FastAPI validation pattern:
- @field_validator on individual fields for domain invariants
- @model_validator for cross-field validation
- ValidationError caught and re-raised as HTTPException at the route level
- BaseModel subclasses carry all validation logic; no manual isinstance checks
- Models injected as typed function parameters (automatic JSON parsing + validation)
- Depends() used for shared validation helpers
"""

from __future__ import annotations

from typing import Annotated, Any

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, EmailStr, Field, ValidationError, field_validator, model_validator

router = APIRouter(prefix="/accounts", tags=["accounts"])


class PasswordChange(BaseModel):
    current_password: str = Field(min_length=8)
    new_password: str = Field(min_length=8)
    confirm_password: str

    @field_validator("new_password")
    @classmethod
    def password_complexity(cls, v: str) -> str:
        if not any(c.isupper() for c in v):
            raise ValueError("password must contain at least one uppercase letter")
        if not any(c.isdigit() for c in v):
            raise ValueError("password must contain at least one digit")
        return v

    @model_validator(mode="after")
    def passwords_match(self) -> "PasswordChange":
        if self.new_password != self.confirm_password:
            raise ValueError("new_password and confirm_password must match")
        return self


class AccountCreate(BaseModel):
    username: str = Field(min_length=3, max_length=50, pattern=r"^[a-z0-9_]+$")
    email: EmailStr
    age: int = Field(ge=13, le=120)

    @field_validator("username")
    @classmethod
    def username_not_reserved(cls, v: str) -> str:
        reserved = {"admin", "root", "system", "api"}
        if v.lower() in reserved:
            raise ValueError(f"'{v}' is a reserved username")
        return v


def get_current_user() -> dict[str, Any]:
    return {"id": 1, "role": "user"}


@router.post("/register", status_code=201)
async def register(payload: AccountCreate) -> dict[str, Any]:
    return {"id": 99, "username": payload.username, "email": payload.email}


@router.post("/password-change")
async def change_password(
    payload: PasswordChange,
    current_user: Annotated[dict[str, Any], Depends(get_current_user)],
) -> dict[str, Any]:
    if payload.current_password == payload.new_password:
        raise HTTPException(status_code=422, detail="new password must differ from current")
    return {"status": "ok", "user_id": current_user["id"]}
