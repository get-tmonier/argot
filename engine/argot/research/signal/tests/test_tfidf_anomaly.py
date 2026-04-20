from __future__ import annotations

from argot.research.signal.scorers.tfidf_anomaly import TfidfAnomalyScorer


def test_tfidf_anomaly_ordering() -> None:
    scorer = TfidfAnomalyScorer()

    corpus_records = [
        {"hunk_tokens": [{"text": "foo"}, {"text": "bar"}]},
        {"hunk_tokens": [{"text": "foo"}, {"text": "baz"}]},
        {"hunk_tokens": [{"text": "bar"}, {"text": "baz"}]},
        {"hunk_tokens": [{"text": "foo"}, {"text": "foo"}]},
        {"hunk_tokens": [{"text": "bar"}, {"text": "bar"}]},
        {"hunk_tokens": [{"text": "baz"}, {"text": "baz"}]},
    ]
    scorer.fit(corpus_records)

    in_vocab_fixture = [{"hunk_tokens": [{"text": "foo"}, {"text": "bar"}]}]
    out_of_vocab_fixture = [{"hunk_tokens": [{"text": "zzz"}, {"text": "qqq"}]}]

    fixture_records = in_vocab_fixture + out_of_vocab_fixture
    scores = scorer.score(fixture_records)

    assert len(scores) == 2
    assert (
        scores[1] > scores[0]
    ), f"Expected in-vocab({scores[0]:.4f}) < out-of-vocab({scores[1]:.4f})"
