# engine/argot/research/signal/phase14/adapters/registry.py
"""Registry mapping file extensions to LanguageAdapter singleton instances."""

from __future__ import annotations

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.python_adapter import PythonAdapter

# Lazy-initialised singleton adapters (avoids loading tree-sitter grammars at import time)
_adapters: dict[str, LanguageAdapter] = {}


def _python() -> LanguageAdapter:
    if ".py" not in _adapters:
        _adapters[".py"] = PythonAdapter()
    return _adapters[".py"]


def _typescript() -> LanguageAdapter:
    if ".ts" not in _adapters:
        from argot.scoring.adapters.typescript import TypeScriptAdapter

        _adapters[".ts"] = TypeScriptAdapter()
        _adapters[".tsx"] = _adapters[".ts"]
    return _adapters[".ts"]


_EXTENSION_MAP: dict[str, str] = {
    ".py": "python",
    ".ts": "typescript",
    ".tsx": "typescript",
}

_FACTORIES = {
    "python": _python,
    "typescript": _typescript,
}


def get_adapter(ext: str) -> LanguageAdapter:
    """Return the adapter for file extension *ext* (e.g. ``".py"``).

    Falls back to PythonAdapter for unknown extensions.
    """
    lang = _EXTENSION_MAP.get(ext, "python")
    return _FACTORIES[lang]()


def adapter_for_files(paths: list[str]) -> LanguageAdapter:
    """Resolve the dominant adapter from a list of file path strings.

    Counts extensions, picks the most common one that has a registered adapter,
    falls back to PythonAdapter.
    """
    from collections import Counter

    counts: Counter[str] = Counter()
    for p in paths:
        ext = p[p.rfind(".") :] if "." in p else ""
        if ext in _EXTENSION_MAP:
            counts[ext] += 1

    if not counts:
        return get_adapter(".py")
    dominant_ext = counts.most_common(1)[0][0]
    return get_adapter(dominant_ext)
