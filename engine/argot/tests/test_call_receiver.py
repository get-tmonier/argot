from __future__ import annotations


def test_module_exports() -> None:
    from argot.scoring.scorers.call_receiver import CallReceiverScorer, extract_callees

    assert callable(extract_callees)
    assert isinstance(CallReceiverScorer, type)


def test_extract_callees_unknown_language_raises() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    try:
        extract_callees("fetch()", "ruby")  # type: ignore[arg-type]
    except ValueError:
        return
    raise AssertionError("expected ValueError")


def test_python_plain_identifier_calls() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "fetch()\nprint('hi')\nlen(xs)"
    result = extract_callees(source, "python")
    assert set(c for c in result if c is not None) == {"fetch", "print", "len"}


def test_python_empty_source_returns_empty() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    assert extract_callees("", "python") == []


def test_python_no_calls_returns_empty() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    assert extract_callees("x = 1\ny = 2", "python") == []


def test_python_syntax_error_returns_empty() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    assert extract_callees("def ((((", "python") == []


def test_python_attribute_calls() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "obj.method()\na.b.c()\nMath.floor(x)"
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "obj.method" in result
    assert "a.b.c" in result
    assert "Math.floor" in result


def test_python_decorator_as_call() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "\n".join(["@app.route('/')", "def index():", "    pass"])
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "app.route" in result


def test_python_bare_decorator_is_not_a_call() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "\n".join(["@staticmethod", "def m():", "    pass"])
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert result == []


def test_python_subscript_root_still_none() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # arr[0]() → subscript root, not a call or identifier → None (unchanged)
    source = "arr[0]()"
    result = extract_callees(source, "python")
    assert result == [None]


def test_python_call_root_single_hop_is_canonical() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # foo()() → outer call's callee is foo() (a call node) → "<call>"
    # inner call foo() → "foo"
    source = "foo()()"
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "foo" in result
    assert "<call>" in result


def test_python_call_root_multi_hop_is_canonical() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # foo().bar() → outer callee is foo().bar (member of call) → "<call>.bar"
    # inner call foo() → "foo"
    source = "foo().bar()"
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "foo" in result
    assert "<call>.bar" in result


def test_python_call_root_does_not_affect_identifier_rooted_chains() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # Regression: plain identifier-rooted chains must still extract unchanged
    source = "obj.method()\na.b.c()\nMath.floor(x)"
    result = [c for c in extract_callees(source, "python") if c is not None]
    assert "obj.method" in result
    assert "a.b.c" in result
    assert "Math.floor" in result
    assert "<call>" not in result
    assert "<call>.method" not in result


def test_python_smoke_realistic_source() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = """\
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
"""
    result = {c for c in extract_callees(source, "python") if c is not None}
    assert "logging.getLogger" in result
    assert "logger.info" in result
    assert "self._compute" in result
    assert "math.floor" in result


def test_typescript_plain_identifier_calls() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "fetch(url);\nconsole.log('x');\nsetTimeout(fn, 1);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "fetch" in result
    assert "console.log" in result
    assert "setTimeout" in result


def test_typescript_dotted_member_expression() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "Math.random(); axios.post('/x'); crypto.randomBytes(16);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Math.random" in result
    assert "axios.post" in result
    assert "crypto.randomBytes" in result


def test_typescript_new_expression() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = "const w = new Worker(src); const r = new express.Router();"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Worker" in result
    assert "express.Router" in result


def test_typescript_subscript_root_still_none() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # arr[0]() → subscript root → None (unchanged)
    source = "arr[0]();"
    result = extract_callees(source, "typescript")
    assert None in result
    assert "<call>" not in [c for c in result if c is not None]


def test_typescript_call_root_single_hop_is_canonical() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # Router()() → outer callee is Router() (call_expression) → "<call>"
    # inner call Router() → "Router"
    source = "Router()();"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Router" in result
    assert "<call>" in result


def test_typescript_call_root_route_chain() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # Router().route('/x') → callee is Router().route (member of call) → "<call>.route"
    # inner Router() → "Router"
    source = "Router().route('/users/:id');"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Router" in result
    assert "<call>.route" in result


def test_typescript_call_root_multi_hop_chain() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # Full Express routing chain: Router().route(path).get(h).post(h).delete(h)
    # Each chained call's callee has a call_expression as its object root.
    source = "Router().route('/users/:id').get(getHandler).post(postHandler).delete(delHandler);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Router" in result  # inner Router()
    assert "<call>.route" in result  # Router().route(...)
    assert "<call>.get" in result  # <result>.get(...)
    assert "<call>.post" in result  # <result>.post(...)
    assert "<call>.delete" in result  # <result>.delete(...)


def test_typescript_call_root_does_not_affect_identifier_rooted_chains() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    # Regression: plain identifier-rooted chains must extract unchanged
    source = "Math.random(); axios.post('/x'); crypto.randomBytes(16);"
    result = [c for c in extract_callees(source, "typescript") if c is not None]
    assert "Math.random" in result
    assert "axios.post" in result
    assert "crypto.randomBytes" in result
    assert "<call>" not in result
    assert "<call>.random" not in result


def test_typescript_empty_and_parse_error() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    assert extract_callees("", "typescript") == []
    result = extract_callees("const x = ???", "typescript")
    assert all(r is None or isinstance(r, str) for r in result)


def test_typescript_smoke_realistic_source() -> None:
    from argot.scoring.scorers.call_receiver import extract_callees

    source = """\
import { Hono } from "hono";

const app = new Hono();

app.get("/", async (c) => {
  const data = await fetch("https://example.com").then((r) => r.json());
  c.header("Content-Type", "application/json");
  return c.json(data);
});

export default app;
"""
    result = {c for c in extract_callees(source, "typescript") if c is not None}
    assert "Hono" in result
    assert "app.get" in result
    assert "fetch" in result
    assert "c.header" in result
    assert "c.json" in result


def test_has_root_error_clean_source_is_false() -> None:
    from argot.scoring.scorers.call_receiver import _has_root_error

    assert _has_root_error("def foo():\n    return 1\n", "python") is False


def test_has_root_error_fragment_is_true() -> None:
    from argot.scoring.scorers.call_receiver import _has_root_error

    # A triple-quoted docstring body without its opening ''' → ERROR at root
    fragment = "    :param x: int — the count\n    :returns: float"
    assert _has_root_error(fragment, "python") is True


# ---------------------------------------------------------------------------
# CallReceiverScorer tests
# ---------------------------------------------------------------------------


def test_scorer_fit_builds_attested_set(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("import logging\nlogger = logging.getLogger()\nlogger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    assert "logging.getLogger" in scorer.attested
    assert "logger.info" in scorer.attested


def test_scorer_fit_empty_file_list_raises() -> None:
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    try:
        CallReceiverScorer([], language="python")
    except ValueError:
        return
    raise AssertionError("expected ValueError for empty model_a_files")


def test_scorer_default_alpha_cap(tmp_path) -> None:  # type: ignore[no-untyped-def]
    """alpha=1.0 and cap=5 are the shipping defaults."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    assert scorer.alpha == 1.0
    assert scorer.cap == 5


def test_count_unattested_zero_for_all_attested(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\nlogger.debug('y')\n")
    scorer = CallReceiverScorer([f], language="python")
    assert scorer.count_unattested("logger.info('hello')") == 0


def test_count_unattested_counts_distinct(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    n = scorer.count_unattested("Math.random()\ncrypto.randomBytes(16)")
    assert n == 2


def test_count_unattested_deduplicates(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    # Same callee twice → still only 1 distinct unattested
    assert scorer.count_unattested("Math.random()\nMath.random()") == 1


def test_count_unattested_empty_hunk(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    assert scorer.count_unattested("") == 0
    assert scorer.count_unattested("x = 1") == 0


def test_count_unattested_zero_for_root_error_fragment(tmp_path) -> None:  # type: ignore[no-untyped-def]
    """Root-ERROR fragments must return 0 (parse-fragment guard)."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("logger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    fragment = "    :param x: int — the count\n    :returns: float"
    assert scorer.count_unattested(fragment) == 0


def test_scorer_fit_skips_data_dominant_files(tmp_path) -> None:  # type: ignore[no-untyped-def]
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    code = tmp_path / "code.py"
    code.write_text("import logging\nlogger = logging.getLogger()\nlogger.info('x')\n")
    locale = tmp_path / "locale.py"
    locale.write_text(
        "CITIES = [\n" + ",\n".join([f"    'city_{i}'" for i in range(200)]) + ",\n]\n"
    )
    adapter = PythonAdapter()
    scorer = CallReceiverScorer([code, locale], language="python", adapter=adapter)
    assert "logging.getLogger" in scorer.attested
    assert scorer.n_skipped_data_dominant == 1
