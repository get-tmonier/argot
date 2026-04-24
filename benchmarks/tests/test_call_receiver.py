from __future__ import annotations


def test_module_exports():
    from argot_bench.call_receiver import CallReceiverScorer, extract_callees

    assert callable(extract_callees)
    assert isinstance(CallReceiverScorer, type)


def test_extract_callees_stub_raises_not_implemented():
    from argot_bench.call_receiver import extract_callees

    try:
        extract_callees("fetch()", "python")
    except NotImplementedError:
        return
    raise AssertionError("expected NotImplementedError")
