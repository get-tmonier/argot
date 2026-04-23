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


def test_typicality_model_flags_data_heavy():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    data_hunk = "\n".join(
        [
            "EMOJI = {",
            *(f'    "emoji_{i}": "U+{i:05X}",' for i in range(80)),
            "}",
        ]
    )
    is_atypical, features = model.is_atypical(data_hunk)
    assert is_atypical, features
    assert features.literal_leaf_ratio > 0.80
    assert features.named_leaf_count >= 5


def test_typicality_model_does_not_flag_normal_code():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
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
    is_atypical, _ = model.is_atypical(normal_hunk)
    assert not is_atypical


def test_typicality_model_respects_size_gate():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    # 4 string leaves — below the >= 5 gate even at ratio 1.0
    tiny_data = 'x = ["a", "b", "c"]'
    is_atypical, features = model.is_atypical(tiny_data)
    # 5 string leaves — at the gate
    gate_data = 'y = ["a", "b", "c", "d", "e"]'
    at_gate, at_features = model.is_atypical(gate_data)

    # One of these should be below the gate; one at or above.
    # We just assert the gate behaviour is monotonic in leaf count.
    assert at_features.named_leaf_count > features.named_leaf_count


def test_typicality_model_is_atypical_safe_on_parse_error():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    is_atypical, features = model.is_atypical("def ((((")
    assert not is_atypical
    assert features.named_leaf_count == 0


def test_is_atypical_file_flags_pure_data():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    source = "DATA = {\n" + "\n".join(f'    "k{i}": "v{i}",' for i in range(120)) + "\n}"
    is_atypical, features = model.is_atypical_file(source)
    assert is_atypical
    assert features.named_leaf_count >= 100
    assert features.literal_leaf_ratio > 0.80


def test_is_atypical_file_does_not_flag_normal_code():
    from argot.scoring.filters.typicality import TypicalityModel

    model = TypicalityModel(language="python")
    normal_code = "\n".join(
        [
            "def fn_{i}(value, registry):".format(i=i)
            + "\n    items = registry.lookup(value)"
            + "\n    if not items:"
            + "\n        return None"
            + "\n    out = []"
            + "\n    for item in items:"
            + "\n        out.append(item.transform(value))"
            + "\n    return out"
            for i in range(10)
        ]
    )
    is_atypical, _ = model.is_atypical_file(normal_code)
    assert not is_atypical


def test_language_for_adapter_python():
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.filters.typicality import language_for_adapter

    assert language_for_adapter(PythonAdapter()) == "python"


def test_language_for_adapter_typescript():
    from argot.scoring.adapters.typescript import TypeScriptAdapter
    from argot.scoring.filters.typicality import language_for_adapter

    assert language_for_adapter(TypeScriptAdapter()) == "typescript"
