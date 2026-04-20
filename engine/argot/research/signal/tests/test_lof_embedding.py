from __future__ import annotations

from unittest.mock import MagicMock

import torch
import torch.nn.functional as F  # noqa: N812

from argot.research.signal.scorers.lof_embedding import LofEmbeddingScorer


def test_lof_embedding_ordering() -> None:
    scorer = LofEmbeddingScorer.__new__(LofEmbeddingScorer)
    scorer._lof = None

    # Corpus: 10 points tightly clustered near +e₀. Outlier points at -e₀,
    # guaranteeing cosine distance ≈ 2 from all corpus points (vs ≈ 0 in-distribution).
    gen = torch.Generator().manual_seed(0)
    base = torch.zeros(10, 16)
    base[:, 0] = 1.0
    corpus_emb = F.normalize(base + torch.randn(10, 16, generator=gen) * 0.05, p=2, dim=1)
    in_distribution = corpus_emb[0:1].clone()
    outlier = F.normalize(torch.tensor([[-1.0] + [0.0] * 15]), p=2, dim=1)

    mock_encoder = MagicMock()
    scorer._encoder = mock_encoder

    mock_encoder.encode_texts.return_value = corpus_emb.clone()
    corpus_records = [{"hunk_tokens": [{"text": f"t{i}"}]} for i in range(10)]
    scorer.fit(corpus_records)

    mock_encoder.encode_texts.return_value = torch.cat([in_distribution, outlier], dim=0)
    fixture_records = [
        {"hunk_tokens": [{"text": "t0"}]},
        {"hunk_tokens": [{"text": "t99"}]},
    ]
    scores = scorer.score(fixture_records)

    assert len(scores) == 2
    assert (
        scores[1] > scores[0]
    ), f"Expected outlier({scores[1]:.4f}) > in_distribution({scores[0]:.4f})"
