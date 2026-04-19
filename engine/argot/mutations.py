from __future__ import annotations

import random
import re
from collections.abc import Callable
from typing import Any

MutationFn = Callable[[dict[str, Any], int], dict[str, Any]]

MUTATIONS: dict[str, MutationFn] = {}


def _register(name: str) -> Callable[[MutationFn], MutationFn]:
    def deco(fn: MutationFn) -> MutationFn:
        MUTATIONS[name] = fn
        return fn

    return deco


def apply_mutation(name: str, record: dict[str, Any], seed: int) -> dict[str, Any]:
    if name not in MUTATIONS:
        raise KeyError(f"unknown mutation: {name!r}")
    return MUTATIONS[name](record, seed)


def _clone_with_hunk(record: dict[str, Any], new_hunk: list[dict[str, Any]]) -> dict[str, Any]:
    return {**record, "hunk_tokens": new_hunk}


_STRING_RE = re.compile(r"^(['\"])(.*)\1$", re.DOTALL)
_IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_CAMEL_SPLIT_RE = re.compile(r"(?<!^)(?=[A-Z])")

_DEBUG_TEMPLATES: dict[str, list[str]] = {
    "python": ["print", "(", '"DEBUG"', ")"],
    "typescript": ["console", ".", "log", "(", '"DEBUG"', ")"],
    "javascript": ["console", ".", "log", "(", '"DEBUG"', ")"],
}


def _debug_template_for(language: str | None) -> list[str]:
    if language in _DEBUG_TEMPLATES:
        return _DEBUG_TEMPLATES[language]
    return _DEBUG_TEMPLATES["python"]


def _swap_case(ident: str) -> str:
    if "_" in ident and ident.upper() == ident:
        return ident  # SCREAMING_SNAKE — leave alone
    if "_" in ident:
        # snake_case → camelCase
        parts = ident.split("_")
        if not parts[0]:
            return ident  # leading underscore — leave alone
        return parts[0] + "".join(p.capitalize() for p in parts[1:] if p)
    # Check if it's a single-word all-uppercase (CONST) or all-lowercase (foo)
    if ident == ident.upper() or ident == ident.lower():
        return ident  # no change
    # camelCase or PascalCase → snake_case
    parts = _CAMEL_SPLIT_RE.split(ident)
    if len(parts) == 1:
        return ident  # safety check (shouldn't reach here)
    return "_".join(p.lower() for p in parts if p)


@_register("case_swap")
def _case_swap(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    new_hunk: list[dict[str, Any]] = []
    for tok in record["hunk_tokens"]:
        text = tok["text"]
        if _IDENT_RE.fullmatch(text) is None:
            new_hunk.append(tok)
            continue
        new_hunk.append({**tok, "text": _swap_case(text)})
    return _clone_with_hunk(record, new_hunk)


@_register("debug_inject")
def _debug_inject(record: dict[str, Any], seed: int) -> dict[str, Any]:
    hunk = record["hunk_tokens"]
    if not hunk:
        return _clone_with_hunk(record, list(hunk))
    rng = random.Random(seed)
    pos = rng.randint(0, len(hunk))
    injection = [{"text": t} for t in _debug_template_for(record.get("language"))]
    new_hunk = list(hunk[:pos]) + injection + list(hunk[pos:])
    return _clone_with_hunk(record, new_hunk)


@_register("error_flip")
def _error_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("quote_flip")
def _quote_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed  # deterministic transform
    new_hunk: list[dict[str, Any]] = []
    for tok in record["hunk_tokens"]:
        text = tok["text"]
        m = _STRING_RE.match(text)
        if m is None:
            new_hunk.append(tok)
            continue
        old_quote, body = m.group(1), m.group(2)
        new_quote = "'" if old_quote == '"' else '"'
        new_hunk.append({**tok, "text": f"{new_quote}{body}{new_quote}"})
    return _clone_with_hunk(record, new_hunk)
