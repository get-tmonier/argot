"""MLM surprise scorer — three variants using UniXCoder masked-language model.

Zero-shot scorer: no training required, no fit() side-effects.

Three registered variants:
  mlm_surprise_mean  — mean negative log-prob across hunk tokens (higher = more surprising)
  mlm_surprise_min   — minimum log-prob token = maximum single-token surprise — the single
                        most OOV token in the hunk
  mlm_surprise_p05   — 5th-percentile negative log-prob (tail-extreme surprise)
"""

from __future__ import annotations

import math
from typing import Any, Literal

import torch
import torch.nn.functional as F  # noqa: N812
from transformers import AutoModelForMaskedLM, AutoTokenizer

from argot.jepa.pretrained_encoder import select_device
from argot.research.signal.base import REGISTRY

_MODEL_ID = "microsoft/unixcoder-base"
_MAX_LENGTH = 512
_DEFAULT_BATCH_SIZE = 32


class MlmSurpriseScorer:
    """Zero-shot MLM surprise scorer backed by UniXCoder.

    Parameters
    ----------
    variant:
        Aggregation variant over per-token log-probs.
        ``mean`` — arithmetic mean of negative log-probs.
        ``min``  — minimum log-prob token = maximum single-token surprise
                   (the single most OOV token in the hunk).
        ``p05``  — 5th-percentile negative log-prob (extreme tail).
    batch_size:
        Number of masked sequences processed per forward pass.
        Reduces peak GPU memory for long hunks at the cost of more passes.
    """

    def __init__(
        self,
        *,
        variant: Literal["mean", "min", "p05"],
        batch_size: int = _DEFAULT_BATCH_SIZE,
    ) -> None:
        self._variant = variant
        self._batch_size = batch_size
        self.name = f"mlm_surprise_{variant}"

        device = select_device()
        self._device = device
        # Load tokenizer + MLM head (separate from PretrainedEncoder which uses AutoModel)
        self._tokenizer: Any = AutoTokenizer.from_pretrained(_MODEL_ID)  # type: ignore[no-untyped-call]
        _raw: Any = AutoModelForMaskedLM.from_pretrained(_MODEL_ID)
        self._model: Any = _raw.to(device)
        self._model.eval()
        for p in self._model.parameters():
            p.requires_grad = False

        self._mask_token_id: int = int(self._tokenizer.mask_token_id)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        """No-op — this is a zero-shot scorer."""

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        """Return one anomaly score per fixture (higher = more anomalous)."""
        results: list[float] = []
        for rec in fixtures:
            score = self._score_one(rec)
            results.append(score)
        return results

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _score_one(self, rec: dict[str, Any]) -> float:
        """Compute the surprise score for a single fixture record."""
        ctx_text = " ".join(t["text"] for t in rec.get("context_before", []))
        hunk_text = " ".join(t["text"] for t in rec["hunk_tokens"])

        # Encode context and hunk separately (no special tokens) to avoid BPE
        # non-compositionality — joint encoding changes boundary tokens.
        ctx_ids: list[int] = self._tokenizer.encode(ctx_text, add_special_tokens=False)
        hunk_ids: list[int] = self._tokenizer.encode(hunk_text, add_special_tokens=False)

        # Truncate ctx to fit within max_length (leave room for [CLS] + [SEP] + hunk_ids)
        max_ctx = max(0, _MAX_LENGTH - len(hunk_ids) - 2)
        ctx_ids = ctx_ids[-max_ctx:] if max_ctx > 0 else []

        # Build full token sequence with special tokens
        cls_id: int = int(self._tokenizer.cls_token_id)
        sep_id: int = int(self._tokenizer.sep_token_id)
        all_ids: list[int] = [cls_id] + ctx_ids + hunk_ids + [sep_id]
        hunk_start_pos = 1 + len(ctx_ids)
        hunk_end_pos = hunk_start_pos + len(hunk_ids)

        input_ids: torch.Tensor = torch.tensor(all_ids)
        attention_mask: torch.Tensor = torch.ones(len(all_ids), dtype=torch.long)

        hunk_positions = list(range(hunk_start_pos, hunk_end_pos))

        if not hunk_positions:
            # Edge case: nothing to score
            return 0.0

        logprobs = self._compute_logprobs(input_ids, attention_mask, hunk_positions)
        return self._aggregate(logprobs)

    def _compute_logprobs(
        self,
        input_ids: torch.Tensor,
        attention_mask: torch.Tensor,
        hunk_positions: list[int],
    ) -> list[float]:
        """Batch-masked forward passes; return per-position log-probs (negative = surprising)."""
        n = len(hunk_positions)
        # Build all n masked sequences up-front as a 2D tensor
        # shape: (n, seq_len)
        repeated = input_ids.unsqueeze(0).expand(n, -1).clone()  # (n, seq_len)
        for row_i, pos in enumerate(hunk_positions):
            repeated[row_i, pos] = self._mask_token_id

        attn_repeated = attention_mask.unsqueeze(0).expand(n, -1)  # (n, seq_len)

        logprobs: list[float] = []
        for batch_start in range(0, n, self._batch_size):
            batch_end = min(batch_start + self._batch_size, n)
            batch_ids = repeated[batch_start:batch_end].to(self._device)
            batch_attn = attn_repeated[batch_start:batch_end].to(self._device)

            with torch.no_grad():
                logits: torch.Tensor = self._model(
                    input_ids=batch_ids,
                    attention_mask=batch_attn,
                ).logits  # (batch, seq_len, vocab)

            for sub_i, pos in enumerate(hunk_positions[batch_start:batch_end]):
                true_token_id = int(input_ids[pos].item())
                lp = float(F.log_softmax(logits[sub_i, pos, :], dim=-1)[true_token_id].item())
                logprobs.append(lp)

        return logprobs

    def _aggregate(self, logprobs: list[float]) -> float:
        """Aggregate log-probs (all negative) into a single anomaly score (higher = worse)."""
        if not logprobs:
            return 0.0

        # negate: more negative log-prob → higher anomaly score
        neg_lps = [-lp for lp in logprobs]

        if self._variant == "mean":
            return sum(neg_lps) / len(neg_lps)

        if self._variant == "min":
            # "min" of negated = max surprise per token
            return max(neg_lps)

        # p05: 5th percentile of log-probs (most surprisingly low)
        # = negate the 5th-percentile of logprobs
        sorted_lps = sorted(logprobs)
        idx = max(0, int(math.floor(0.05 * len(sorted_lps))))
        return -sorted_lps[idx]


REGISTRY["mlm_surprise_mean"] = lambda: MlmSurpriseScorer(variant="mean")
REGISTRY["mlm_surprise_min"] = lambda: MlmSurpriseScorer(variant="min")
REGISTRY["mlm_surprise_p05"] = lambda: MlmSurpriseScorer(variant="p05")
