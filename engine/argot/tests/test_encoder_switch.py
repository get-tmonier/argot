from __future__ import annotations

from argot.train import train_model
from argot.validate import score_records


def _make_records(n: int) -> list[dict[str, object]]:
    return [
        {
            "context_before": [{"text": f"token{i}"}],
            "hunk_tokens": [{"text": f"hunk{i}"}],
            "author_date_iso": str(1000 + i),
            "_repo": "alpha" if i % 2 == 0 else "beta",
        }
        for i in range(n)
    ]


def test_tfidf_encoder_kind_and_scores() -> None:
    records = _make_records(60)
    bundle = train_model(records, epochs=1, encoder="tfidf")
    assert bundle.encoder_kind == "tfidf"
    scores = score_records(bundle, records[:5])
    assert len(scores) == 5
    assert all(isinstance(s, float) for s in scores)
