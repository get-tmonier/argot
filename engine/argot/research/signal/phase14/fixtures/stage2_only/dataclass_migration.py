# engine/argot/research/signal/phase14/fixtures/stage2_only/dataclass_migration.py
"""@dataclass(frozen=True, slots=True) with 6 fields — host repo uses plain __init__ classes.

FastAPI corpus uses Pydantic BaseModel for structured data; frozen+slots dataclasses are absent.
Imports: dataclasses, datetime (both stdlib, in FastAPI corpus).
"""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime


@dataclass(frozen=True, slots=True)
class RequestRecord:
    method: str
    path: str
    status_code: int
    latency_ms: float
    body_size: int = 0
    timestamp: datetime = field(default_factory=datetime.utcnow)

    def is_slow(self, threshold_ms: float = 200.0) -> bool:
        return self.latency_ms > threshold_ms

    def to_log_line(self) -> str:
        return (
            f"{self.timestamp.isoformat()} {self.method} {self.path}"
            f" {self.status_code} {self.latency_ms:.1f}ms"
        )

    def summary(self) -> dict[str, object]:
        return {
            "method": self.method,
            "path": self.path,
            "status": self.status_code,
            "slow": self.is_slow(),
        }
