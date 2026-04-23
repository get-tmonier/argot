"""
Control: idiomatic FastAPI dependency injection with Depends() and generator deps.

This file demonstrates the canonical FastAPI DI pattern:
- Generator-based dependencies with yield for resource lifecycle management
- Nested Depends() chains resolved recursively by the framework
- Annotated[T, Depends(...)] shorthand for cleaner signatures
- use_cache=True (default) for singleton-per-request deps
- OAuth2PasswordBearer for token extraction
- Sub-dependencies that accept their own Depends() parameters

No manual setup/teardown, no singletons, no class instantiation at call sites.
Dependencies are plain callables (functions or classes) registered only via Depends().
"""

from __future__ import annotations

from collections.abc import Generator
from typing import Annotated, Any

from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer
from pydantic import BaseModel
from sqlalchemy.orm import Session, sessionmaker

router = APIRouter(tags=["di-demo"])

# Simulated sessionmaker
SessionLocal: sessionmaker[Session] = sessionmaker()  # type: ignore[call-arg]

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/auth/token")


# --- Generator dependency: DB session with guaranteed cleanup ---

def get_db() -> Generator[Session, None, None]:
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


DbDep = Annotated[Session, Depends(get_db)]


# --- Nested dependency: decode and validate JWT ---

class TokenPayload(BaseModel):
    sub: str
    scopes: list[str] = []


def decode_token(token: Annotated[str, Depends(oauth2_scheme)]) -> TokenPayload:
    # Placeholder: real code calls jwt.decode()
    if token == "bad":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="invalid token",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return TokenPayload(sub="user:1", scopes=["read", "write"])


TokenDep = Annotated[TokenPayload, Depends(decode_token)]


# --- Further nested: resolve user from DB using token payload ---

def get_current_user(token: TokenDep, db: DbDep) -> dict[str, Any]:
    user_id = int(token.sub.split(":")[1])
    user = db.get(object, user_id)  # type: ignore[call-overload]
    if user is None:
        raise HTTPException(status_code=404, detail="authenticated user not found")
    return dict(user.__dict__)  # type: ignore[union-attr]


CurrentUserDep = Annotated[dict[str, Any], Depends(get_current_user)]


# --- Pagination dependency ---

class PaginationParams(BaseModel):
    skip: int = 0
    limit: int = 20


def pagination(skip: int = 0, limit: int = 20) -> PaginationParams:
    return PaginationParams(skip=skip, limit=limit)


PaginationDep = Annotated[PaginationParams, Depends(pagination)]


# --- Endpoints using the dependency chain ---

@router.get("/items")
async def list_items(
    page: PaginationDep,
    db: DbDep,
    current_user: CurrentUserDep,
) -> dict[str, Any]:
    return {"skip": page.skip, "limit": page.limit, "user": current_user["id"], "items": []}


@router.get("/items/{item_id}")
async def get_item(
    item_id: int,
    db: DbDep,
    current_user: CurrentUserDep,
) -> dict[str, Any]:
    item = db.get(object, item_id)  # type: ignore[call-overload]
    if item is None:
        raise HTTPException(status_code=404, detail=f"item {item_id} not found")
    return dict(item.__dict__)  # type: ignore[union-attr]


@router.post("/items", status_code=201)
async def create_item(
    body: dict[str, Any],
    db: DbDep,
    current_user: CurrentUserDep,
) -> dict[str, Any]:
    if "read" not in current_user.get("scopes", ["read", "write"]):
        raise HTTPException(status_code=403, detail="insufficient scope")
    # Placeholder: real code inserts into DB
    return {"id": 1, **body, "owner_id": current_user["id"]}
