from __future__ import annotations

from pathlib import Path
from unittest.mock import MagicMock, patch

import torch
import pytest

from argot.research.signal.runner import _run_entry


def test_runner_smoke_ky(tmp_path: Path) -> None:
    catalog_dir = Path(__file__).parents[3] / "acceptance" / "catalog"
    if not (catalog_dir / "ky").exists():
        pytest.skip("ky catalog entry not found")

    with patch("argot.research.signal.scorers.knn_cosine.PretrainedEncoder") as mock_enc_cls:
        mock_enc = MagicMock()
        mock_enc_cls.return_value = mock_enc
        mock_enc.torch_device = torch.device("cpu")

        def fake_encode(texts: list[str]) -> torch.Tensor:
            n = len(texts)
            return torch.ones(n, 16) / (16 ** 0.5)

        mock_enc.encode_texts.side_effect = fake_encode

        _run_entry("ky", catalog_dir, ["knn_cosine"], tmp_path)

    out_file = tmp_path / "ky.md"
    assert out_file.exists()
    content = out_file.read_text()
    assert "## Raw Scores" in content
    assert "## Ranks" in content
    assert "## Summary" in content
    assert "knn_cosine" in content
