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
