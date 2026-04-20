from __future__ import annotations

from unittest.mock import MagicMock, patch

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
