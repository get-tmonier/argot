from __future__ import annotations

from typing import Any

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
    assert set(MUTATIONS) == {
        "case_swap",
        "debug_inject",
        "error_flip",
        "quote_flip",
        "semantic_logging",
        "semantic_error",
        "semantic_validation",
        "semantic_composition",
        "semantic_di",
    }


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


def test_case_swap_snake_to_camel() -> None:
    rec = _make_record(hunk_texts=["get_user_id", "foo_bar"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["getUserId", "fooBar"]


def test_case_swap_camel_to_snake() -> None:
    rec = _make_record(hunk_texts=["getUserId", "fooBar"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["get_user_id", "foo_bar"]


def test_case_swap_pascal_to_snake() -> None:
    rec = _make_record(hunk_texts=["HttpClient", "UserRecord"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["http_client", "user_record"]


def test_case_swap_preserves_single_lowercase_word() -> None:
    rec = _make_record(hunk_texts=["foo", "bar", "baz"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["foo", "bar", "baz"]


def test_case_swap_preserves_screaming_snake() -> None:
    rec = _make_record(hunk_texts=["MAX_SIZE", "DEBUG"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["MAX_SIZE", "DEBUG"]


def test_case_swap_skips_non_identifier_tokens() -> None:
    rec = _make_record(hunk_texts=["(", ")", ":", "'", '"hello"', "123"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["(", ")", ":", "'", '"hello"', "123"]


def test_debug_inject_python_adds_print_call() -> None:
    rec = _make_record(lang="python", hunk_texts=["def", "foo", "(", ")"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "print" in texts
    assert '"DEBUG"' in texts
    assert len(out["hunk_tokens"]) == 4 + 4  # original + 4-token print call


def test_debug_inject_typescript_adds_console_log() -> None:
    rec = _make_record(lang="typescript", hunk_texts=["const", "x", "=", "1"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "console" in texts
    assert "log" in texts
    assert "." in texts
    assert len(out["hunk_tokens"]) == 4 + 6


def test_debug_inject_javascript_uses_same_template_as_typescript() -> None:
    rec = _make_record(lang="javascript", hunk_texts=["a"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "console" in texts


def test_debug_inject_unknown_language_falls_back_to_python() -> None:
    rec = _make_record(lang="rust", hunk_texts=["let", "x"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "print" in texts


def test_debug_inject_is_deterministic_per_seed() -> None:
    rec = _make_record(lang="python", hunk_texts=["a", "b", "c", "d"])
    a = apply_mutation("debug_inject", rec, seed=7)
    b = apply_mutation("debug_inject", rec, seed=7)
    assert a == b


def test_debug_inject_differs_across_seeds() -> None:
    rec = _make_record(lang="python", hunk_texts=[str(i) for i in range(50)])
    a = apply_mutation("debug_inject", rec, seed=0)
    b = apply_mutation("debug_inject", rec, seed=1)
    assert a != b


def test_debug_inject_empty_hunk_passes_through() -> None:
    rec = {
        "_repo": "demo",
        "author_date_iso": "1700000000",
        "language": "python",
        "context_before": [{"text": "x"}],
        "hunk_tokens": [],
    }
    out = apply_mutation("debug_inject", rec, seed=0)
    assert out["hunk_tokens"] == []


def test_error_flip_python_except_and_raise() -> None:
    rec = _make_record(
        lang="python",
        hunk_texts=["try", ":", "pass", "except", "Exception", ":", "raise"],
    )
    out = apply_mutation("error_flip", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "except" not in texts
    assert "raise" not in texts
    assert texts.count("finally") == 1
    assert texts.count("return") == 1


def test_error_flip_typescript_catch_and_throw() -> None:
    rec = _make_record(
        lang="typescript",
        hunk_texts=["try", "{", "}", "catch", "(", "e", ")", "{", "throw", "e", "}"],
    )
    out = apply_mutation("error_flip", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "catch" not in texts
    assert "throw" not in texts
    assert "finally" in texts
    assert "return" in texts


def test_error_flip_leaves_unrelated_tokens_alone() -> None:
    rec = _make_record(lang="python", hunk_texts=["def", "foo", "(", ")", ":", "pass"])
    out = apply_mutation("error_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["def", "foo", "(", ")", ":", "pass"]


def test_error_flip_unknown_language_uses_python_mapping() -> None:
    rec = _make_record(lang="rust", hunk_texts=["raise", "except"])
    out = apply_mutation("error_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["return", "finally"]


def test_error_flip_is_deterministic() -> None:
    rec = _make_record(lang="python", hunk_texts=["raise", "x"])
    assert apply_mutation("error_flip", rec, seed=1) == apply_mutation("error_flip", rec, seed=99)


# ── Semantic mutators ──────────────────────────────────────────────────────


def _make_ts_record(hunk_tokens: list[str]) -> dict[str, Any]:
    return {
        "language": "typescript",
        "context_before": [{"text": "Effect"}, {"text": "."}, {"text": "gen"}],
        "context_after": [],
        "hunk_tokens": [{"text": t} for t in hunk_tokens],
    }


def _make_py_record(hunk_tokens: list[str]) -> dict[str, Any]:
    return {
        "language": "python",
        "context_before": [{"text": "async"}, {"text": "def"}, {"text": "fetch"}],
        "context_after": [],
        "hunk_tokens": [{"text": t} for t in hunk_tokens],
    }


def test_semantic_logging_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Effect", ".", "logInfo", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "console" in text
    assert result["hunk_tokens"] != rec["hunk_tokens"]


def test_semantic_logging_py_replaces_hunk() -> None:
    rec = _make_py_record(["logger", ".", "info", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "print" in text


def test_semantic_error_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Effect", ".", "fail", "(", "err", ")"])
    result = apply_mutation("semantic_error", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "throw" in text or "try" in text


def test_semantic_error_py_replaces_hunk() -> None:
    rec = _make_py_record(["raise", " ", "HTTPStatusError", "(", "msg", ")"])
    result = apply_mutation("semantic_error", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "except" in text


def test_semantic_validation_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["Schema", ".", "parse", "(", "raw", ")"])
    result = apply_mutation("semantic_validation", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "if" in text or "typeof" in text


def test_semantic_composition_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["pipe", "(", "a", ",", "f", ",", "g", ")"])
    result = apply_mutation("semantic_composition", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "async" in text or "await" in text or "const" in text


def test_semantic_di_ts_replaces_hunk() -> None:
    rec = _make_ts_record(["yield", "*", "ModelTrainer"])
    result = apply_mutation("semantic_di", rec, seed=0)
    text = " ".join(t["text"] for t in result["hunk_tokens"])
    assert "new" in text


def test_semantic_mutators_all_registered() -> None:
    for name in (
        "semantic_logging",
        "semantic_error",
        "semantic_validation",
        "semantic_composition",
        "semantic_di",
    ):
        assert name in MUTATIONS, f"{name!r} not registered"


def test_semantic_mutators_preserve_non_hunk_fields() -> None:
    rec = _make_ts_record(["Effect", ".", "logInfo", "(", '"msg"', ")"])
    result = apply_mutation("semantic_logging", rec, seed=0)
    assert result["language"] == rec["language"]
    assert result["context_before"] == rec["context_before"]
