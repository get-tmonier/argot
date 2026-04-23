from __future__ import annotations


def test_typicality_features_has_four_fields():
    from argot_bench.typicality import TypicalityFeatures

    tf = TypicalityFeatures(
        literal_leaf_ratio=0.5,
        control_node_density=2.0,
        ast_type_entropy=3.1,
        unique_token_ratio=0.4,
    )
    assert tf.literal_leaf_ratio == 0.5
    assert tf.control_node_density == 2.0
    assert tf.ast_type_entropy == 3.1
    assert tf.unique_token_ratio == 0.4


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
    # literal_leaf_ratio should be well below data-table levels (< 0.6)
    # and clearly below the 0.85 threshold used for data-heavy detection.
    # With tree-sitter Python, only 'none'/'true'/'false'/numeric/string
    # nodes count as literals; the parse snippet has ~1 literal in ~21 named
    # leaves, so the lower bound is intentionally loose.
    assert 0.0 <= f.literal_leaf_ratio <= 0.6, f
    assert f.control_node_density > 5.0, f
    assert f.ast_type_entropy > 1.5, f


def test_python_empty_source_returns_zero_features():
    from argot_bench.typicality import TypicalityFeatures, compute_features

    assert compute_features("", "python") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0)


def test_python_parse_error_returns_zero_features():
    """Unparseable Python returns the neutral-zero tuple — not a flag."""
    from argot_bench.typicality import TypicalityFeatures, compute_features

    source = "def (((("
    assert compute_features(source, "python") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0)


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
    assert compute_features(source, "typescript") == TypicalityFeatures(0.0, 0.0, 0.0, 0.0)


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
    """Even with a tiny pool, the model still flags obviously atypical hunks."""
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
