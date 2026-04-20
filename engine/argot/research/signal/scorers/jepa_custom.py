from __future__ import annotations

import random
import time
from typing import Any, Literal

import numpy as np
import torch
from torch.optim import AdamW
from torch.utils.data import DataLoader, TensorDataset

from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.jepa.pretrained_encoder import PretrainedEncoder, select_device
from argot.research.signal.base import REGISTRY
from argot.train import ModelBundle, _texts_for_records
from argot.validate import score_from_tensors, score_records, split_by_time


class JepaCustomScorer:
    """JEPA scorer with configurable predictor capacity and LR schedule.

    Duplicates _train_pretrained inline to support predictor_overrides and
    lr_schedule without modifying train.py.
    """

    name = "jepa_custom"

    def __init__(
        self,
        *,
        epochs: int = 20,
        lr: float = 1e-4,
        batch_size: int = 128,
        lambd: float = 0.09,
        lr_schedule: Literal["flat", "cosine"] = "flat",
        predictor_overrides: dict[str, int] | None = None,
        random_seed: int | None = None,
        aggregation: Literal["mean", "topk", "random_topk"] = "mean",
        topk_k: int = 64,
        zscore_vs_corpus: bool = False,
    ) -> None:
        self._epochs = epochs
        self._lr = lr
        self._batch_size = batch_size
        self._lambd = lambd
        self._lr_schedule = lr_schedule
        self._predictor_overrides = predictor_overrides or {}
        self._random_seed = random_seed
        self._aggregation = aggregation
        self._topk_k = topk_k
        self._zscore_vs_corpus = zscore_vs_corpus
        self._bundle: ModelBundle | None = None
        self._zscore_stats: tuple[float, float] | None = None

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

        if preencoded is not None:
            # corpus is pre-sorted by caller; split index mirrors split_by_time ratio
            split_idx = int(len(corpus) * 0.8)
            train_records = corpus[:split_idx]
            held_out_records = corpus[split_idx:]
            train_pre: tuple[torch.Tensor, torch.Tensor] | None = (
                preencoded[0][:split_idx],
                preencoded[1][:split_idx],
            )
            held_out_pre: tuple[torch.Tensor, torch.Tensor] | None = (
                preencoded[0][split_idx:],
                preencoded[1][split_idx:],
            )
        else:
            train_records, held_out_records = split_by_time(corpus, ratio=0.8)
            train_pre = None
            held_out_pre = None

        self._bundle = self._train(train_records, preencoded=train_pre)

        if self._zscore_vs_corpus and held_out_records:
            if held_out_pre is not None:
                ref_scores = score_from_tensors(
                    self._bundle,
                    held_out_pre[0],
                    held_out_pre[1],
                    aggregation=self._aggregation,
                    topk_k=self._topk_k,
                )
            else:
                ref_scores = score_records(
                    self._bundle,
                    held_out_records,
                    aggregation=self._aggregation,
                    topk_k=self._topk_k,
                )
            self._zscore_stats = (float(np.mean(ref_scores)), float(np.std(ref_scores)))

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        if self._bundle is None:
            raise RuntimeError("fit() must be called before score()")
        return score_records(
            self._bundle,
            fixtures,
            aggregation=self._aggregation,
            topk_k=self._topk_k,
            zscore_ref_stats=self._zscore_stats if self._zscore_vs_corpus else None,
        )

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
        predictor = ArgotPredictor(embed_dim=embed_dim, **self._predictor_overrides)
        model = JEPAArgot(identity_encoder, predictor, lambd=self._lambd)  # type: ignore[arg-type]
        model = model.to(device)
        optimizer = AdamW(predictor.parameters(), lr=self._lr, weight_decay=1e-3)

        scheduler = (
            torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=self._epochs)
            if self._lr_schedule == "cosine"
            else None
        )

        epochs = self._epochs
        print(
            f"  train   {epochs} epochs × {len(loader)} batches "
            f"schedule={self._lr_schedule} device={device}",
            flush=True,
        )
        train_t0 = time.perf_counter()
        model.train()
        for epoch in range(1, epochs + 1):
            ep_t0 = time.perf_counter()
            total_loss = 0.0
            for ctx_batch, hunk_batch in loader:
                ctx_batch = ctx_batch.to(device)
                hunk_batch = hunk_batch.to(device)
                optimizer.zero_grad()
                losses = model(ctx_batch, hunk_batch)
                losses["loss"].backward()
                torch.nn.utils.clip_grad_norm_(predictor.parameters(), 1.0)
                optimizer.step()
                total_loss += losses["loss"].item()
            if scheduler is not None:
                scheduler.step()
            ep_dt = time.perf_counter() - ep_t0
            if epoch == 1 or epoch == epochs or epoch % 10 == 0:
                elapsed = time.perf_counter() - train_t0
                eta = ep_dt * (epochs - epoch)
                print(
                    f"  epoch   {epoch:>3d}/{epochs}  loss={total_loss / len(loader):.4f}  "
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


REGISTRY["jepa_custom"] = JepaCustomScorer
