from __future__ import annotations

from unittest.mock import MagicMock, patch

from argot.research.signal.scorers.jepa_custom import JepaCustomScorer
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

    mock_split.assert_called_once_with(corpus, ratio=0.8)
    mock_train.assert_called_once_with(
        corpus,
        encoder="pretrained",
        epochs=42,
        lr=1e-4,
        batch_size=64,
        lambd=0.05,
    )
    mock_score.assert_called_once_with(fake_bundle, fixtures)
    assert result == fake_scores


def test_jepa_custom_predictor_overrides_and_schedule() -> None:
    """JepaCustomScorer passes predictor_overrides and lr_schedule to _train."""
    corpus = make_corpus()
    fixtures = make_fixtures()
    fake_bundle = MagicMock()

    with (
        patch("argot.research.signal.scorers.jepa_custom.split_by_time", return_value=(corpus, [])),
        patch("argot.research.signal.scorers.jepa_custom.score_records", return_value=[0.3, 0.7]),
        patch.object(JepaCustomScorer, "_train", return_value=fake_bundle) as mock_train,
    ):
        scorer = JepaCustomScorer(
            epochs=20,
            lr=1e-4,
            lr_schedule="cosine",
            predictor_overrides={"depth": 6, "mlp_dim": 1024},
        )
        scorer.fit(corpus)
        result = scorer.score(fixtures)

    mock_train.assert_called_once_with(corpus, preencoded=None)
    assert scorer._lr_schedule == "cosine"
    assert scorer._predictor_overrides == {"depth": 6, "mlp_dim": 1024}
    assert result == [0.3, 0.7]


def test_jepa_custom_score_before_fit_raises() -> None:
    scorer = JepaCustomScorer()
    try:
        scorer.score([])
        assert False, "Expected RuntimeError"
    except RuntimeError:
        pass
