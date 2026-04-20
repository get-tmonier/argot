from __future__ import annotations

from unittest.mock import MagicMock, patch

from argot.research.signal.scorers.ensemble_jepa import EnsembleJepaScorer
from argot.research.signal.scorers.jepa_pretrained import JepaPretrainedScorer


def test_jepa_pretrained_seeding() -> None:
    """random_seed=N causes all three RNGs to be seeded with N before training."""
    corpus = [{"hunk_tokens": [{"text": "x"}], "file_path": "a.py", "context_before": []}]

    mock_bundle = MagicMock()
    with (
        patch(
            "argot.research.signal.scorers.jepa_pretrained.split_by_time",
            return_value=(corpus, []),
        ),
        patch(
            "argot.research.signal.scorers.jepa_pretrained.train_model",
            return_value=mock_bundle,
        ),
        patch(
            "argot.research.signal.scorers.jepa_pretrained.score_records",
            return_value=[0.5],
        ),
        patch("torch.manual_seed") as mock_torch_seed,
        patch("numpy.random.seed") as mock_np_seed,
        patch("random.seed") as mock_py_seed,
    ):
        scorer = JepaPretrainedScorer(random_seed=42)
        scorer.fit(corpus)

    mock_torch_seed.assert_called_once_with(42)
    mock_np_seed.assert_called_once_with(42)
    mock_py_seed.assert_called_once_with(42)


def test_ensemble_averages_scores_across_members() -> None:
    """EnsembleJepaScorer averages N member scores and seeds them consecutively."""
    corpus = [{"hunk_tokens": [{"text": "x"}], "file_path": "a.py", "context_before": []}]
    fixtures = [{"hunk_tokens": [{"text": "y"}], "context_before": []}]

    member_scores = [[0.1], [0.3], [0.5]]  # 3 members, 1 fixture each
    mock_member = MagicMock()
    mock_member.score.side_effect = member_scores

    with patch(
        "argot.research.signal.scorers.ensemble_jepa.JepaCustomScorer",
        return_value=mock_member,
    ) as mock_cls:
        scorer = EnsembleJepaScorer(n=3, base_seed=7)
        scorer.fit(corpus)
        result = scorer.score(fixtures)

    # Three members constructed with consecutive seeds 7, 8, 9
    assert mock_cls.call_count == 3
    seeds = [c.kwargs["random_seed"] for c in mock_cls.call_args_list]
    assert seeds == [7, 8, 9]

    # Scores averaged: (0.1 + 0.3 + 0.5) / 3 ≈ 0.3
    assert len(result) == 1
    assert abs(result[0] - (0.1 + 0.3 + 0.5) / 3) < 1e-9


def test_ensemble_score_before_fit_raises() -> None:
    scorer = EnsembleJepaScorer()
    try:
        scorer.score([])
        raise AssertionError("Expected RuntimeError")
    except RuntimeError:
        pass
