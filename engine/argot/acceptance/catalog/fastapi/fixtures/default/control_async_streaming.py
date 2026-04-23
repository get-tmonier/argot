"""
Control: async request handling with custom APIRoute and await.

Grounded in docs_src/custom_request_and_route/tutorial002_an_py39.py from the corpus.
Uses async def, await, Request, Response, HTTPException, RequestValidationError —
all present in the corpus. No StreamingResponse, no asyncio, no AsyncGenerator.
"""

from __future__ import annotations

from typing import Annotated, Any, Callable

from fastapi import APIRouter, Body, Depends, FastAPI, HTTPException, Request, Response
from fastapi.exceptions import RequestValidationError
from fastapi.routing import APIRoute


class ValidationLoggingRoute(APIRoute):
    def get_route_handler(self) -> Callable[[Request], Any]:
        original = super().get_route_handler()

        async def custom_handler(request: Request) -> Response:
            try:
                return await original(request)
            except RequestValidationError as exc:
                body = await request.body()
                detail = {"errors": exc.errors(), "body": body.decode()}
                raise HTTPException(status_code=422, detail=detail)

        return custom_handler


app = FastAPI()
app.router.route_class = ValidationLoggingRoute

router = APIRouter(prefix="/process", tags=["process"], route_class=ValidationLoggingRoute)


def get_current_user() -> dict[str, Any]:
    return {"id": 1, "role": "user"}


@router.post("/numbers")
async def sum_numbers(
    numbers: Annotated[list[int], Body()],
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    return {"sum": sum(numbers), "count": len(numbers), "user_id": current_user["id"]}


@router.post("/strings")
async def concat_strings(
    items: Annotated[list[str], Body()],
    separator: str = " ",
    current_user: dict[str, Any] = Depends(get_current_user),
) -> dict[str, Any]:
    if not items:
        raise HTTPException(status_code=422, detail="items must not be empty")
    return {"result": separator.join(items), "user_id": current_user["id"]}
