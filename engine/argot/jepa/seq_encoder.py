from __future__ import annotations

import torch
from torch import nn


class MeanPoolEncoder(nn.Module):
    """Learned token embeddings + masked mean pooling → embed_dim."""

    def __init__(
        self,
        vocab_size: int,
        embed_dim: int = 128,
        output_dim: int = 192,
        padding_idx: int = 0,
    ) -> None:
        super().__init__()
        self.padding_idx = padding_idx
        self.embedding = nn.Embedding(vocab_size, embed_dim, padding_idx=padding_idx)
        self.proj = nn.Linear(embed_dim, output_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """x: (B, seq_len) long tensor of token IDs → (B, output_dim)"""
        mask = (x != self.padding_idx).float().unsqueeze(-1)  # (B, seq_len, 1)
        emb = self.embedding(x)  # (B, seq_len, embed_dim)
        pooled = (emb * mask).sum(dim=1) / mask.sum(dim=1).clamp(min=1.0)  # (B, embed_dim)
        return self.proj(pooled)  # type: ignore[no-any-return]
