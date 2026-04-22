# engine/argot/research/signal/phase14/fixtures/stage2_only/walrus_operator.py
"""Walrus operator (:=) in while/if conditions — Python 3.8+ pattern, stdlib only.

FastAPI corpus uses traditional assignment; := is absent from the corpus.
Imports: re (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

import re


def extract_fields(text: str) -> dict[str, str]:
    result: dict[str, str] = {}
    pattern = re.compile(r"(\w+):\s*(.+?)(?:\n|$)")
    pos = 0
    while m := pattern.search(text, pos):
        result[m.group(1)] = m.group(2).strip()
        pos = m.end()
    return result


def first_matching(items: list[str], pred: object) -> str | None:
    return next(
        (item for item in items if (cleaned := item.strip()) and pred(cleaned)),  # type: ignore[operator]
        None,
    )


def drain_chunks(data: bytes, size: int = 64) -> list[bytes]:
    view = memoryview(data)
    chunks: list[bytes] = []
    offset = 0
    while chunk := bytes(view[offset : offset + size]):
        chunks.append(chunk)
        offset += size
    return chunks
