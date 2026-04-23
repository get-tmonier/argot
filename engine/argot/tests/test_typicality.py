from __future__ import annotations


def test_typicality_features_has_five_fields():
    from argot.scoring.filters.typicality import TypicalityFeatures

    tf = TypicalityFeatures(
        literal_leaf_ratio=0.5,
        control_node_density=2.0,
        ast_type_entropy=3.1,
        unique_token_ratio=0.4,
        named_leaf_count=42,
    )
    assert tf.literal_leaf_ratio == 0.5
    assert tf.named_leaf_count == 42


def test_python_data_heavy_has_high_literal_ratio():
    from argot.scoring.filters.typicality import compute_features

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
    assert f.named_leaf_count > 5, f


def test_python_normal_code_has_moderate_literal_ratio():
    from argot.scoring.filters.typicality import compute_features

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


def test_python_empty_and_invalid_return_neutral():
    from argot.scoring.filters.typicality import TypicalityFeatures, compute_features

    neutral = TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)
    assert compute_features("", "python") == neutral
    assert compute_features("def ((((", "python") == neutral


def test_typescript_data_heavy_has_high_literal_ratio():
    from argot.scoring.filters.typicality import compute_features

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


def test_typescript_normal_code_has_moderate_literal_ratio():
    from argot.scoring.filters.typicality import compute_features

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
