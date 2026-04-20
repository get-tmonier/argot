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


_ERROR_MAPS: dict[str, dict[str, str]] = {
    "python": {"except": "finally", "raise": "return"},
    "typescript": {"catch": "finally", "throw": "return", "throws": "returns"},
    "javascript": {"catch": "finally", "throw": "return", "throws": "returns"},
}


def _error_map_for(language: str | None) -> dict[str, str]:
    if language in _ERROR_MAPS:
        return _ERROR_MAPS[language]
    return _ERROR_MAPS["python"]


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
    del seed
    mapping = _error_map_for(record.get("language"))
    new_hunk = [
        {**tok, "text": mapping[tok["text"]]} if tok["text"] in mapping else tok
        for tok in record["hunk_tokens"]
    ]
    return _clone_with_hunk(record, new_hunk)


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


# ── Semantic mutators ──────────────────────────────────────────────────────

_SEMANTIC_LOGGING: dict[str, list[str]] = {
    "typescript": [
        "console",
        ".",
        "log",
        "(",
        '"processing"',
        ",",
        "data",
        ")",
        ";",
        "console",
        ".",
        "error",
        "(",
        '"failed"',
        ",",
        "error",
        ".",
        "message",
        ")",
    ],
    "javascript": [
        "console",
        ".",
        "log",
        "(",
        '"processing"',
        ",",
        "data",
        ")",
        ";",
        "console",
        ".",
        "error",
        "(",
        '"failed"',
        ",",
        "error",
        ".",
        "message",
        ")",
    ],
    "python": [
        "print",
        "(",
        "f",
        '"processing {data}"',
        ")",
        "print",
        "(",
        "f",
        '"error: {str(e)}"',
        ")",
    ],
}

_SEMANTIC_ERROR: dict[str, list[str]] = {
    "typescript": [
        "try",
        "{",
        "const",
        "result",
        "=",
        "await",
        "fn",
        "(",
        ")",
        ";",
        "if",
        "(",
        "!",
        "result",
        ".",
        "ok",
        ")",
        "{",
        "throw",
        "new",
        "Error",
        "(",
        "`failed`",
        ")",
        ";",
        "}",
        "}",
        "catch",
        "(",
        "e",
        ")",
        "{",
        "console",
        ".",
        "error",
        "(",
        "e",
        ")",
        ";",
        "throw",
        "e",
        ";",
        "}",
    ],
    "javascript": [
        "try",
        "{",
        "const",
        "result",
        "=",
        "await",
        "fn",
        "(",
        ")",
        ";",
        "throw",
        "new",
        "Error",
        "(",
        "`failed`",
        ")",
        ";",
        "}",
        "catch",
        "(",
        "e",
        ")",
        "{",
        "throw",
        "e",
        ";",
        "}",
    ],
    "python": [
        "try",
        ":",
        "result",
        "=",
        "await",
        "client",
        ".",
        "get",
        "(",
        "url",
        ")",
        "except",
        "Exception",
        "as",
        "e",
        ":",
        "print",
        "(",
        "f",
        '"error: {e}"',
        ")",
        "return",
        "None",
    ],
}

_SEMANTIC_VALIDATION: dict[str, list[str]] = {
    "typescript": [
        "if",
        "(",
        "!",
        "input",
        ".",
        "name",
        "||",
        "typeof",
        "input",
        ".",
        "name",
        "!==",
        '"string"',
        ")",
        "{",
        "throw",
        "new",
        "Error",
        "(",
        '"name required"',
        ")",
        ";",
        "}",
        "if",
        "(",
        "input",
        ".",
        "age",
        "<",
        "0",
        ")",
        "{",
        "throw",
        "new",
        "Error",
        "(",
        '"invalid age"',
        ")",
        ";",
        "}",
    ],
    "javascript": [
        "if",
        "(",
        "!",
        "input",
        ".",
        "name",
        ")",
        "{",
        "throw",
        "new",
        "Error",
        "(",
        '"name required"',
        ")",
        ";",
        "}",
    ],
    "python": [
        "if",
        "not",
        "isinstance",
        "(",
        "data",
        ",",
        "dict",
        ")",
        ":",
        "raise",
        "ValueError",
        "(",
        '"data must be a dict"',
        ")",
        "if",
        '"name"',
        "not",
        "in",
        "data",
        ":",
        "raise",
        "ValueError",
        "(",
        '"name is required"',
        ")",
    ],
}

_SEMANTIC_COMPOSITION: dict[str, list[str]] = {
    "typescript": [
        "const",
        "step1",
        "=",
        "parseInput",
        "(",
        "raw",
        ")",
        ";",
        "const",
        "step2",
        "=",
        "await",
        "validateStep1",
        "(",
        "step1",
        ")",
        ";",
        "const",
        "step3",
        "=",
        "await",
        "transformStep2",
        "(",
        "step2",
        ")",
        ";",
        "return",
        "step3",
        ";",
    ],
    "javascript": [
        "const",
        "step1",
        "=",
        "parseInput",
        "(",
        "raw",
        ")",
        ";",
        "const",
        "result",
        "=",
        "await",
        "transform",
        "(",
        "step1",
        ")",
        ";",
        "return",
        "result",
        ";",
    ],
    "python": [
        "step1",
        "=",
        "parse_input",
        "(",
        "raw",
        ")",
        "step2",
        "=",
        "validate",
        "(",
        "step1",
        ")",
        "result",
        "=",
        "await",
        "transform",
        "(",
        "step2",
        ")",
        "return",
        "result",
    ],
}

_SEMANTIC_DI: dict[str, list[str]] = {
    "typescript": [
        "const",
        "db",
        "=",
        "new",
        "DatabaseConnection",
        "(",
        "{",
        "host",
        ":",
        '"localhost"',
        ",",
        "port",
        ":",
        "5432",
        "}",
        ")",
        ";",
        "const",
        "repo",
        "=",
        "new",
        "UserRepository",
        "(",
        "db",
        ")",
        ";",
        "const",
        "service",
        "=",
        "new",
        "UserService",
        "(",
        "repo",
        ")",
        ";",
        "return",
        "await",
        "service",
        ".",
        "find",
        "(",
        "id",
        ")",
        ";",
    ],
    "javascript": [
        "const",
        "db",
        "=",
        "new",
        "DatabaseConnection",
        "(",
        ")",
        ";",
        "const",
        "service",
        "=",
        "new",
        "UserService",
        "(",
        "db",
        ")",
        ";",
        "return",
        "service",
        ".",
        "find",
        "(",
        "id",
        ")",
        ";",
    ],
    "python": [
        "db",
        "=",
        "DatabaseConnection",
        "(",
        "host",
        "=",
        '"localhost"',
        ")",
        "repo",
        "=",
        "UserRepository",
        "(",
        "db",
        ")",
        "service",
        "=",
        "UserService",
        "(",
        "repo",
        ")",
        "return",
        "await",
        "service",
        ".",
        "find",
        "(",
        "user_id",
        ")",
    ],
}


def _semantic_snippet(
    templates: dict[str, list[str]], language: str | None
) -> list[dict[str, Any]]:
    lang = language if language in templates else "python"
    return [{"text": t} for t in templates[lang]]


@_register("semantic_logging")
def _semantic_logging(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_LOGGING, record.get("language")))


@_register("semantic_error")
def _semantic_error(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_ERROR, record.get("language")))


@_register("semantic_validation")
def _semantic_validation(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_VALIDATION, record.get("language")))


@_register("semantic_composition")
def _semantic_composition(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(
        record, _semantic_snippet(_SEMANTIC_COMPOSITION, record.get("language"))
    )


@_register("semantic_di")
def _semantic_di(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    return _clone_with_hunk(record, _semantic_snippet(_SEMANTIC_DI, record.get("language")))
