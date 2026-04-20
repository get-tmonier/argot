from __future__ import annotations

from typing import cast

import torch
import torch.nn.functional as F  # noqa: N812
from torch import nn

from argot.jepa.encoder import TokenEncoder
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.sigreg import SIGReg


class JEPAArgot(nn.Module):
    """JEPA model: context_before predicts hunk embedding."""

    def __init__(
        self,
        encoder: TokenEncoder,
        predictor: ArgotPredictor,
        lambd: float = 0.09,
        sigreg_knots: int = 17,
        sigreg_num_proj: int = 1024,
    ) -> None:
        super().__init__()
        self.encoder = encoder
        self.predictor = predictor
        self.sigreg = SIGReg(knots=sigreg_knots, num_proj=sigreg_num_proj)
        self.lambd = lambd

    def encode(self, x: torch.Tensor) -> torch.Tensor:
        """x: (B, input_dim) → (B, embed_dim)"""
        return cast(torch.Tensor, self.encoder(x))

    def predict(self, z: torch.Tensor) -> torch.Tensor:
        """z: (B, embed_dim) → (B, embed_dim)"""
        return cast(torch.Tensor, self.predictor(z.unsqueeze(1))).squeeze(1)

    def forward(self, ctx: torch.Tensor, hunk: torch.Tensor) -> dict[str, torch.Tensor]:
        """
        ctx:  (B, input_dim) — context_before TF-IDF vector
        hunk: (B, input_dim) — hunk_tokens TF-IDF vector
        Returns dict with loss, pred_loss, sigreg_loss.
        """
        z_ctx = self.encode(ctx)
        z_hunk = self.encode(hunk)
        z_pred = self.predict(z_ctx)

        pred_loss = F.mse_loss(z_pred, z_hunk)
        sigreg_loss = self.sigreg(z_hunk.unsqueeze(0))  # (1, B, D)
        loss = pred_loss + self.lambd * sigreg_loss

        return {"loss": loss, "pred_loss": pred_loss, "sigreg_loss": sigreg_loss}

    def surprise(self, ctx: torch.Tensor, hunk: torch.Tensor) -> torch.Tensor:
        """Scalar surprise score (MSE) for inference."""
        z_ctx = self.encode(ctx)
        z_hunk = self.encode(hunk)
        z_pred = self.predict(z_ctx)
        return F.mse_loss(z_pred, z_hunk, reduction="none").mean(dim=-1)

    def surprise_topk(self, ctx: torch.Tensor, hunk: torch.Tensor, k: int) -> torch.Tensor:
        """Top-k surprise: mean of the k highest squared errors per sample."""
        z_ctx = self.encode(ctx)
        z_hunk = self.encode(hunk)
        z_pred = self.predict(z_ctx)
        return F.mse_loss(z_pred, z_hunk, reduction="none").topk(k, dim=-1).values.mean(dim=-1)
