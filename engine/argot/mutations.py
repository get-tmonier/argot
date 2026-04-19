from __future__ import annotations

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


@_register("case_swap")
def _case_swap(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("debug_inject")
def _debug_inject(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("error_flip")
def _error_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("quote_flip")
def _quote_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError
