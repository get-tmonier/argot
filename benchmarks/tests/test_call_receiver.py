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


def test_python_attribute_calls():
    from argot_bench.call_receiver import extract_callees

    source = "obj.method()\na.b.c()\nMath.floor(x)"
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "obj.method" in result
    assert "a.b.c" in result
    assert "Math.floor" in result


def test_python_decorator_as_call():
    from argot_bench.call_receiver import extract_callees

    source = "\n".join(
        [
            "@app.route('/')",
            "def index():",
            "    pass",
        ]
    )
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "app.route" in result


def test_python_bare_decorator_is_not_a_call():
    from argot_bench.call_receiver import extract_callees

    source = "\n".join(
        [
            "@staticmethod",
            "def m():",
            "    pass",
        ]
    )
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert result == []


def test_python_complex_chain_returns_none():
    from argot_bench.call_receiver import extract_callees

    # foo()()  -> innermost object is itself a call → None for outer call
    # arr[0]() -> innermost is a subscript → None
    source = "foo()()\narr[0]()"
    result = extract_callees(source, "python")
    # foo() is a call (extracts "foo"); the outer ()() also a call whose callee is foo() → None.
    non_none = [c for c in result if c is not None]
    assert "foo" in non_none
    assert None in result
