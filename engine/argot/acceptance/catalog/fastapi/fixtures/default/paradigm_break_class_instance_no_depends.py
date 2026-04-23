"""
Paradigm break: class instances as default parameter values instead of Depends().

Service objects (EmailService, CacheService) are instantiated at module level
and injected as plain default argument values: `service: EmailService = email_service`.
Without Depends(), FastAPI cannot manage lifecycle, override for tests, or
compose nested dependency graphs.

Corpus evidence:
- `Depends(...)` sites: 428 (dominant lifecycle pattern)
- Class instances as default values bypassing Depends(): 0 corpus sites (absent)

The structural axis: `= Depends(get_service)` replaced by `= module_level_instance`
at every endpoint signature. This looks superficially like DI but loses all
framework guarantees.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from fastapi import FastAPI, HTTPException

app = FastAPI()


@dataclass
class Settings:
    smtp_host: str = "localhost"
    smtp_port: int = 587
    redis_host: str = "localhost"
    redis_port: int = 6379


@dataclass
class EmailService:
    settings: Settings
    _sent: list[dict[str, Any]] = field(default_factory=list)

    def send(self, to: str, subject: str, body: str) -> dict[str, Any]:
        record = {"to": to, "subject": subject, "body": body}
        self._sent.append(record)
        return record

    def list_sent(self) -> list[dict[str, Any]]:
        return list(self._sent)


@dataclass
class CacheService:
    settings: Settings
    _store: dict[str, Any] = field(default_factory=dict)

    def get(self, key: str) -> Any | None:
        return self._store.get(key)

    def set(self, key: str, value: Any) -> None:
        self._store[key] = value

    def delete(self, key: str) -> bool:
        return self._store.pop(key, None) is not None


# BREAK: module-level instantiation — these are singletons, not DI-managed resources
_settings = Settings()
email_service = EmailService(settings=_settings)
cache_service = CacheService(settings=_settings)


# BREAK: plain default values instead of Depends() — FastAPI cannot substitute these
@app.post("/emails/send")
async def send_email(
    payload: dict[str, Any],
    service: EmailService = email_service,  # not Depends(get_email_service)
) -> dict[str, Any]:
    to = payload.get("to", "")
    subject = payload.get("subject", "")
    body = payload.get("body", "")
    if not to:
        raise HTTPException(status_code=422, detail="recipient 'to' is required")
    return service.send(to=str(to), subject=str(subject), body=str(body))


@app.get("/emails/sent")
async def list_sent(
    service: EmailService = email_service,  # not Depends(get_email_service)
) -> list[dict[str, Any]]:
    return service.list_sent()


@app.get("/cache/{key}")
async def cache_get(
    key: str,
    cache: CacheService = cache_service,  # not Depends(get_cache_service)
) -> dict[str, Any]:
    value = cache.get(key)
    if value is None:
        raise HTTPException(status_code=404, detail=f"key '{key}' not found")
    return {"key": key, "value": value}


@app.put("/cache/{key}")
async def cache_set(
    key: str,
    payload: dict[str, Any],
    cache: CacheService = cache_service,  # not Depends(get_cache_service)
) -> dict[str, Any]:
    value = payload.get("value")
    cache.set(key, value)
    return {"key": key, "value": value}


@app.delete("/cache/{key}", status_code=204)
async def cache_delete(
    key: str,
    cache: CacheService = cache_service,  # not Depends(get_cache_service)
) -> None:
    if not cache.delete(key):
        raise HTTPException(status_code=404, detail=f"key '{key}' not found")
