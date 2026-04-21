"""Delta-MLM scorer — adapts UniXCoder on the target corpus via LoRA, then measures
``surprise_base - surprise_adapted`` per hunk token.

Addresses the "inverted category" problem: rare-but-idiomatic patterns (e.g. FastAPI's
Depends(), BackgroundTasks) look surprising to the *base* UniXCoder prior, causing
zero-shot MLM to mislabel idiomatic code as anomalous.  After fine-tuning on the corpus,
idiomatic patterns become *less* surprising to the adapted model, so
``delta = surprise_base - surprise_adapted = logprob_adapted - logprob_base`` is *high*
for idiomatic code and *low* for paradigm breaks — inverting the direction relative to
raw surprise.

Three registered variants:
  delta_mlm_mean  — mean delta across hunk positions
  delta_mlm_min   — minimum delta (worst-case single position)
  delta_mlm_p05   — 5th-percentile delta (tail-extreme)
"""

from __future__ import annotations

import math
import random
from typing import Any, Literal

import torch
import torch.nn.functional as F  # noqa: N812
from transformers import AutoModelForMaskedLM, AutoTokenizer

from argot.jepa.pretrained_encoder import select_device
from argot.research.signal.base import REGISTRY

# ---------------------------------------------------------------------------
# Optional peft import — fall back to MLM-head-only fine-tuning if unavailable.
# ---------------------------------------------------------------------------
try:
    from peft import LoraConfig, TaskType, get_peft_model

    _PEFT_AVAILABLE = True
except ImportError:  # pragma: no cover
    _PEFT_AVAILABLE = False

_MODEL_ID = "microsoft/unixcoder-base"
_MAX_LENGTH = 512
_DEFAULT_SCORE_BATCH = 32


# ---------------------------------------------------------------------------
# Adapter training helper
# ---------------------------------------------------------------------------


class DeltaMlmAdapter:
    """Fine-tune UniXCoder MLM head on a corpus using LoRA (or fallback head-only tuning).

    Parameters
    ----------
    n_steps:
        Number of gradient steps.
    lr:
        AdamW learning rate.
    batch_size:
        Number of sequences per gradient step.
    mask_prob:
        Fraction of tokens to mask during MLM training.
    """

    def __init__(
        self,
        *,
        n_steps: int = 200,
        lr: float = 5e-5,
        batch_size: int = 8,
        mask_prob: float = 0.15,
    ) -> None:
        self._n_steps = n_steps
        self._lr = lr
        self._batch_size = batch_size
        self._mask_prob = mask_prob
        self._model: Any = None

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def train(self, corpus: list[dict[str, Any]]) -> None:
        """Fine-tune UniXCoder MLM head on corpus texts.

        If the ``peft`` library is available, applies LoRA to ``query`` and ``value``
        projection matrices.  Otherwise, falls back to fine-tuning only the MLM
        classification head (``cls`` parameters), which avoids touching the encoder
        backbone weights while still adapting token predictions.
        """
        device = select_device()
        tokenizer: Any = AutoTokenizer.from_pretrained(_MODEL_ID)  # type: ignore[no-untyped-call]
        base: Any = AutoModelForMaskedLM.from_pretrained(_MODEL_ID)

        if _PEFT_AVAILABLE:
            # Apply LoRA — targets query/value projections inside the encoder.
            # TaskType.TOKEN_CLS is the closest available task type to masked LM.
            lora_config = LoraConfig(
                task_type=TaskType.TOKEN_CLS,
                r=8,
                lora_alpha=16,
                target_modules=["query", "value"],
                lora_dropout=0.1,
                bias="none",
            )
            model: Any = get_peft_model(base, lora_config)
        else:
            # Fallback: freeze backbone, only train the MLM classification head.
            for p in base.parameters():
                p.requires_grad = False
            # `cls` is the MLM head in UniXCoder / RoBERTa-family models
            for p in base.cls.parameters():
                p.requires_grad = True
            model = base

        model = model.to(device)
        model.train()

        trainable = [p for p in model.parameters() if p.requires_grad]
        if not trainable:
            # Nothing to train (e.g. all params frozen by mock or misconfiguration).
            model.eval()
            self._model = model
            return
        optimizer = torch.optim.AdamW(trainable, lr=self._lr)

        mask_token_id: int = int(tokenizer.mask_token_id)
        cls_id: int = int(tokenizer.cls_token_id)
        sep_id: int = int(tokenizer.sep_token_id)

        # Pre-tokenize all corpus texts once to avoid repeated slow encode calls.
        all_sequences: list[list[int]] = []
        for rec in corpus:
            ctx_text = " ".join(t["text"] for t in rec.get("context_before", []))
            hunk_text = " ".join(t["text"] for t in rec.get("hunk_tokens", []))
            combined = (ctx_text + " " + hunk_text).strip()
            if not combined:
                continue
            ids: list[int] = tokenizer.encode(combined, add_special_tokens=False)
            if not ids:
                continue
            # Truncate to fit [CLS] + ids + [SEP] within _MAX_LENGTH
            ids = ids[: _MAX_LENGTH - 2]
            all_sequences.append([cls_id] + ids + [sep_id])

        if not all_sequences:
            # Nothing to train on — leave model in its current state.
            model.eval()
            self._model = model
            return

        for _step in range(self._n_steps):
            # Sample a random batch of sequences
            batch_seqs = random.sample(all_sequences, min(self._batch_size, len(all_sequences)))

            # Pad to the length of the longest sequence in the batch
            max_len = max(len(s) for s in batch_seqs)
            pad_id: int = int(tokenizer.pad_token_id or 0)

            input_ids_list: list[list[int]] = []
            labels_list: list[list[int]] = []
            attention_list: list[list[int]] = []

            for seq in batch_seqs:
                masked_seq = list(seq)
                label_seq = [-100] * len(seq)  # -100 = ignore in cross-entropy
                # Mask random positions (skip [CLS] and [SEP])
                for pos in range(1, len(seq) - 1):
                    if random.random() < self._mask_prob:
                        label_seq[pos] = seq[pos]
                        masked_seq[pos] = mask_token_id

                pad_len = max_len - len(seq)
                input_ids_list.append(masked_seq + [pad_id] * pad_len)
                labels_list.append(label_seq + [-100] * pad_len)
                attention_list.append([1] * len(seq) + [0] * pad_len)

            batch_input = torch.tensor(input_ids_list, dtype=torch.long, device=device)
            batch_labels = torch.tensor(labels_list, dtype=torch.long, device=device)
            batch_attn = torch.tensor(attention_list, dtype=torch.long, device=device)

            optimizer.zero_grad()
            outputs: Any = model(
                input_ids=batch_input,
                attention_mask=batch_attn,
                labels=batch_labels,
            )
            loss: torch.Tensor = outputs.loss
            loss.backward()  # type: ignore[no-untyped-call]
            optimizer.step()

        model.eval()
        self._model = model

    def get_model(self) -> Any:
        """Return the fine-tuned model (must call ``train()`` first)."""
        if self._model is None:
            raise RuntimeError("DeltaMlmAdapter.train() has not been called yet.")
        return self._model


# ---------------------------------------------------------------------------
# Delta scorer
# ---------------------------------------------------------------------------


class DeltaMlmScorer:
    """Corpus-adapted MLM scorer using ``surprise_base - surprise_adapted`` as signal.

    A *positive* delta means the base model found the token surprising but the adapted
    model did not — the hallmark of a rare-but-idiomatic pattern.  A *zero or negative*
    delta means the adapter learned nothing new about that token — consistent with a
    paradigm break that does not appear in idiomatic corpus code.

    Parameters
    ----------
    n_steps, lr, batch_size, mask_prob:
        Forwarded to :class:`DeltaMlmAdapter`.
    agg_variant:
        How to aggregate per-token deltas into a single fixture score.
        ``mean`` — arithmetic mean.
        ``min``  — minimum delta (worst-case position).
        ``p05``  — 5th-percentile delta (extreme tail).
    score_batch_size:
        Number of masked sequences per forward pass during scoring (controls memory).
    """

    def __init__(
        self,
        *,
        n_steps: int = 200,
        lr: float = 5e-5,
        batch_size: int = 8,
        mask_prob: float = 0.15,
        agg_variant: Literal["mean", "min", "p05"] = "mean",
        score_batch_size: int = _DEFAULT_SCORE_BATCH,
    ) -> None:
        self._n_steps = n_steps
        self._lr = lr
        self._batch_size = batch_size
        self._mask_prob = mask_prob
        self._agg_variant = agg_variant
        self._score_batch_size = score_batch_size
        self.name = f"delta_mlm_{agg_variant}"

        self._device: torch.device | None = None
        self._tokenizer: Any = None
        self._base_model: Any = None
        self._adapted_model: Any = None
        self._mask_token_id: int = 0

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        """Train LoRA adapter on corpus and load the frozen base model for comparison."""
        device = select_device()
        self._device = device

        # Load tokenizer (shared between base and adapted model)
        self._tokenizer = AutoTokenizer.from_pretrained(_MODEL_ID)  # type: ignore[no-untyped-call]
        self._mask_token_id = int(self._tokenizer.mask_token_id)

        # Train adapted model
        adapter = DeltaMlmAdapter(
            n_steps=self._n_steps,
            lr=self._lr,
            batch_size=self._batch_size,
            mask_prob=self._mask_prob,
        )
        adapter.train(corpus)
        self._adapted_model = adapter.get_model()

        # Load a separate frozen base model (do NOT share with adapted)
        base: Any = AutoModelForMaskedLM.from_pretrained(_MODEL_ID)
        base = base.to(device)
        base.eval()
        for p in base.parameters():
            p.requires_grad = False
        self._base_model = base

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        """Return one delta score per fixture (higher = more idiomatic)."""
        results: list[float] = []
        for rec in fixtures:
            results.append(self._score_one(rec))
        return results

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _score_one(self, rec: dict[str, Any]) -> float:
        """Compute delta score for a single fixture."""
        assert self._tokenizer is not None, "call fit() before score()"
        assert self._device is not None

        ctx_text = " ".join(t["text"] for t in rec.get("context_before", []))
        hunk_text = " ".join(t["text"] for t in rec["hunk_tokens"])

        # Encode context and hunk separately (no special tokens)
        ctx_ids: list[int] = self._tokenizer.encode(ctx_text, add_special_tokens=False)
        hunk_ids: list[int] = self._tokenizer.encode(hunk_text, add_special_tokens=False)

        # Truncate ctx to leave room for [CLS] + hunk_ids + [SEP]
        max_ctx = max(0, _MAX_LENGTH - len(hunk_ids) - 2)
        ctx_ids = ctx_ids[-max_ctx:] if max_ctx > 0 else []

        cls_id: int = int(self._tokenizer.cls_token_id)
        sep_id: int = int(self._tokenizer.sep_token_id)
        all_ids: list[int] = [cls_id] + ctx_ids + hunk_ids + [sep_id]
        hunk_start = 1 + len(ctx_ids)
        hunk_end = hunk_start + len(hunk_ids)

        input_ids = torch.tensor(all_ids)
        attention_mask = torch.ones(len(all_ids), dtype=torch.long)
        hunk_positions = list(range(hunk_start, hunk_end))

        if not hunk_positions:
            return 0.0

        base_lps = self._compute_logprobs(
            self._base_model, input_ids, attention_mask, hunk_positions
        )
        adapted_lps = self._compute_logprobs(
            self._adapted_model, input_ids, attention_mask, hunk_positions
        )

        # delta_i = logprob_adapted_i - logprob_base_i
        # positive → adapter found it less surprising than base → idiomatic signal
        deltas = [a - b for b, a in zip(base_lps, adapted_lps, strict=True)]
        return self._aggregate(deltas)

    def _compute_logprobs(
        self,
        model: Any,
        input_ids: torch.Tensor,
        attention_mask: torch.Tensor,
        hunk_positions: list[int],
    ) -> list[float]:
        """Batch-masked forward passes against *model*; return per-position log-probs."""
        assert self._device is not None
        n = len(hunk_positions)
        repeated = input_ids.unsqueeze(0).expand(n, -1).clone()  # (n, seq_len)
        for row_i, pos in enumerate(hunk_positions):
            repeated[row_i, pos] = self._mask_token_id

        attn_repeated = attention_mask.unsqueeze(0).expand(n, -1)  # (n, seq_len)

        logprobs: list[float] = []
        for batch_start in range(0, n, self._score_batch_size):
            batch_end = min(batch_start + self._score_batch_size, n)
            batch_ids = repeated[batch_start:batch_end].to(self._device)
            batch_attn = attn_repeated[batch_start:batch_end].to(self._device)

            with torch.no_grad():
                logits: torch.Tensor = model(
                    input_ids=batch_ids,
                    attention_mask=batch_attn,
                ).logits  # (batch, seq_len, vocab)

            for sub_i, pos in enumerate(hunk_positions[batch_start:batch_end]):
                true_token_id = int(input_ids[pos].item())
                lp = float(F.log_softmax(logits[sub_i, pos, :], dim=-1)[true_token_id].item())
                logprobs.append(lp)

        return logprobs

    def _aggregate(self, deltas: list[float]) -> float:
        """Aggregate per-token deltas into a single score."""
        if not deltas:
            return 0.0

        if self._agg_variant == "mean":
            return sum(deltas) / len(deltas)

        if self._agg_variant == "min":
            return min(deltas)

        # p05: 5th-percentile of deltas (extreme low end)
        sorted_deltas = sorted(deltas)
        idx = max(0, int(math.floor(0.05 * len(sorted_deltas))))
        return sorted_deltas[idx]


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------

REGISTRY["delta_mlm_mean"] = lambda: DeltaMlmScorer(agg_variant="mean")
REGISTRY["delta_mlm_min"] = lambda: DeltaMlmScorer(agg_variant="min")
REGISTRY["delta_mlm_p05"] = lambda: DeltaMlmScorer(agg_variant="p05")
