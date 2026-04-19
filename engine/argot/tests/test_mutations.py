from __future__ import annotations

import pytest

from argot.mutations import MUTATIONS, apply_mutation


def _make_record(lang: str = "python", hunk_texts: list[str] | None = None) -> dict[str, object]:
    return {
        "_repo": "demo",
        "author_date_iso": "1700000000",
        "language": lang,
        "context_before": [{"text": "x"}],
        "hunk_tokens": [{"text": t} for t in (hunk_texts or ["def", "foo", "(", ")", ":"])],
    }


def test_dispatcher_lists_all_four_mutations() -> None:
    assert set(MUTATIONS) == {"case_swap", "debug_inject", "error_flip", "quote_flip"}


def test_dispatcher_rejects_unknown_name() -> None:
    with pytest.raises(KeyError):
        apply_mutation("not_a_mutation", _make_record(), seed=0)


def test_dispatcher_preserves_context_before() -> None:
    rec = _make_record()
    out = apply_mutation("quote_flip", rec, seed=0)
    assert out["context_before"] == rec["context_before"]


def test_dispatcher_returns_new_record() -> None:
    rec = _make_record()
    out = apply_mutation("quote_flip", rec, seed=0)
    assert out is not rec
    assert out["hunk_tokens"] is not rec["hunk_tokens"]


def test_quote_flip_swaps_double_to_single() -> None:
    rec = _make_record(hunk_texts=['"hello"', "x"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["'hello'", "x"]


def test_quote_flip_swaps_single_to_double() -> None:
    rec = _make_record(hunk_texts=["'hello'", "'world'"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ['"hello"', '"world"']


def test_quote_flip_leaves_non_strings_unchanged() -> None:
    rec = _make_record(hunk_texts=["def", "foo", "(", ")", ":"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["def", "foo", "(", ")", ":"]


def test_quote_flip_ignores_unbalanced_quotes() -> None:
    rec = _make_record(hunk_texts=['"hello', "hello'"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ['"hello', "hello'"]


def test_quote_flip_is_deterministic() -> None:
    rec = _make_record(hunk_texts=['"a"', "'b'"])
    assert apply_mutation("quote_flip", rec, seed=0) == apply_mutation("quote_flip", rec, seed=42)
