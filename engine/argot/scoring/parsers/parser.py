from __future__ import annotations

from typing import Protocol, runtime_checkable


@runtime_checkable
class Parser(Protocol):
    def extract_imports(self, src: str) -> frozenset[str]: ...

    def prose_line_ranges(self, src: str) -> frozenset[int]: ...
