from __future__ import annotations


def test_module_exports():
    from argot_bench.call_receiver import CallReceiverScorer, extract_callees

    assert callable(extract_callees)
    assert isinstance(CallReceiverScorer, type)


def test_extract_callees_unknown_language_raises_value_error():
    from argot_bench.call_receiver import extract_callees

    try:
        extract_callees("fetch()", "ruby")  # type: ignore[arg-type]
    except ValueError:
        return
    raise AssertionError("expected ValueError")


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


def test_typescript_plain_identifier_calls():
    from argot_bench.call_receiver import extract_callees

    source = "fetch(url);\nconsole.log('x');\nsetTimeout(fn, 1);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "fetch" in result
    assert "console.log" in result
    assert "setTimeout" in result


def test_typescript_dotted_member_expression():
    from argot_bench.call_receiver import extract_callees

    source = "Math.random(); axios.post('/x'); crypto.randomBytes(16);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Math.random" in result
    assert "axios.post" in result
    assert "crypto.randomBytes" in result


def test_typescript_new_expression():
    from argot_bench.call_receiver import extract_callees

    source = "const w = new Worker(src); const r = new express.Router();"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Worker" in result
    assert "express.Router" in result


def test_typescript_complex_chain_returns_none():
    from argot_bench.call_receiver import extract_callees

    source = "Router().route('/x').get(h);\narr[0]();"
    result = extract_callees(source, "typescript")
    non_none = [c for c in result if c is not None]
    assert "Router" in non_none
    assert None in result


def test_typescript_empty_and_parse_error():
    from argot_bench.call_receiver import extract_callees

    assert extract_callees("", "typescript") == []
    result = extract_callees("const x = ???", "typescript")
    assert all(r is None or isinstance(r, str) for r in result)


def test_python_smoke_realistic_source():
    from argot_bench.call_receiver import extract_callees

    source = '''\
import logging
from typing import Any

logger = logging.getLogger(__name__)


@dataclass
class Foo:
    def bar(self, x: int) -> int:
        logger.info("x=%s", x)
        result = self._compute(x)
        return result

    def _compute(self, x: int) -> int:
        return math.floor(x ** 0.5)
'''
    result = {c for c in extract_callees(source, "python") if c is not None}
    assert "logging.getLogger" in result
    assert "logger.info" in result
    assert "self._compute" in result
    assert "math.floor" in result


def test_typescript_smoke_realistic_source():
    from argot_bench.call_receiver import extract_callees

    source = '''\
import { Hono } from "hono";

const app = new Hono();

app.get("/", async (c) => {
  const data = await fetch("https://example.com").then((r) => r.json());
  c.header("Content-Type", "application/json");
  return c.json(data);
});

export default app;
'''
    result = {c for c in extract_callees(source, "typescript") if c is not None}
    assert "Hono" in result     # new Hono()
    assert "app.get" in result
    assert "fetch" in result
    assert "c.header" in result
    assert "c.json" in result


def test_scorer_fit_builds_attested_set(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("import logging\nlogger = logging.getLogger()\nlogger.info('x')\n")

    scorer = CallReceiverScorer([f], language="python", k=1)
    assert "logging.getLogger" in scorer.attested
    assert "logger.info" in scorer.attested


def test_scorer_fit_empty_file_list_raises():
    from argot_bench.call_receiver import CallReceiverScorer

    try:
        CallReceiverScorer([], language="python", k=1)
    except ValueError:
        return
    raise AssertionError("expected ValueError for empty model_a_files")


def test_scorer_k1_flags_single_unattested(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("import logging\nlogger = logging.getLogger()\nlogger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python", k=1)

    result = scorer.score_hunk("x = Math.random()")
    assert result.flagged is True
    assert "Math.random" in result.unattested


def test_scorer_k1_all_attested_does_not_flag(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\nlogger.debug('y')\n")
    scorer = CallReceiverScorer([f], language="python", k=1)

    result = scorer.score_hunk("logger.info('hello')\nlogger.debug('world')")
    assert result.flagged is False
    assert result.unattested == ()


def test_scorer_empty_hunk_does_not_flag(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python", k=1)

    for hunk in ("", "x = 1", "# only a comment"):
        result = scorer.score_hunk(hunk)
        assert result.flagged is False, f"unexpectedly flagged: {hunk!r}"


def test_scorer_k2_single_unattested_does_not_flag(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python", k=2)

    result = scorer.score_hunk("x = Math.random()")
    assert result.flagged is False
    assert "Math.random" in result.unattested


def test_scorer_k2_two_distinct_unattested_flags(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python", k=2)

    result = scorer.score_hunk(
        "x = Math.random()\ny = crypto.randomBytes(16)"
    )
    assert result.flagged is True
    assert "Math.random" in result.unattested
    assert "crypto.randomBytes" in result.unattested


def test_scorer_k2_same_callee_twice_does_not_flag(tmp_path):
    from argot_bench.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python", k=2)

    result = scorer.score_hunk("Math.random()\nMath.random()")
    assert result.flagged is False
    assert result.unattested == ("Math.random",)
