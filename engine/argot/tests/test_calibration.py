from __future__ import annotations

import json
import tempfile
from datetime import UTC
from pathlib import Path

import pytest

from argot.scoring.calibration import load_config
from argot.scoring.calibration.random_hunk_sampler import sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_FASTAPI_FIXTURES = _CATALOG / "fastapi" / "fixtures" / "default"
_BPE_MODEL_B = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
_CONTROL_FILES = sorted(_FASTAPI_FIXTURES.glob("control_*.py"))


def _scorer_with_cal(seed: int, n: int = 5) -> SequentialImportBpeScorer:
    hunks = sample_hunks(_FASTAPI_FIXTURES, n, seed)
    return SequentialImportBpeScorer(
        model_a_files=_CONTROL_FILES,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=hunks,
    )


def test_calibration_determinism() -> None:
    """Same seed produces identical threshold on two independent runs."""
    s1 = _scorer_with_cal(seed=42)
    s2 = _scorer_with_cal(seed=42)
    assert s1.bpe_threshold == pytest.approx(s2.bpe_threshold, abs=0.0)


def test_different_seeds_may_differ() -> None:
    """Different seeds produce potentially different thresholds (probabilistic)."""
    s0 = _scorer_with_cal(seed=0)
    s1 = _scorer_with_cal(seed=99)
    # Not guaranteed to differ, but thresholds are independent
    assert isinstance(s0.bpe_threshold, float)
    assert isinstance(s1.bpe_threshold, float)


def test_empty_corpus_raises() -> None:
    """sample_hunks raises ValueError when source dir has no qualifying hunks."""
    with (
        tempfile.TemporaryDirectory() as tmp,
        pytest.raises(ValueError, match="Only 0 qualifying hunks"),
    ):
        sample_hunks(Path(tmp), n=1, seed=0)


def test_scorer_config_json_roundtrip(tmp_path: Path) -> None:
    """Write scorer-config.json then read it back with load_config."""
    scorer = _scorer_with_cal(seed=7)
    import pygit2

    try:
        repo = pygit2.Repository(str(Path(__file__).parent.parent.parent.parent))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    from datetime import datetime

    config = {
        "version": 1,
        "threshold": scorer.bpe_threshold,
        "calibration": {
            "n_cal": scorer.n_calibration,
            "seed": 7,
            "repo_sha": repo_sha,
            "timestamp_utc": datetime.now(tz=UTC).isoformat(),
        },
    }
    out = tmp_path / "scorer-config.json"
    out.write_text(json.dumps(config))

    loaded = load_config(out)
    assert loaded["version"] == 1
    threshold = loaded["threshold"]
    assert isinstance(threshold, float)
    assert threshold == pytest.approx(scorer.bpe_threshold, abs=1e-10)
    cal = loaded["calibration"]
    assert isinstance(cal, dict)
    assert cal["seed"] == 7


def test_load_config_rejects_unknown_version(tmp_path: Path) -> None:
    """load_config raises ValueError for unknown version numbers."""
    bad_config = tmp_path / "scorer-config.json"
    bad_config.write_text(json.dumps({"version": 99, "threshold": 1.0}))
    with pytest.raises(ValueError, match="Unsupported scorer-config.json version"):
        load_config(bad_config)


def test_n_calibration_matches_hunks() -> None:
    """Scorer n_calibration equals the number of calibration hunks passed."""
    hunks = sample_hunks(_FASTAPI_FIXTURES, 8, seed=3)
    scorer = SequentialImportBpeScorer(
        model_a_files=_CONTROL_FILES,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=hunks,
    )
    assert scorer.n_calibration == 8
    assert len(scorer.cal_scores) == 8
