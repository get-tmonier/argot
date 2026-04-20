from __future__ import annotations

from typing import Any

import torch
import torch.nn.functional as F  # noqa: N812
from transformers import AutoModelForCausalLM, AutoTokenizer

from argot.research.signal.base import REGISTRY

_MODEL_ID = "Salesforce/codegen-350M-multi"


class LmPerplexityScorer:
    name = "lm_perplexity"

    def __init__(self) -> None:
        if torch.backends.mps.is_available():
            device_str = "mps"
        elif torch.cuda.is_available():
            device_str = "cuda"
        else:
            device_str = "cpu"
        self._device = torch.device(device_str)
        self._tokenizer: Any = AutoTokenizer.from_pretrained(_MODEL_ID)  # type: ignore[no-untyped-call]
        _raw: Any = AutoModelForCausalLM.from_pretrained(_MODEL_ID)
        self._model: Any = _raw.to(self._device)
        self._model.eval()

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        pass

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        scores: list[float] = []
        max_len: int = self._model.config.max_position_embeddings
        for rec in fixtures:
            ctx_text = " ".join(t["text"] for t in rec.get("ctx_before_tokens", []))
            hunk_text = " ".join(t["text"] for t in rec["hunk_tokens"])
            ctx_ids: list[int] = self._tokenizer.encode(ctx_text, add_special_tokens=False)
            hunk_ids: list[int] = self._tokenizer.encode(hunk_text, add_special_tokens=False)
            max_ctx = max(0, max_len - len(hunk_ids) - 2)
            ctx_ids = ctx_ids[-max_ctx:] if max_ctx > 0 else []
            input_ids = torch.tensor([ctx_ids + hunk_ids], device=self._device)
            hunk_start = len(ctx_ids)
            with torch.no_grad():
                logits = self._model(input_ids).logits  # (1, seq, vocab)
            if hunk_start > 0:
                shift_logits = logits[0, hunk_start - 1 : -1, :]
                targets = input_ids[0, hunk_start:]
            else:
                shift_logits = logits[0, :-1, :]
                targets = input_ids[0, 1:]
            nll = F.cross_entropy(shift_logits, targets, reduction="none")
            scores.append(float(nll.mean()))
        return scores


REGISTRY["lm_perplexity"] = LmPerplexityScorer
