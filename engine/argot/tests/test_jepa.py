from __future__ import annotations

import torch

from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.sigreg import SIGReg


def test_sigreg_returns_scalar() -> None:
    reg = SIGReg(knots=5, num_proj=32)
    proj = torch.randn(1, 4, 16)  # (T, B, D)
    loss = reg(proj)
    assert loss.shape == ()
    assert loss.isfinite()


def test_token_encoder_output_shape() -> None:
    enc = TokenEncoder(input_dim=100, embed_dim=32)
    x = torch.randn(8, 100)
    out = enc(x)
    assert out.shape == (8, 32)


def test_predictor_output_shape() -> None:
    pred = ArgotPredictor(embed_dim=32, depth=2, heads=4, mlp_dim=64)
    x = torch.randn(8, 1, 32)
    out = pred(x)
    assert out.shape == (8, 1, 32)


def test_jepa_argot_forward_keys_and_finite_loss() -> None:
    model = JEPAArgot(
        TokenEncoder(input_dim=100, embed_dim=32),
        ArgotPredictor(embed_dim=32, depth=2, heads=4, mlp_dim=64),
        lambd=0.09,
        sigreg_num_proj=32,
    )
    ctx = torch.randn(4, 100)
    hunk = torch.randn(4, 100)
    out = model(ctx, hunk)
    assert set(out.keys()) == {"loss", "pred_loss", "sigreg_loss"}
    for v in out.values():
        assert v.isfinite()


def test_jepa_argot_surprise_higher_for_distant_embeddings() -> None:
    """Surprise should be larger when hunk is far from what context predicts."""
    torch.manual_seed(0)
    model = JEPAArgot(
        TokenEncoder(input_dim=100, embed_dim=32),
        ArgotPredictor(embed_dim=32, depth=2, heads=4, mlp_dim=64),
        lambd=0.09,
        sigreg_num_proj=32,
    )
    model.eval()
    ctx = torch.randn(1, 100)
    with torch.no_grad():
        close_hunk = torch.zeros(1, 100)
        random_hunk = torch.randn(1, 100) * 10  # far from anything plausible

        s_close = model.surprise(ctx, close_hunk).item()
        s_far = model.surprise(ctx, random_hunk).item()

    # Both should be finite; far hunk should generally score higher
    assert torch.isfinite(torch.tensor(s_close))
    assert torch.isfinite(torch.tensor(s_far))
    # This is a statistical test — passes with manual_seed(0)
    assert s_far > s_close
