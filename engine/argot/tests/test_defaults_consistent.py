from __future__ import annotations

import ast
import inspect
from pathlib import Path

from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_ENGINE_ROOT = Path(__file__).parent.parent


def test_call_receiver_alpha_defaults_match_across_layers() -> None:
    sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_alpha = sig.parameters["call_receiver_alpha"].default

    calib_src = (_ENGINE_ROOT / "scoring" / "calibration" / "__init__.py").read_text()
    calib_tree = ast.parse(calib_src)
    calib_matches = [
        float(node.value.value)
        for node in ast.walk(calib_tree)
        if (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.target.id == "call_receiver_alpha"
            and node.value is not None
            and isinstance(node.value, ast.Constant)
        )
    ]
    assert (
        len(calib_matches) == 1
    ), f"Expected 1 call_receiver_alpha in calibration/__init__.py, found {len(calib_matches)}"
    calib_alpha = calib_matches[0]

    check_src = (_ENGINE_ROOT / "check.py").read_text()
    check_tree = ast.parse(check_src)
    check_matches = [
        float(node.args[1].value)
        for node in ast.walk(check_tree)
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "get"
            and len(node.args) >= 2
            and isinstance(node.args[0], ast.Constant)
            and node.args[0].value == "call_receiver_alpha"
            and isinstance(node.args[1], ast.Constant)
        )
    ]
    assert (
        len(check_matches) == 1
    ), f"Expected 1 call_receiver_alpha fallback in check.py, found {len(check_matches)}"
    check_fallback = check_matches[0]

    assert scorer_alpha == calib_alpha == check_fallback == 2.0, (
        f"Alpha defaults drifted: scorer={scorer_alpha}, calibrate={calib_alpha}, "
        f"check_fallback={check_fallback}. Update all together."
    )


def test_threshold_percentile_default_is_max_formula() -> None:
    """Scorer and calibration CLI must both default to the max formula (era-10 shipping config).

    The scorer uses None (= max(cal_scores)); the CLI uses 100.0 (= p100 = max).
    Enforces consistent defaults across SequentialImportBpeScorer and CLI entry-point.
    """
    sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_pct = sig.parameters["threshold_percentile"].default

    calib_src = (_ENGINE_ROOT / "scoring" / "calibration" / "__init__.py").read_text()
    calib_tree = ast.parse(calib_src)

    # Extract the 'default' keyword value from the --threshold-percentile add_argument call
    calib_pct_list: list[float] = []
    for node in ast.walk(calib_tree):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "add_argument"
        ):
            has_flag = any(
                isinstance(a, ast.Constant) and "--threshold-percentile" in str(a.value)
                for a in node.args
            )
            if has_flag:
                for kw in node.keywords:
                    if kw.arg == "default" and isinstance(kw.value, ast.Constant):
                        calib_pct_list.append(float(kw.value.value))

    assert len(calib_pct_list) == 1, (
        f"Expected 1 --threshold-percentile default in calibration/__init__.py, "
        f"found {len(calib_pct_list)}"
    )
    calib_pct = calib_pct_list[0]

    # Scorer uses None (max formula); CLI uses 100.0 (p100 = max). Both represent the same
    # era-10 shipping config: max(cal_scores) as the threshold.
    assert scorer_pct is None, (
        f"SequentialImportBpeScorer.threshold_percentile default should be None (max formula, "
        f"era-10 shipping), got {scorer_pct!r}."
    )
    assert calib_pct == 100.0, (
        f"calibration CLI --threshold-percentile default should be 100.0 (max formula, "
        f"era-10 shipping), got {calib_pct!r}."
    )
