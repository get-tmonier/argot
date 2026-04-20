"""
Control: Annotated[T, Depends(...)] shorthand for all dependency injection.

This file demonstrates the modern FastAPI DI pattern using Annotated:
- All dependencies declared as Annotated[Type, Depends(factory)] type aliases
- Reusable TypeAlias definitions at module level for common deps
- yield-based dependencies for resource lifecycle management
- No positional Depends() in function signatures — all via Annotated
- HTTPException raised through the dependency chain

Annotated appears 646 times in the FastAPI corpus — this is deeply in-distribution.
"""

from __future__ import annotations

from typing import Annotated, Any, Generator

from fastapi import APIRouter, Depends, HTTPException
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

router = APIRouter(prefix="/resources", tags=["resources"])

_db: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alpha"},
    2: {"id": 2, "name": "Beta"},
}
_next_id = 3

security = HTTPBearer(auto_error=False)


def get_db_session() -> Generator[dict[int, dict[str, Any]], None, None]:
    # Simulates a DB session with setup/teardown
    session = _db
    try:
        yield session
    finally:
        pass


def get_current_user(
    credentials: Annotated[HTTPAuthorizationCredentials | None, Depends(security)],
) -> dict[str, Any]:
    if credentials is None:
        raise HTTPException(status_code=401, detail="not authenticated")
    return {"id": 1, "token": credentials.credentials}


DbSession = Annotated[dict[int, dict[str, Any]], Depends(get_db_session)]
CurrentUser = Annotated[dict[str, Any], Depends(get_current_user)]


@router.get("")
async def list_resources(
    db: DbSession,
    current_user: CurrentUser,
) -> list[dict[str, Any]]:
    return list(db.values())


@router.get("/{resource_id}")
async def get_resource(
    resource_id: int,
    db: DbSession,
    current_user: CurrentUser,
) -> dict[str, Any]:
    item = db.get(resource_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"resource {resource_id} not found")
    return item


@router.post("", status_code=201)
async def create_resource(
    payload: dict[str, Any],
    db: DbSession,
    current_user: CurrentUser,
) -> dict[str, Any]:
    global _next_id
    item: dict[str, Any] = {"id": _next_id, **payload}
    db[_next_id] = item
    _next_id += 1
    return item


@router.delete("/{resource_id}", status_code=204)
async def delete_resource(
    resource_id: int,
    db: DbSession,
    current_user: CurrentUser,
) -> None:
    if resource_id not in db:
        raise HTTPException(status_code=404, detail=f"resource {resource_id} not found")
    del db[resource_id]
