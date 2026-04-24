"""Break fixture — not for import."""
from __future__ import annotations
from typing import Annotated

from fastapi import FastAPI, Depends

app = FastAPI()


# Decoy idiomatic dependency — NOT inside the hunk range
class DatabasePool:
    def query(self, sql: str) -> list[dict[str, object]]:
        return []


def get_db() -> DatabasePool:
    return DatabasePool()


@app.get("/items-idiomatic")
async def list_items(db: Annotated[DatabasePool, Depends(get_db)]) -> list[dict[str, object]]:
    return db.query("SELECT * FROM items")


# hunk starts here
import injector
from injector import Injector, inject, Module, singleton


class UserService:
    @inject
    def __init__(self) -> None:
        self._data: list[dict[str, str]] = []

    def get_user(self, user_id: int) -> dict[str, str] | None:
        return next((u for u in self._data if u.get("id") == str(user_id)), None)


class ServiceModule(Module):
    def configure(self, binder: injector.Binder) -> None:
        binder.bind(UserService, to=UserService, scope=singleton)


_injector = Injector([ServiceModule()])


@app.get("/users/{user_id}")
async def get_user(user_id: int) -> dict[str, object]:
    svc = _injector.get(UserService)
    user = svc.get_user(user_id)
    if user is None:
        return {"error": "not found"}
    return dict(user)


@app.get("/users")
async def list_users() -> list[dict[str, str]]:
    svc = _injector.get(UserService)
    return svc._data
# hunk ends here
