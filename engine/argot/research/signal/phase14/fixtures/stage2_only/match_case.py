# engine/argot/research/signal/phase14/fixtures/stage2_only/match_case.py
"""Structural pattern matching (match/case) — Python 3.10+, stdlib only.

FastAPI corpus predates widespread match/case adoption; this pattern is absent.
Imports: typing (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

from typing import Any


def classify_response(response: dict[str, Any]) -> str:
    match response:
        case {"status": 200, "data": [*items]} if items:
            return f"ok:{len(items)}"
        case {"status": 200}:
            return "ok:empty"
        case {"status": 404}:
            return "not_found"
        case {"status": int(code)} if 500 <= code < 600:
            return f"server_error:{code}"
        case _:
            return "unknown"


def route_command(cmd: str, args: list[str]) -> tuple[str, list[str]]:
    match (cmd, args):
        case ("get", [resource, *rest]):
            return "fetch", [resource, *rest]
        case ("post", [resource, payload, *_]):
            return "create", [resource, payload]
        case ("delete", [resource]):
            return "remove", [resource]
        case _:
            return "noop", []
