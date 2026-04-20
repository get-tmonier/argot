from __future__ import annotations

import torch
import torch.nn.functional as F
from unittest.mock import MagicMock

from argot.research.signal.scorers.knn_cosine import KnnCosineScorer


def test_knn_cosine_ordering() -> None:
    scorer = KnnCosineScorer.__new__(KnnCosineScorer)
    scorer._corpus_emb = None

    corpus_emb = F.normalize(torch.randn(3, 16), p=2, dim=1)
    identical = corpus_emb[0:1].clone()
    foreign = F.normalize(torch.randn(1, 16), p=2, dim=1)

    mock_encoder = MagicMock()
    scorer._encoder = mock_encoder

    mock_encoder.encode_texts.return_value = corpus_emb.clone()
    corpus_records = [{"hunk_tokens": [{"text": f"t{i}"}]} for i in range(3)]
    scorer.fit(corpus_records)

    mock_encoder.encode_texts.return_value = torch.cat([identical, foreign], dim=0)
    fixture_records = [{"hunk_tokens": [{"text": "t0"}]}, {"hunk_tokens": [{"text": "t99"}]}]
    scores = scorer.score(fixture_records)

    assert len(scores) == 2
    assert scores[0] < scores[1], f"Expected identical({scores[0]:.4f}) < foreign({scores[1]:.4f})"
