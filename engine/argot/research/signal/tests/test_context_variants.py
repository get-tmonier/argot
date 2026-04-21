from __future__ import annotations

from argot.research.signal.context_variants import build_context

SOURCE_CLASS_WITH_METHODS = """\
class Foo:
    def method_a(self):
        x = 1
        return x

    def method_b(self):
        y = 2
        return y

    def method_c(self):
        z = 3
        return z
"""

SOURCE_NESTED = """\
class Outer:
    def inner(self):
        a = 1
        b = 2
        return a + b
"""

SOURCE_MODULE_LEVEL = """\
x = 0
for i in range(10):
    x += i
result = x
"""

SOURCE_SINGLE_CHILD = """\
class Solo:
    def only_method(self):
        val = 42
        return val
"""


def _texts(dicts: list[dict[str, str]]) -> list[str]:
    return [d["text"] for d in dicts]


class TestParentOnly:
    def test_nested_function_hunk_inside_inner(self) -> None:
        # hunk = line 4 ("        b = 2") inside inner()
        result = build_context(SOURCE_NESTED, 4, 4, "parent_only")
        texts = _texts(result)
        # Should contain lines from inner() body, not Outer class lines
        assert any("a = 1" in t for t in texts)
        assert any("return a + b" in t for t in texts)
        # Hunk line should be absent (replaced with blank)
        assert not any("b = 2" in t for t in texts)

    def test_module_level_fallback_to_module(self) -> None:
        # hunk = line 3 ("    x += i") inside a for-loop at module level
        result = build_context(SOURCE_MODULE_LEVEL, 3, 3, "parent_only")
        texts = _texts(result)
        # Module is the enclosing scope; hunk line replaced with blank
        assert not any("x += i" in t for t in texts)
        # Other module lines present
        assert any("x = 0" in t for t in texts)


class TestSiblingsOnly:
    def test_class_methods_siblings(self) -> None:
        # hunk = line 7 ("        y = 2") inside method_b
        result = build_context(SOURCE_CLASS_WITH_METHODS, 7, 7, "siblings_only")
        texts = _texts(result)
        # method_a and method_c lines should appear
        assert any("method_a" in t for t in texts)
        assert any("method_c" in t for t in texts)
        # method_b body should NOT appear
        assert not any("method_b" in t for t in texts)

    def test_single_child_falls_back_to_parent_only(self) -> None:
        # method_b in SOURCE_CLASS_WITH_METHODS has only one child that encloses hunk;
        # SOURCE_SINGLE_CHILD has only one method, so siblings_only falls back to parent_only
        po = build_context(SOURCE_SINGLE_CHILD, 3, 3, "parent_only")
        so = build_context(SOURCE_SINGLE_CHILD, 3, 3, "siblings_only")
        assert _texts(po) == _texts(so)


class TestSyntaxErrorFallback:
    def test_all_non_baseline_modes_return_baseline(self) -> None:
        bad_source = "def broken(\n    x =\n"
        baseline = _texts(build_context(bad_source, 2, 2, "baseline"))
        for mode in ("parent_only", "file_only", "siblings_only"):
            result = _texts(build_context(bad_source, 2, 2, mode))
            assert result == baseline, f"mode={mode} did not match baseline"


class TestCharBudgetTruncation:
    def test_file_only_truncated_within_budget(self) -> None:
        long_line = "x = " + "a" * 200
        many_lines = "\n".join([long_line] * 30)
        result = build_context(many_lines, 15, 15, "file_only")
        total_chars = sum(len(d["text"]) for d in result)
        assert total_chars <= 2100


class TestBaseline:
    def test_returns_up_to_20_lines_before_hunk(self) -> None:
        source = "\n".join(f"line{i}" for i in range(1, 31))
        result = build_context(source, 25, 25, "baseline")
        assert len(result) == 20
        assert result[0]["text"] == "line5"
        assert result[-1]["text"] == "line24"

    def test_near_file_start_returns_fewer_lines(self) -> None:
        source = "\n".join(f"line{i}" for i in range(1, 11))
        result = build_context(source, 3, 3, "baseline")
        assert len(result) == 2
        assert result[0]["text"] == "line1"
        assert result[1]["text"] == "line2"
