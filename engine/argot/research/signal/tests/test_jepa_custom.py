from __future__ import annotations

from unittest.mock import MagicMock, patch

from argot.research.signal.scorers.jepa_pretrained import JepaPretrainedScorer
from argot.research.signal.tests.conftest import make_corpus, make_fixtures


def test_jepa_pretrained_kwargs_respected() -> None:
    corpus = make_corpus()
    fixtures = make_fixtures()
    fake_bundle = MagicMock()
    fake_scores = [0.5, 0.8]

    with (
        patch(
            "argot.research.signal.scorers.jepa_pretrained.split_by_time",
            return_value=(corpus, []),
        ) as mock_split,
        patch(
            "argot.research.signal.scorers.jepa_pretrained.train_model",
            return_value=fake_bundle,
        ) as mock_train,
        patch(
            "argot.research.signal.scorers.jepa_pretrained.score_records",
            return_value=fake_scores,
        ) as mock_score,
    ):
        scorer = JepaPretrainedScorer(epochs=42, lr=1e-4, batch_size=64, lambd=0.05)
        scorer.fit(corpus)
        result = scorer.score(fixtures)

    # split_by_time called with correct ratio
    mock_split.assert_called_once_with(corpus, ratio=0.8)

    # train_model called with the custom kwargs
    mock_train.assert_called_once_with(
        corpus,
        encoder="pretrained",
        epochs=42,
        lr=1e-4,
        batch_size=64,
        lambd=0.05,
    )

    # score_records called with the bundle returned by train_model
    mock_score.assert_called_once_with(fake_bundle, fixtures)

    assert result == fake_scores
