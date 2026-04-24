from __future__ import annotations


def test_module_exports():
    from argot_bench.call_receiver import CallReceiverScorer, extract_callees

    assert callable(extract_callees)
    assert isinstance(CallReceiverScorer, type)


def test_extract_callees_typescript_raises_not_implemented():
    from argot_bench.call_receiver import extract_callees

    try:
        extract_callees("fetch()", "typescript")
    except NotImplementedError:
        return
    raise AssertionError("expected NotImplementedError")


def test_python_plain_identifier_calls():
    from argot_bench.call_receiver import extract_callees

    source = "fetch()\nprint('hi')\nlen(xs)"
    result = extract_callees(source, "python")
    assert set(c for c in result if c is not None) == {"fetch", "print", "len"}


def test_python_empty_source_returns_empty_list():
    from argot_bench.call_receiver import extract_callees

    assert extract_callees("", "python") == []


def test_python_no_calls_returns_empty_list():
    from argot_bench.call_receiver import extract_callees

    assert extract_callees("x = 1\ny = 2", "python") == []


def test_python_syntax_error_returns_empty_list():
    from argot_bench.call_receiver import extract_callees

    assert extract_callees("def ((((", "python") == []
