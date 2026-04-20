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


def _diverse_sample(
    embeddings: torch.Tensor,
    n: int,
    method: Literal["linear", "diverse_kmeans", "fps"],
    seed: int = 0,
) -> torch.Tensor:
    """Return indices of n records selected by the given strategy.

    linear:         first n indices (existing behaviour)
    diverse_kmeans: KMeans(n_clusters=min(8, n//32)) on embeddings, equal
                    count per cluster (floor division, remainder dropped)
    fps:            greedy farthest-point selection starting from a random seed
    """
    total = len(embeddings)
    if n >= total:
        return torch.arange(total, dtype=torch.long)

    if method == "linear":
        return torch.arange(n, dtype=torch.long)

    if method == "diverse_kmeans":
        from sklearn.cluster import KMeans

        n_clusters = min(8, n // 32)
        n_clusters = max(1, n_clusters)  # clamp to at least 1
        per_cluster = n // n_clusters
        if per_cluster == 0:
            # n < n_clusters: just return first n
            return torch.arange(n, dtype=torch.long)

        emb_np = embeddings.cpu().numpy()
        km = KMeans(n_clusters=n_clusters, random_state=seed, n_init="auto")
        labels = km.fit_predict(emb_np)
        rng = np.random.default_rng(seed)
        selected: list[int] = []
        for c in range(n_clusters):
            cluster_idx = np.where(labels == c)[0]
            rng.shuffle(cluster_idx)
            selected.extend(cluster_idx[:per_cluster].tolist())
        return torch.tensor(selected, dtype=torch.long)

    # fps
    rng = np.random.default_rng(seed)
    emb_cpu = embeddings.cpu().float()
    first = int(rng.integers(total))
    selected_fps: list[int] = [first]
    # min_dists[i] = squared L2 distance from point i to nearest selected point
    diff = emb_cpu - emb_cpu[first]
    min_dists = (diff * diff).sum(dim=1).numpy()
    min_dists[first] = -1.0  # mark as selected

    for _ in range(n - 1):
        next_idx = int(np.argmax(min_dists))
        selected_fps.append(next_idx)
        diff = emb_cpu - emb_cpu[next_idx]
        new_dists = (diff * diff).sum(dim=1).numpy()
        min_dists = np.minimum(min_dists, new_dists)
        min_dists[next_idx] = -1.0  # mark as selected

    return torch.tensor(selected_fps, dtype=torch.long)


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
        sampling: Literal["linear", "diverse_kmeans", "fps"] = "linear",
        corpus_cap: int = 2000,
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
        self._sampling = sampling
        self._corpus_cap = corpus_cap
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

        train_pre: tuple[torch.Tensor, torch.Tensor] | None
        held_out_pre: tuple[torch.Tensor, torch.Tensor] | None
        if preencoded is not None:
            # corpus is pre-sorted by caller; split index mirrors split_by_time ratio
            split_idx = int(len(corpus) * 0.8)
            train_records = corpus[:split_idx]
            held_out_records = corpus[split_idx:]
            _train_pre: tuple[torch.Tensor, torch.Tensor] = (
                preencoded[0][:split_idx],
                preencoded[1][:split_idx],
            )
            held_out_pre = (
                preencoded[0][split_idx:],
                preencoded[1][split_idx:],
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
