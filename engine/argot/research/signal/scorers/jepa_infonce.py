from __future__ import annotations

import random
import time
from typing import Any, Literal

import numpy as np
import torch
import torch.nn.functional as F  # noqa: N812
from torch.optim import AdamW
from torch.utils.data import DataLoader, TensorDataset

from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.scorers.jepa_custom import _diverse_sample
from argot.train import ModelBundle, _texts_for_records
from argot.validate import score_records, split_by_time


class JepaInfoNCEScorer:
    """JEPA scorer trained with MSE + InfoNCE loss.

    InfoNCE sharpens the predictor by pushing predicted embeddings closer
    to the true hunk than to other in-batch hunks, without foreign data.
    At inference time only the MSE-based surprise() is used.
    """

    name = "jepa_infonce"

    def __init__(
        self,
        *,
        epochs: int = 20,
        lr: float = 1e-4,
        batch_size: int = 128,
        lambd: float = 0.09,
        beta: float = 0.25,
        tau: float = 0.07,
        warmup_epochs: int = 0,
        random_seed: int | None = None,
        sampling: Literal["linear", "diverse_kmeans", "fps"] = "linear",
        corpus_cap: int = 2000,
    ) -> None:
        self._epochs = epochs
        self._lr = lr
        self._batch_size = batch_size
        self._lambd = lambd
        self._beta = beta
        self._tau = tau
        self._warmup_epochs = warmup_epochs
        self._random_seed = random_seed
        self._sampling = sampling
        self._corpus_cap = corpus_cap
        self._bundle: ModelBundle | None = None

    def fit(
        self,
        corpus: list[dict[str, Any]],
        *,
        preencoded: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> None:
        if self._random_seed is not None:
            torch.manual_seed(self._random_seed)
            np.random.seed(self._random_seed)
            random.seed(self._random_seed)

        train_pre: tuple[torch.Tensor, torch.Tensor] | None
        if preencoded is not None:
            # corpus is pre-sorted by caller; split index mirrors split_by_time ratio
            split_idx = int(len(corpus) * 0.8)
            train_records = corpus[:split_idx]
            _train_pre: tuple[torch.Tensor, torch.Tensor] = (
                preencoded[0][:split_idx],
                preencoded[1][:split_idx],
            )
            # Apply corpus cap + diversity sampling to training split only
            if len(train_records) > self._corpus_cap or self._sampling != "linear":
                emb = (_train_pre[0] + _train_pre[1]) / 2.0  # mean of ctx+hunk embeddings
                idx = _diverse_sample(
                    emb, self._corpus_cap, self._sampling, seed=self._random_seed or 0
                )
                train_records = [train_records[i] for i in idx.tolist()]
                _train_pre = (_train_pre[0][idx], _train_pre[1][idx])
            train_pre = _train_pre
        else:
            train_records, _ = split_by_time(corpus, ratio=0.8)
            train_pre = None

        self._bundle = self._train(train_records, preencoded=train_pre)

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._bundle is None:
            raise RuntimeError("fit() must be called before score()")
        return score_records(self._bundle, fixtures)

    def _train(
        self,
        records: list[dict[str, Any]],
        *,
        preencoded: tuple[torch.Tensor, torch.Tensor] | None = None,
    ) -> ModelBundle:
        device = select_device()
        pretrained = PretrainedEncoder(device=device)
        embed_dim = pretrained.embed_dim

        if preencoded is not None:
            ctx_x, hunk_x = preencoded
            print(f"  [pre-encoded tensors: shape={ctx_x.shape}]", flush=True)
        else:
            ctx_texts, hunk_texts = _texts_for_records(records)
            n = len(records)
            print(f"  encode  {n}×2 texts on device={pretrained.torch_device} ...", flush=True)
            t0 = time.perf_counter()
            with torch.no_grad():
                ctx_x = pretrained.encode_texts(ctx_texts).cpu()
                hunk_x = pretrained.encode_texts(hunk_texts).cpu()
            dt = time.perf_counter() - t0
            rate = (2 * n) / dt if dt > 0 else 0.0
            print(f"  encode  done in {dt:.1f}s  ({rate:.0f} texts/s)", flush=True)

        loader = DataLoader(
            TensorDataset(ctx_x, hunk_x),
            batch_size=self._batch_size,
            shuffle=True,
        )

        identity_encoder = torch.nn.Identity()
        predictor = ArgotPredictor(embed_dim=embed_dim, depth=6, mlp_dim=1024)
        model = JEPAArgot(identity_encoder, predictor, lambd=self._lambd)  # type: ignore[arg-type]
        model = model.to(device)
        optimizer = AdamW(predictor.parameters(), lr=self._lr, weight_decay=1e-3)

        epochs = self._epochs
        tau = self._tau
        beta = self._beta
        warmup_epochs = self._warmup_epochs
        lambd = self._lambd

        print(
            f"  train   {epochs} epochs × {len(loader)} batches "
            f"beta={beta} tau={tau} warmup={warmup_epochs} device={device}",
            flush=True,
        )
        train_t0 = time.perf_counter()
        model.train()
        for epoch in range(1, epochs + 1):
            ep_t0 = time.perf_counter()
            total_loss = 0.0
            total_mse = 0.0
            total_infonce = 0.0
            for ctx_batch, hunk_batch in loader:
                ctx_batch = ctx_batch.to(device)
                hunk_batch = hunk_batch.to(device)
                optimizer.zero_grad()

                # --- InfoNCE loss ---
                z_ctx = model.encode(ctx_batch)
                z_hunk = model.encode(hunk_batch)
                z_pred = model.predict(z_ctx)

                mse = F.mse_loss(z_pred, z_hunk)
                sigreg_loss = model.sigreg(z_hunk.unsqueeze(0))

                # In-batch InfoNCE
                z_pred_n = F.normalize(z_pred, dim=-1)
                z_hunk_n = F.normalize(z_hunk, dim=-1)
                logits = (z_pred_n @ z_hunk_n.T) / tau
                labels = torch.arange(z_pred.shape[0], device=device)
                infonce = F.cross_entropy(logits, labels)

                # Beta warm-up: linear 0→beta over first warmup_epochs epochs
                beta_eff = beta * min(1.0, epoch / warmup_epochs) if warmup_epochs > 0 else beta

                loss = mse + beta_eff * infonce + lambd * sigreg_loss

                loss.backward()
                torch.nn.utils.clip_grad_norm_(predictor.parameters(), 1.0)
                optimizer.step()
                total_loss += loss.item()
                total_mse += mse.item()
                total_infonce += infonce.item()

            ep_dt = time.perf_counter() - ep_t0
            if epoch == 1 or epoch == epochs or epoch % 10 == 0:
                elapsed = time.perf_counter() - train_t0
                eta = ep_dt * (epochs - epoch)
                n_batches = len(loader)
                print(
                    f"  epoch   {epoch:>3d}/{epochs}  loss={total_loss / n_batches:.4f}  "
                    f"mse={total_mse / n_batches:.4f}  infonce={total_infonce / n_batches:.4f}  "
                    f"ep_time={ep_dt:.1f}s  elapsed={elapsed:.0f}s  eta={eta:.0f}s",
                    flush=True,
                )
        train_dt = time.perf_counter() - train_t0
        print(f"  train   done in {train_dt:.0f}s  ({train_dt / epochs:.1f}s/ep avg)", flush=True)

        model = model.to("cpu")
        return ModelBundle(
            vectorizer=pretrained,
            model=model,
            input_dim=embed_dim,
            embed_dim=embed_dim,
            encoder_kind="pretrained",
        )
