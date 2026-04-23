from __future__ import annotations


def test_typicality_features_has_five_fields():
    from argot_bench.typicality import TypicalityFeatures

    tf = TypicalityFeatures(
        literal_leaf_ratio=0.5,
        control_node_density=2.0,
        ast_type_entropy=3.1,
        unique_token_ratio=0.4,
        named_leaf_count=42,
    )
    assert tf.literal_leaf_ratio == 0.5
    assert tf.control_node_density == 2.0
    assert tf.ast_type_entropy == 3.1
    assert tf.unique_token_ratio == 0.4
    assert tf.named_leaf_count == 42


def test_python_data_heavy_has_high_literal_ratio():
    """A Python dict of string literals should have literal_leaf_ratio > 0.85."""
    from argot_bench.typicality import compute_features

    source = """
EMOJI_CODES = {
    "smile": "U+1F600",
    "grin":  "U+1F601",
    "joy":   "U+1F602",
    "rofl":  "U+1F923",
    "heart": "U+2764",
    "star":  "U+2B50",
}
""".strip()
    f = compute_features(source, "python")
    assert f.literal_leaf_ratio > 0.85, f
    assert f.control_node_density == 0.0, f


def test_python_normal_code_has_moderate_literal_ratio():
    """A normal function with branches has moderate literal ratio and nonzero control density."""
    from argot_bench.typicality import compute_features

    source = """
def parse(request, registry):
    handlers = registry.lookup(request.path)
    if not handlers:
        raise KeyError(request.path)
    for h in handlers:
        if h.matches(request):
            return h.handle(request)
    return None
""".strip()
    f = compute_features(source, "python")
    assert 0.0 <= f.literal_leaf_ratio <= 0.6, f
    assert f.control_node_density > 5.0, f
    assert f.ast_type_entropy > 1.5, f


def test_python_empty_source_returns_zero_features():
    from argot_bench.typicality import TypicalityFeatures, compute_features

    assert compute_features("", "python") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


def test_python_parse_error_returns_zero_features():
    """Unparseable Python returns the neutral-zero tuple — not a flag."""
    from argot_bench.typicality import TypicalityFeatures, compute_features

    source = "def (((("
    assert compute_features(source, "python") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


def test_python_entropy_positive_for_diverse_code():
    """ast_type_entropy is positive for any non-trivial source."""
    from argot_bench.typicality import compute_features

    source = "def f(x):\n    return x + 1\n"
    f = compute_features(source, "python")
    assert f.ast_type_entropy > 0.0, f
    assert 0.0 < f.unique_token_ratio <= 1.0, f


def test_typescript_data_heavy_has_high_literal_ratio():
    from argot_bench.typicality import compute_features

    source = """
export const TITLES = [
  "The Great Gatsby",
  "Moby Dick",
  "War and Peace",
  "Crime and Punishment",
  "Anna Karenina",
  "The Brothers Karamazov",
];
""".strip()
    f = compute_features(source, "typescript")
    assert f.literal_leaf_ratio > 0.85, f
    assert f.control_node_density == 0.0, f


def test_typescript_normal_code_has_moderate_literal_ratio():
    from argot_bench.typicality import compute_features

    source = """
function parse(request: Request, registry: Registry): Handler | null {
    const handlers = registry.lookup(request.path);
    if (handlers.length === 0) {
        throw new Error(request.path);
    }
    for (const h of handlers) {
        if (h.matches(request)) {
            return h.handle(request);
        }
    }
    return null;
}
""".strip()
    f = compute_features(source, "typescript")
    assert 0.1 <= f.literal_leaf_ratio <= 0.6, f
    assert f.control_node_density > 5.0, f


def test_typescript_parse_error_returns_zero_features():
    from argot_bench.typicality import TypicalityFeatures, compute_features

    source = "function ((("
    assert compute_features(source, "typescript") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


def test_typicality_features_tracks_named_leaf_count():
    """named_leaf_count is positive for any parseable, non-empty source."""
    from argot_bench.typicality import compute_features

    source = """
ITEMS = {
    "a": 1,
    "b": 2,
    "c": 3,
}
""".strip()
    f = compute_features(source, "python")
    assert f.named_leaf_count > 0, f


def _synthetic_normal_python_pool(n: int = 50) -> list[str]:
    """Generate a pool of structurally normal Python snippets."""
    pool: list[str] = []
    for i in range(n):
        pool.append(
            "\n".join(
                [
                    f"def fn_{i}(value, registry):",
                    "    items = registry.lookup(value)",
                    "    if not items:",
                    "        return None",
                    "    out = []",
                    "    for item in items:",
                    "        out.append(item.transform(value))",
                    "    return out",
                ]
            )
        )
    return pool


def _data_heavy_python_hunk() -> str:
    return "\n".join(
        [
            "EMOJI = {",
            *(f'    "emoji_{i}": "U+{i:05X}",' for i in range(80)),
            "}",
        ]
    )


def test_typicality_model_flags_data_heavy_against_code_pool():
    from argot_bench.typicality import TypicalityModel

    pool = _synthetic_normal_python_pool(n=60)
    model = TypicalityModel(language="python")
    model.fit(pool)

    data_hunk = _data_heavy_python_hunk()
    is_atypical, distance, features = model.is_atypical(data_hunk)
    assert is_atypical, (distance, features)
    assert features is not None
    assert features.literal_leaf_ratio > 0.8
    assert features.named_leaf_count > 30


def test_typicality_model_does_not_flag_normal_code():
    from argot_bench.typicality import TypicalityModel

    pool = _synthetic_normal_python_pool(n=60)
    model = TypicalityModel(language="python")
    model.fit(pool)

    normal_hunk = """
def parse(request, registry):
    handlers = registry.lookup(request.path)
    if not handlers:
        raise KeyError(request.path)
    for h in handlers:
        if h.matches(request):
            return h.handle(request)
    return None
""".strip()
    is_atypical, distance, features = model.is_atypical(normal_hunk)
    assert not is_atypical, (distance, features)


def test_typicality_model_handles_tiny_pool():
    """fit() is a no-op, so pool size doesn't affect the verdict on data-heavy hunks."""
    from argot_bench.typicality import TypicalityModel

    pool = _synthetic_normal_python_pool(n=4)
    model = TypicalityModel(language="python")
    model.fit(pool)

    data_hunk = _data_heavy_python_hunk()
    is_atypical, _, _ = model.is_atypical(data_hunk)
    assert is_atypical


def test_typicality_model_is_atypical_safe_on_parse_error():
    """Unparseable hunks are treated as typical (not filtered)."""
    from argot_bench.typicality import TypicalityModel

    pool = _synthetic_normal_python_pool(n=60)
    model = TypicalityModel(language="python")
    model.fit(pool)

    is_atypical, distance, features = model.is_atypical("def ((((")
    assert not is_atypical
    assert features is not None  # features will be the neutral zero tuple


def _large_data_file_python(n_entries: int = 200) -> str:
    """Generate a Python file that is data-dominant at file level (>= 100 leaves, ratio > 0.80)."""
    lines = ["DATA = {"]
    for i in range(n_entries):
        lines.append(f'    "key_{i}": "value_{i}",')
    lines.append("}")
    return "\n".join(lines)


def test_is_atypical_file_flags_large_data_file():
    """A file with 200+ literal entries is flagged at file level."""
    from argot_bench.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    source = _large_data_file_python(n_entries=200)
    is_atyp, features = model.is_atypical_file(source)
    assert features.named_leaf_count >= 100, f"leaf count too low: {features}"
    assert features.literal_leaf_ratio > 0.80, f"ratio too low: {features}"
    assert is_atyp, f"large data file not flagged: {features}"


def test_is_atypical_file_does_not_flag_code_file():
    """A real code file with control flow is not flagged at file level."""
    from argot_bench.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    # Replicate a substantial code file with many functions and branches
    lines = []
    for i in range(20):
        lines += [
            f"def fn_{i}(value, registry):",
            "    items = registry.lookup(value)",
            "    if not items:",
            "        return None",
            "    out = []",
            "    for item in items:",
            "        out.append(item.transform(value))",
            "    return out",
            "",
        ]
    source = "\n".join(lines)
    is_atyp, features = model.is_atypical_file(source)
    assert not is_atyp, f"code file wrongly flagged: {features}"


def test_is_atypical_file_returns_false_for_empty():
    """Empty source returns (False, NEUTRAL) — never filter what we couldn't parse."""
    from argot_bench.typicality import TypicalityFeatures, TypicalityModel

    model = TypicalityModel(language="python")
    is_atyp, features = model.is_atypical_file("")
    assert not is_atyp
    assert features == TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


def test_typicality_model_respects_size_gate():
    """Gate boundary: pure-literal list with < 5 leaves NOT flagged; >= 5 leaves IS flagged."""
    from argot_bench.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    model.fit([])  # fit is a no-op

    # 3-element string list: 3 named leaves (string literals only), ratio 1.0 — below gate 5
    below_gate = '["a", "b", "c"]'
    is_atypical_below, _, features_below = model.is_atypical(below_gate)
    assert features_below.named_leaf_count < 5, f"unexpectedly large: {features_below}"
    assert not is_atypical_below, f"sub-gate hunk wrongly flagged: {features_below}"

    # 5-element string list: 5 named leaves, ratio 1.0 — at gate, must fire
    above_gate = '["a", "b", "c", "d", "e"]'
    is_atypical_above, _, features_above = model.is_atypical(above_gate)
    assert features_above.named_leaf_count >= 5, f"unexpectedly small: {features_above}"
    assert features_above.literal_leaf_ratio > 0.80, f"ratio too low: {features_above}"
    assert is_atypical_above, f"at-gate hunk not flagged: {features_above}"
