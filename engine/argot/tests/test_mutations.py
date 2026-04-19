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
