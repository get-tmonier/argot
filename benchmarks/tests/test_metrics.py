from argot_bench.metrics import (
    auc_catalog,
    calibration_stability,
    fp_rate,
    recall_by_category,
    stage_attribution,
    threshold_cv,
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


def test_threshold_cv_identical_seeds():
    assert threshold_cv([1.0, 1.0, 1.0, 1.0, 1.0]) == 0.0


def test_threshold_cv_computes_std_over_mean():
    # mean = 2, std (population) ≈ 0.8165 → CV ≈ 0.408
    thresholds = [1.0, 2.0, 3.0]
    cv = threshold_cv(thresholds)
    assert abs(cv - 0.408) < 0.01


def test_threshold_cv_zero_mean_returns_nan_sentinel():
    # Defensive: empty or all-zero inputs return 0.0 (can't be computed)
    assert threshold_cv([]) == 0.0
    assert threshold_cv([0.0, 0.0]) == 0.0


def test_calibration_stability_jaccard_and_rel_var():
    # Three seed-specific cal score sets
    sets_of_ids = [
        {"h1", "h2", "h3", "h4"},
        {"h1", "h2", "h3", "h5"},
        {"h1", "h2", "h3", "h6"},
    ]
    thresholds = [2.5, 2.6, 2.4]  # mean 2.5, rel_var = variance / mean = 0.00667/2.5
    result = calibration_stability(sets_of_ids, thresholds)
    # Jaccard pair-average: intersection 3 / union 5 = 0.6 for each pair
    assert abs(result["jaccard"] - 0.6) < 1e-9
    assert abs(result["rel_var"] - (0.00666667 / 2.5)) < 1e-4


def test_recall_by_difficulty_groups_correctly():
    results = [
        {"difficulty": "easy", "flagged": True},
        {"difficulty": "easy", "flagged": True},
        {"difficulty": "easy", "flagged": False},
        {"difficulty": "medium", "flagged": True},
        {"difficulty": "medium", "flagged": False},
        {"difficulty": "hard", "flagged": True},
        {"difficulty": "uncaught", "flagged": False},
        {"difficulty": None, "flagged": True},
    ]
    from argot_bench.metrics import recall_by_difficulty
    r = recall_by_difficulty(results)
    assert abs(r["easy"] - 2/3) < 1e-9
    assert r["medium"] == 0.5
    assert r["hard"] == 1.0
    assert r["uncaught"] == 0.0
    assert None not in r


def test_recall_by_difficulty_empty_returns_empty():
    from argot_bench.metrics import recall_by_difficulty
    assert recall_by_difficulty([]) == {}


def test_recall_by_difficulty_skips_none_difficulty():
    from argot_bench.metrics import recall_by_difficulty
    assert recall_by_difficulty([{"difficulty": None, "flagged": True}]) == {}


def test_metrics_stage_attribution_counts_call_receiver():
    from argot_bench.metrics import stage_attribution

    results = [
        {"reason": "import"},
        {"reason": "call_receiver"},
        {"reason": "call_receiver"},
        {"reason": "bpe"},
        {"reason": "none"},
    ]
    sa = stage_attribution(results)
    assert sa.get("call_receiver") == 2
