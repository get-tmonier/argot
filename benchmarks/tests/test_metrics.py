from argot_bench.metrics import (
    auc_catalog,
    fp_rate,
    recall_by_category,
    stage_attribution,
)


def test_auc_catalog_perfect_separation():
    breaks = [5.0, 4.0, 3.5]
    controls = [1.0, 0.5, 0.2]
    assert auc_catalog(breaks, controls) == 1.0


def test_auc_catalog_fully_inverted():
    breaks = [0.2, 0.3, 0.4]
    controls = [1.0, 0.9, 0.8]
    assert auc_catalog(breaks, controls) == 0.0


def test_auc_catalog_random():
    breaks = [1.0, 2.0]
    controls = [1.5, 1.5]
    # Two ties vs two positives → 0.5
    assert 0.25 <= auc_catalog(breaks, controls) <= 0.75


def test_recall_by_category_groups_and_averages():
    results = [
        {"category": "A", "flagged": True},
        {"category": "A", "flagged": False},
        {"category": "A", "flagged": True},
        {"category": "B", "flagged": True},
        {"category": "B", "flagged": True},
    ]
    r = recall_by_category(results)
    assert r == {"A": 2 / 3, "B": 1.0}


def test_fp_rate_counts_flagged_over_total():
    hunks = [{"flagged": True}, {"flagged": False}, {"flagged": False}]
    assert fp_rate(hunks) == 1 / 3


def test_fp_rate_empty_returns_zero():
    assert fp_rate([]) == 0.0


def test_stage_attribution_counts_reasons():
    results = [
        {"reason": "import"},
        {"reason": "import"},
        {"reason": "bpe"},
        {"reason": "none"},
    ]
    att = stage_attribution(results)
    assert att == {"import": 2, "bpe": 1, "none": 1}
