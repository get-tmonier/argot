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


def test_compute_features_stub_raises_not_implemented():
    from argot_bench.typicality import compute_features

    try:
        compute_features("x = 1", "python")
    except NotImplementedError:
        return
    raise AssertionError("expected NotImplementedError")
