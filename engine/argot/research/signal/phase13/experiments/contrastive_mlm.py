# engine/argot/research/signal/phase13/experiments/contrastive_mlm.py
"""Phase 13: Contrastive-MLM scorer using LoRA fine-tuned CodeBERT."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import torch
import torch.nn.functional as F  # noqa: N812

_CODEBERT_MODEL = "microsoft/codebert-base-mlm"
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
_ARTIFACTS_DIR = Path(__file__).parent / ".artifacts"


def get_tokenizer_and_base_model() -> tuple[Any, Any]:
    from transformers import AutoModelForMaskedLM, AutoTokenizer

    tokenizer = AutoTokenizer.from_pretrained(_CODEBERT_MODEL)  # type: ignore[no-untyped-call]
    base_model = AutoModelForMaskedLM.from_pretrained(_CODEBERT_MODEL)
    return tokenizer, base_model


def build_lora_model(base_model: Any) -> Any:
    import peft

    lora_config = peft.LoraConfig(
        r=8,
        lora_alpha=16,
        target_modules=["query", "value"],
    )
    return peft.get_peft_model(base_model, lora_config)


def fine_tune_lora(
    lora_model: Any,
    corpus_paths: list[Path],
    tokenizer: Any,
    device: torch.device,
    *,
    epochs: int = 2,
    lr: float = 1e-4,
    batch_size: int = 8,
    checkpoint_dir: Path | None = None,
) -> Any:
    lora_model = lora_model.to(device)
    lora_model.train()
    optimizer = torch.optim.AdamW(lora_model.parameters(), lr=lr)

    chunks: list[list[int]] = []
    for path in corpus_paths:
        source = path.read_text(encoding="utf-8", errors="replace")
        ids: list[int] = tokenizer.encode(source, add_special_tokens=True)
        for start in range(0, len(ids), 512):
            chunk = ids[start : start + 512]
            if chunk:
                chunks.append(chunk)

    for _ in range(epochs):
        for b_start in range(0, len(chunks), batch_size):
            batch_chunks = chunks[b_start : b_start + batch_size]
            max_len = max(len(c) for c in batch_chunks)
            pad_id: int = tokenizer.pad_token_id or 0
            input_ids_list = [c + [pad_id] * (max_len - len(c)) for c in batch_chunks]
            input_tensor = torch.tensor(input_ids_list, device=device)
            labels = input_tensor.clone()

            # 15% random masking
            special_ids = {
                tokenizer.cls_token_id,
                tokenizer.sep_token_id,
                tokenizer.pad_token_id,
            }
            mask_candidates = torch.rand(input_tensor.shape, device=device) < 0.15
            for sp_id in special_ids:
                if sp_id is not None:
                    mask_candidates &= input_tensor != sp_id

            masked_input = input_tensor.clone()
            masked_input[mask_candidates] = tokenizer.mask_token_id
            labels[~mask_candidates] = -100

            attention_mask = (input_tensor != pad_id).long()
            outputs = lora_model(
                input_ids=masked_input,
                attention_mask=attention_mask,
                labels=labels,
            )
            loss = outputs.loss
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

    if checkpoint_dir is not None:
        lora_model.save_pretrained(checkpoint_dir)

    return lora_model


def score_hunk_mlm(
    hunk_source: str,
    tokenizer: Any,
    model_a: Any,
    model_b: Any,
    device: torch.device,
) -> tuple[float, float, float]:
    encoding = tokenizer(
        hunk_source,
        return_tensors="pt",
        truncation=False,
        add_special_tokens=True,
    )
    input_ids: torch.Tensor = encoding["input_ids"][0]
    seq_len = input_ids.shape[0]

    special_ids = {
        tokenizer.cls_token_id,
        tokenizer.sep_token_id,
        tokenizer.pad_token_id,
    }

    def _score_window(window_ids: torch.Tensor) -> list[float]:
        n = window_ids.shape[0]
        non_special = [i for i in range(n) if window_ids[i].item() not in special_ids]
        if not non_special:
            return []

        per_token_scores: list[float] = []
        per_token_batch = 32

        for b_start in range(0, len(non_special), per_token_batch):
            batch_positions = non_special[b_start : b_start + per_token_batch]
            batch_input = window_ids.unsqueeze(0).repeat(len(batch_positions), 1).to(device)
            for j, pos in enumerate(batch_positions):
                batch_input[j, pos] = tokenizer.mask_token_id

            attn_mask = torch.ones_like(batch_input)

            with torch.no_grad():
                logits_a = model_a(input_ids=batch_input, attention_mask=attn_mask).logits
                logits_b = model_b(input_ids=batch_input, attention_mask=attn_mask).logits

            for j, pos in enumerate(batch_positions):
                true_token = int(window_ids[pos].item())
                log_p_a = F.log_softmax(logits_a[j, pos, :], dim=-1)[true_token].item()
                log_p_b = F.log_softmax(logits_b[j, pos, :], dim=-1)[true_token].item()
                per_token_scores.append(log_p_b - log_p_a)

        return per_token_scores

    if seq_len <= 512:
        all_scores = _score_window(input_ids)
    else:
        window_size = 480
        step = 32
        window_scores: list[list[float]] = []
        for start in range(0, seq_len - window_size + 1, step):
            window = input_ids[start : start + window_size]
            window_scores.append(_score_window(window))
        all_scores = []
        for ws in window_scores:
            all_scores.extend(ws)

    if not all_scores:
        return 0.0, 0.0, 0.0

    max_score = float(max(all_scores))
    mean_score = float(sum(all_scores) / len(all_scores))
    sorted_scores = sorted(all_scores, reverse=True)
    top5 = sorted_scores[: min(5, len(sorted_scores))]
    top5_mean = float(sum(top5) / len(top5))
    return max_score, mean_score, top5_mean


def score_hunk_mlm_from_record(
    record: dict[str, Any],
    tokenizer: Any,
    model_a: Any,
    model_b: Any,
    device: torch.device,
) -> tuple[float, float, float]:
    fixture_path = Path(record["_fixture_path"])
    hunk_start = record.get("hunk_start_line", 0)
    hunk_end = record.get("hunk_end_line", 0)
    source = fixture_path.read_text(encoding="utf-8", errors="replace")
    lines = source.splitlines(keepends=True)
    hunk_source = "".join(lines[hunk_start:hunk_end]) if hunk_end > hunk_start else source
    if not hunk_source.strip():
        hunk_source = source
    return score_hunk_mlm(hunk_source, tokenizer, model_a, model_b, device)
