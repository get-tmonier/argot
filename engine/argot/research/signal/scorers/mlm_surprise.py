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


class MlmSurpriseScorer:
    """Zero-shot MLM surprise scorer backed by UniXCoder."""

    def __init__(
        self,
        *,
        variant: Literal["mean", "min", "p05"],
    ) -> None:
        self._variant = variant
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
        return [self._score_one(rec) for rec in fixtures]

    def score_all_variants(self, fixtures: list[dict[str, Any]]) -> dict[str, list[float]]:
        """Compute logprobs once and return scores for all three variants.

        Avoids loading the model three times when multiple mlm_surprise_* scorers
        are requested in the same bakeoff run.
        """
        n = len(fixtures)
        all_logprobs: list[list[float]] = []
        for i, rec in enumerate(fixtures):
            lps = self._compute_fixture_logprobs(rec)
            all_logprobs.append(lps)
            print(f"  MLM scored fixture {i + 1}/{n} ({len(lps)} hunk tokens)", flush=True)
            if self._device.type == "mps":
                torch.mps.empty_cache()
        return {
            "mlm_surprise_mean": [self._aggregate_variant(lps, "mean") for lps in all_logprobs],
            "mlm_surprise_min": [self._aggregate_variant(lps, "min") for lps in all_logprobs],
            "mlm_surprise_p05": [self._aggregate_variant(lps, "p05") for lps in all_logprobs],
        }

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _score_one(self, rec: dict[str, Any]) -> float:
        logprobs = self._compute_fixture_logprobs(rec)
        return self._aggregate(logprobs)

    def _compute_fixture_logprobs(self, rec: dict[str, Any]) -> list[float]:
        """Return per-hunk-token log-probs for one fixture record.

        Masks all hunk tokens simultaneously in a single forward pass
        (joint masking) rather than one-at-a-time. This is O(1) passes per
        fixture instead of O(n_tokens / batch_size), and avoids the 3 GB
        logits tensor that the batched approach accumulates on MPS.

        Semantics: P(token_i | context_before, all_hunk_tokens_masked).
        This asks "how expected is this hunk given only context?" — appropriate
        for anomaly detection where hunk-internal correlations are noise.
        """
        ctx_text = " ".join(t["text"] for t in rec.get("context_before", []))
        hunk_text = " ".join(t["text"] for t in rec["hunk_tokens"])

        ctx_ids: list[int] = self._tokenizer.encode(ctx_text, add_special_tokens=False)
        hunk_ids: list[int] = self._tokenizer.encode(hunk_text, add_special_tokens=False)

        max_ctx = max(0, _MAX_LENGTH - len(hunk_ids) - 2)
        ctx_ids = ctx_ids[-max_ctx:] if max_ctx > 0 else []

        cls_id: int = int(self._tokenizer.cls_token_id)
        sep_id: int = int(self._tokenizer.sep_token_id)
        all_ids: list[int] = [cls_id] + ctx_ids + hunk_ids + [sep_id]
        hunk_start_pos = 1 + len(ctx_ids)
        hunk_end_pos = hunk_start_pos + len(hunk_ids)
        hunk_positions = list(range(hunk_start_pos, hunk_end_pos))

        if not hunk_positions:
            return []

        # Mask all hunk positions at once — single forward pass
        masked_ids = torch.tensor(all_ids)
        for pos in hunk_positions:
            masked_ids[pos] = self._mask_token_id

        batch_ids = masked_ids.unsqueeze(0).to(self._device)  # (1, seq_len)
        batch_attn = torch.ones(1, len(all_ids), dtype=torch.long).to(self._device)

        with torch.no_grad():
            logits: torch.Tensor = self._model(
                input_ids=batch_ids,
                attention_mask=batch_attn,
            ).logits  # (1, seq_len, vocab)

        logprobs: list[float] = []
        for pos in hunk_positions:
            true_token_id = int(all_ids[pos])
            lp = float(F.log_softmax(logits[0, pos, :], dim=-1)[true_token_id].item())
            logprobs.append(lp)

        del logits, batch_ids, batch_attn
        return logprobs

    def _aggregate(self, logprobs: list[float]) -> float:
        return self._aggregate_variant(logprobs, self._variant)

    def _aggregate_variant(self, logprobs: list[float], variant: str) -> float:
        """Aggregate log-probs into a single anomaly score for the given variant."""
        if not logprobs:
            return 0.0
        neg_lps = [-lp for lp in logprobs]
        if variant == "mean":
            return sum(neg_lps) / len(neg_lps)
        if variant == "min":
            return max(neg_lps)
        # p05
        sorted_lps = sorted(logprobs)
        idx = max(0, int(math.floor(0.05 * len(sorted_lps))))
        return -sorted_lps[idx]


REGISTRY["mlm_surprise_mean"] = lambda: MlmSurpriseScorer(variant="mean")
REGISTRY["mlm_surprise_min"] = lambda: MlmSurpriseScorer(variant="min")
REGISTRY["mlm_surprise_p05"] = lambda: MlmSurpriseScorer(variant="p05")
