"""
Control: nested Depends() chain with a generator sub-dependency.

Demonstrates the "nested deps" pattern from FastAPI docs_src/dependencies/:
- A top-level dependency (get_query_or_cookie_params) itself depends on two
  sub-dependencies via Depends()
- One sub-dependency (get_db_session) is a generator (yield-based lifecycle)
- The other sub-dependency (get_current_user) chains from an auth extractor
- The endpoint receives the composed result; framework resolves the full graph

Source references:
- docs_src/dependencies/tutorial005.py — nested Depends() composition
- docs_src/dependencies/tutorial002.py — generator dep with yield + finally
- tests/test_tutorial/test_dependencies/ — test coverage for nested graph

Annotated[T, Depends(...)] aliases keep signatures readable (1,136 Annotated
usages in corpus — deeply in-vocabulary).
"""

from __future__ import annotations

from collections.abc import Generator
from typing import Annotated, Any

from fastapi import APIRouter, Depends, HTTPException, Query
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

router = APIRouter(prefix="/reports", tags=["reports"])

# ---------------------------------------------------------------------------
# Simulated backing store
# ---------------------------------------------------------------------------

_db: dict[int, dict[str, Any]] = {
    1: {"id": 1, "name": "Alpha", "owner_id": 42},
    2: {"id": 2, "name": "Beta", "owner_id": 42},
    3: {"id": 3, "name": "Gamma", "owner_id": 99},
}

security = HTTPBearer(auto_error=False)

# ---------------------------------------------------------------------------
# Leaf sub-dependencies
# ---------------------------------------------------------------------------


def get_db_session() -> Generator[dict[int, dict[str, Any]], None, None]:
    """Generator sub-dependency: simulates a DB session with guaranteed cleanup."""
    session = _db
    try:
        yield session
    finally:
        # In a real app: session.close() / rollback on error
        pass


DbSession = Annotated[dict[int, dict[str, Any]], Depends(get_db_session)]


def extract_credentials(
    credentials: Annotated[HTTPAuthorizationCredentials | None, Depends(security)],
) -> str:
    if credentials is None:
        raise HTTPException(status_code=401, detail="not authenticated")
    return credentials.credentials


CredentialsDep = Annotated[str, Depends(extract_credentials)]


def get_current_user(token: CredentialsDep) -> dict[str, Any]:
    """Sub-dependency that itself Depends() on the credential extractor."""
    # Placeholder: real code calls jwt.decode(token, ...)
    if token == "bad":
        raise HTTPException(status_code=401, detail="invalid token")
    return {"id": 42, "token": token, "scopes": ["read", "write"]}


CurrentUser = Annotated[dict[str, Any], Depends(get_current_user)]

# ---------------------------------------------------------------------------
# Top-level composed dependency (nested Depends — the key pattern)
# ---------------------------------------------------------------------------


class ReportParams:
    def __init__(
        self,
        db: DbSession,
        current_user: CurrentUser,
        skip: int = Query(default=0, ge=0),
        limit: int = Query(default=20, ge=1, le=100),
    ) -> None:
        self.db = db
        self.current_user = current_user
        self.skip = skip
        self.limit = limit


ReportParamsDep = Annotated[ReportParams, Depends(ReportParams)]

# ---------------------------------------------------------------------------
# Endpoints — each receives only the composed top-level dep
# ---------------------------------------------------------------------------


@router.get("")
async def list_reports(params: ReportParamsDep) -> dict[str, Any]:
    owner_id = params.current_user["id"]
    owned = [v for v in params.db.values() if v.get("owner_id") == owner_id]
    page = owned[params.skip : params.skip + params.limit]
    return {
        "total": len(owned),
        "skip": params.skip,
        "limit": params.limit,
        "items": page,
    }


@router.get("/{report_id}")
async def get_report(report_id: int, params: ReportParamsDep) -> dict[str, Any]:
    item = params.db.get(report_id)
    if item is None:
        raise HTTPException(status_code=404, detail=f"report {report_id} not found")
    if item.get("owner_id") != params.current_user["id"]:
        raise HTTPException(status_code=403, detail="access denied")
    return item


@router.post("", status_code=201)
async def create_report(
    payload: dict[str, Any],
    params: ReportParamsDep,
) -> dict[str, Any]:
    next_id = max(params.db.keys(), default=0) + 1
    item: dict[str, Any] = {
        "id": next_id,
        "owner_id": params.current_user["id"],
        **payload,
    }
    params.db[next_id] = item
    return item
