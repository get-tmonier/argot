from __future__ import annotations

import torch
from torch import nn


class SIGReg(nn.Module):
    """Sketch Isotropic Gaussian Regularizer (single-GPU)."""

    t: torch.Tensor
    phi: torch.Tensor
    weights: torch.Tensor

    def __init__(self, knots: int = 17, num_proj: int = 1024) -> None:
        super().__init__()
        self.num_proj = num_proj
        t = torch.linspace(0, 3, knots, dtype=torch.float32)
        dt = 3 / (knots - 1)
        w = torch.full((knots,), 2 * dt, dtype=torch.float32)
        w[[0, -1]] = dt
        window = torch.exp(-t.square() / 2.0)
        self.register_buffer("t", t)
        self.register_buffer("phi", window)
        self.register_buffer("weights", w * window)

    def forward(self, proj: torch.Tensor) -> torch.Tensor:
        """proj: (T, B, D)"""
        proj_mat = torch.randn(proj.size(-1), self.num_proj, device=proj.device)
        proj_mat = proj_mat.div_(proj_mat.norm(p=2, dim=0))
        x_t = (proj @ proj_mat).unsqueeze(-1) * self.t
        err = (x_t.cos().mean(-3) - self.phi).square() + x_t.sin().mean(-3).square()
        statistic = (err @ self.weights) * proj.size(-2)
        return statistic.mean()
