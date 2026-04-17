from __future__ import annotations

from typing import cast

import torch
from torch import nn


class TokenEncoder(nn.Module):
    """MLP encoder from TF-IDF vectors to latent embeddings."""

    def __init__(self, input_dim: int = 5000, embed_dim: int = 192) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(input_dim, 512),
            nn.GELU(),
            nn.Linear(512, embed_dim),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """x: (B, input_dim) → (B, embed_dim)"""
        return cast(torch.Tensor, self.net(x))
