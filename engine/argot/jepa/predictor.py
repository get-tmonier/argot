from __future__ import annotations

from typing import cast

import torch
import torch.nn.functional as F  # noqa: N812
from einops import rearrange
from torch import nn


class FeedForward(nn.Module):
    """FeedForward network used in Transformers."""

    def __init__(self, dim: int, hidden_dim: int, dropout: float = 0.0) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.LayerNorm(dim),
            nn.Linear(dim, hidden_dim),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, dim),
            nn.Dropout(dropout),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return cast(torch.Tensor, self.net(x))


class Attention(nn.Module):
    """Scaled dot-product attention with causal masking."""

    def __init__(self, dim: int, heads: int = 8, dim_head: int = 64, dropout: float = 0.0) -> None:
        super().__init__()
        inner_dim = dim_head * heads
        project_out = not (heads == 1 and dim_head == dim)
        self.heads = heads
        self.scale = dim_head**-0.5
        self.dropout = dropout
        self.norm = nn.LayerNorm(dim)
        self.attend = nn.Softmax(dim=-1)
        self.to_qkv = nn.Linear(dim, inner_dim * 3, bias=False)
        self.to_out: nn.Module = (
            nn.Sequential(nn.Linear(inner_dim, dim), nn.Dropout(dropout))
            if project_out
            else nn.Identity()
        )

    def forward(self, x: torch.Tensor, causal: bool = True) -> torch.Tensor:
        """x: (B, T, D)"""
        x = self.norm(x)
        drop = self.dropout if self.training else 0.0
        qkv = self.to_qkv(x).chunk(3, dim=-1)
        q, k, v = (rearrange(t, "b t (h d) -> b h t d", h=self.heads) for t in qkv)
        out = F.scaled_dot_product_attention(q, k, v, dropout_p=drop, is_causal=causal)
        out = rearrange(out, "b h t d -> b t (h d)")
        return cast(torch.Tensor, self.to_out(out))


class Block(nn.Module):
    """Standard Transformer block."""

    def __init__(
        self, dim: int, heads: int, dim_head: int, mlp_dim: int, dropout: float = 0.0
    ) -> None:
        super().__init__()
        self.attn = Attention(dim, heads=heads, dim_head=dim_head, dropout=dropout)
        self.mlp = FeedForward(dim, mlp_dim, dropout=dropout)
        self.norm1 = nn.LayerNorm(dim, elementwise_affine=False, eps=1e-6)
        self.norm2 = nn.LayerNorm(dim, elementwise_affine=False, eps=1e-6)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = x + self.attn(self.norm1(x))
        x = x + self.mlp(self.norm2(x))
        return x


class Transformer(nn.Module):
    """Standard Transformer."""

    def __init__(
        self,
        input_dim: int,
        hidden_dim: int,
        output_dim: int,
        depth: int,
        heads: int,
        dim_head: int,
        mlp_dim: int,
        dropout: float = 0.0,
        block_class: type[Block] = Block,
    ) -> None:
        super().__init__()
        self.norm = nn.LayerNorm(hidden_dim)
        self.layers: nn.ModuleList = nn.ModuleList([])

        self.input_proj: nn.Module = (
            nn.Linear(input_dim, hidden_dim) if input_dim != hidden_dim else nn.Identity()
        )

        self.cond_proj: nn.Module = (
            nn.Linear(input_dim, hidden_dim) if input_dim != hidden_dim else nn.Identity()
        )

        self.output_proj: nn.Module = (
            nn.Linear(hidden_dim, output_dim) if hidden_dim != output_dim else nn.Identity()
        )

        for _ in range(depth):
            self.layers.append(block_class(hidden_dim, heads, dim_head, mlp_dim, dropout))

    def forward(self, x: torch.Tensor, c: torch.Tensor | None = None) -> torch.Tensor:
        x = cast(torch.Tensor, self.input_proj(x))

        for block in self.layers:
            x = block(x)
        x = self.norm(x)
        return cast(torch.Tensor, self.output_proj(x))


class ArgotPredictor(nn.Module):
    """Transformer predictor: context embedding → predicted hunk embedding."""

    def __init__(
        self,
        *,
        embed_dim: int = 192,
        depth: int = 4,
        heads: int = 8,
        mlp_dim: int = 512,
        dim_head: int = 64,
        dropout: float = 0.1,
        emb_dropout: float = 0.0,
    ) -> None:
        super().__init__()
        self.pos_embedding = nn.Parameter(torch.randn(1, 1, embed_dim))
        self.dropout = nn.Dropout(emb_dropout)
        self.transformer = Transformer(
            input_dim=embed_dim,
            hidden_dim=embed_dim,
            output_dim=embed_dim,
            depth=depth,
            heads=heads,
            dim_head=dim_head,
            mlp_dim=mlp_dim,
            dropout=dropout,
            block_class=Block,
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """x: (B, 1, embed_dim) → (B, 1, embed_dim)"""
        x = x + self.pos_embedding
        x = self.dropout(x)
        return cast(torch.Tensor, self.transformer(x))
