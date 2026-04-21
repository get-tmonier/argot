# engine/argot/research/signal/phase13/experiments/contrastive_mlm.py
"""Phase 13: Contrastive-MLM scorer using LoRA fine-tuned CodeBERT.

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_mlm.py \\
        --out docs/research/scoring/signal/phase13/experiments/\\
            contrastive_mlm_fastapi_2026-04-21.md
"""

from __future__ import annotations

import argparse
import random
from collections import defaultdict
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
    epochs: int = 1,
    lr: float = 1e-4,
    batch_size: int = 1,
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
    model: Any,  # single LoRA model (adapters on = A, disable_adapter() = B)
    device: torch.device,
    *,
    k_passes: int = 5,
    mask_prob: float = 0.15,
) -> tuple[float, float, float]:
    encoding = tokenizer(
        hunk_source,
        return_tensors="pt",
        truncation=True,
        max_length=512,
        add_special_tokens=True,
    )
    input_ids: torch.Tensor = encoding["input_ids"][0].to(device)
    n = input_ids.shape[0]

    special_ids = {
        tokenizer.cls_token_id,
        tokenizer.sep_token_id,
        tokenizer.pad_token_id,
    }
    non_special = [i for i in range(n) if input_ids[i].item() not in special_ids]
    if not non_special:
        return 0.0, 0.0, 0.0

    all_scores: list[float] = []
    for _ in range(k_passes):
        n_mask = max(1, int(len(non_special) * mask_prob))
        mask_positions = random.sample(non_special, n_mask)

        masked_input = input_ids.clone()
        for pos in mask_positions:
            masked_input[pos] = tokenizer.mask_token_id
        batch = masked_input.unsqueeze(0)
        attn = torch.ones_like(batch)

        with torch.no_grad():
            logits_a = model(input_ids=batch, attention_mask=attn).logits
            with model.disable_adapter():
                logits_b = model(input_ids=batch, attention_mask=attn).logits

        for pos in mask_positions:
            true_token = int(input_ids[pos].item())
            log_p_a = F.log_softmax(logits_a[0, pos, :], dim=-1)[true_token].item()
            log_p_b = F.log_softmax(logits_b[0, pos, :], dim=-1)[true_token].item()
            all_scores.append(log_p_b - log_p_a)

    if not all_scores:
        return 0.0, 0.0, 0.0

    max_score = float(max(all_scores))
    mean_score = float(sum(all_scores) / len(all_scores))
    sorted_scores = sorted(all_scores, reverse=True)
    top5_mean = float(sum(sorted_scores[: min(5, len(sorted_scores))]) / min(5, len(sorted_scores)))
    return max_score, mean_score, top5_mean


def score_hunk_mlm_from_record(
    record: dict[str, Any],
    tokenizer: Any,
    model: Any,
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
    return score_hunk_mlm(hunk_source, tokenizer, model, device)


# ---------------------------------------------------------------------------
# Runner helpers
# ---------------------------------------------------------------------------


def _build_corpus_paths_fastapi(fastapi_dir: Path) -> list[Path]:
    return sorted((fastapi_dir / "fixtures" / "default").glob("control_*.py"))


def _per_category_auc(
    scores: list[tuple[float, float, float]],
    is_break: list[bool],
    categories: list[str],
    ctrl_scores: list[tuple[float, float, float]],
) -> dict[str, tuple[int, float]]:
    from argot.research.signal.bootstrap import auc_from_scores

    ctrl_max = [s[0] for s in ctrl_scores]
    cat_breaks: dict[str, list[float]] = defaultdict(list)
    for s, b, cat in zip(scores, is_break, categories, strict=False):
        if b:
            cat_breaks[cat].append(s[0])
    return {
        cat: (len(cat_s), auc_from_scores(cat_s, ctrl_max))
        for cat, cat_s in sorted(cat_breaks.items())
    }


def _saturation_check(break_scores: list[tuple[float, float, float]]) -> str:
    max_scores = [s[0] for s in break_scores]
    unique = len(set(max_scores))
    total = len(max_scores)
    if unique == 1:
        return (
            f"**Max-token saturation present**: all {total} breaks share identical"
            f" score {max_scores[0]:.4f}."
        )
    return f"Max-token saturation resolved: {unique}/{total} unique break scores."


def _top3_argmax_tokens(
    record: dict[str, Any],
    tokenizer: Any,
    model: Any,
    device: torch.device,
) -> list[tuple[int, str, float]]:
    fixture_path = Path(record["_fixture_path"])
    hunk_start = record.get("hunk_start_line", 0)
    hunk_end = record.get("hunk_end_line", 0)
    source = fixture_path.read_text(encoding="utf-8", errors="replace")
    lines = source.splitlines(keepends=True)
    hunk_source = "".join(lines[hunk_start:hunk_end]) if hunk_end > hunk_start else source
    if not hunk_source.strip():
        hunk_source = source

    encoding = tokenizer(
        hunk_source,
        return_tensors="pt",
        truncation=True,
        max_length=512,
        add_special_tokens=True,
    )
    input_ids: torch.Tensor = encoding["input_ids"][0]
    special_ids = {
        tokenizer.cls_token_id,
        tokenizer.sep_token_id,
        tokenizer.pad_token_id,
    }
    non_special = [i for i in range(input_ids.shape[0]) if input_ids[i].item() not in special_ids]
    if not non_special:
        return []

    per_token_ratios: list[tuple[int, float]] = []
    batch_size = 32
    for b_start in range(0, len(non_special), batch_size):
        batch_positions = non_special[b_start : b_start + batch_size]
        batch_input = input_ids.unsqueeze(0).repeat(len(batch_positions), 1).to(device)
        for j, pos in enumerate(batch_positions):
            batch_input[j, pos] = tokenizer.mask_token_id
        attn_mask = torch.ones_like(batch_input)
        with torch.no_grad():
            logits_a = model(input_ids=batch_input, attention_mask=attn_mask).logits
            with model.disable_adapter():
                logits_b = model(input_ids=batch_input, attention_mask=attn_mask).logits
        for j, pos in enumerate(batch_positions):
            true_token = int(input_ids[pos].item())
            log_p_a = F.log_softmax(logits_a[j, pos, :], dim=-1)[true_token].item()
            log_p_b = F.log_softmax(logits_b[j, pos, :], dim=-1)[true_token].item()
            per_token_ratios.append((pos, log_p_b - log_p_a))

    per_token_ratios.sort(key=lambda x: x[1], reverse=True)
    top3 = per_token_ratios[:3]
    result: list[tuple[int, str, float]] = []
    for pos, ratio in top3:
        token_text = tokenizer.decode([int(input_ids[pos].item())])
        result.append((pos, token_text, ratio))
    return result


def _interpretation(
    auc: float,
    saturation_note: str,
    top3_info: list[tuple[str, list[tuple[int, str, float]]]],
) -> str:
    if auc >= 0.85:
        band = f"AUC {auc:.4f} ≥ 0.85: FastAPI gate passed. Proceed to click."
    else:
        band = (
            f"AUC {auc:.4f} < 0.85: FastAPI gate FAILED — fine-tuning or scoring is broken."
            " Stop and diagnose."
        )

    token_lines: list[str] = []
    if top3_info:
        token_lines.append("\n### Top-3 argmax tokens per break fixture\n")
        for fixture_name, top3 in top3_info:
            if top3:
                tokens_str = ", ".join(
                    f"`{tok}` (pos {pos}, ratio {ratio:.3f})" for pos, tok, ratio in top3
                )
                token_lines.append(f"- **{fixture_name}**: {tokens_str}")

    top3_section = "\n".join(token_lines) if token_lines else ""
    return f"{band}\n\n{saturation_note}{top3_section}"


def _write_report(
    out: Path,
    overall_auc: float,
    per_cat: dict[str, tuple[int, float]],
    names: list[str],
    scores_tuples: list[tuple[float, float, float]],
    is_break: list[bool],
    categories: list[str],
    top3_by_fixture: list[tuple[str, list[tuple[int, str, float]]]],
) -> None:
    break_scores = [s for s, b in zip(scores_tuples, is_break, strict=False) if b]
    saturation_note = _saturation_check(break_scores)
    interpretation = _interpretation(overall_auc, saturation_note, top3_by_fixture)
    out.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Phase 13 — Contrastive-MLM Experiment (FastAPI, 2026-04-21)\n",
        "",
        "## Summary\n",
        "",
        "| scorer | corpus | approach | AUC |",
        "|---|---|---|---|",
        "| contrastive_tfidf (word) | FastAPI | marginal token freq | 0.9847 |",
        "| bpe_contrastive_tfidf | FastAPI | marginal BPE freq | 1.0000 |",
        "| contrastive_jepa | FastAPI | sentence embedding | 0.5532 |",
        f"| **contrastive_mlm** | **FastAPI** | **conditional MLM log-ratio**"
        f" | **{overall_auc:.4f}** |",
        "",
        "## Per-Category AUC\n",
        "",
        "*(break category vs all controls)*\n",
        "",
        "| category | n_breaks | AUC |",
        "|---|---|---|",
    ]
    for cat, (n, cat_auc) in per_cat.items():
        lines.append(f"| {cat} | {n} | {cat_auc:.4f} |")
    lines += [
        "",
        "## Fixture Scores\n",
        "",
        "| fixture | category | is_break | max_score | mean_score | top5_mean |",
        "|---|---|---|---|---|---|",
    ]
    for name, cat, b, s in zip(names, categories, is_break, scores_tuples, strict=False):
        lines.append(f"| {name} | {cat} | {b} | {s[0]:.4f} | {s[1]:.4f} | {s[2]:.4f} |")
    lines += [
        "",
        "## Held-out vocabulary\n",
        "",
        "N/A: FastAPI has no held-out vocabulary split\n",
        "",
        "## Interpretation\n",
        "",
        interpretation,
        "",
    ]
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def run(fastapi_dir: Path = _FASTAPI_DIR, out: Path | None = None) -> float:
    from argot.acceptance.runner import fixture_to_record, load_manifest
    from argot.jepa.pretrained_encoder import select_device
    from argot.research.signal.bootstrap import auc_from_scores

    device = select_device()

    specs = load_manifest(fastapi_dir)
    records = [fixture_to_record(fastapi_dir, spec) for spec in specs]

    is_break = [spec.is_break for spec in specs]
    categories = [spec.category for spec in specs]
    names = [spec.name for spec in specs]

    n_breaks = sum(is_break)
    n_ctrls = sum(not b for b in is_break)
    assert n_breaks == 31, f"Expected 31 breaks, got {n_breaks}"
    assert n_ctrls == 20, f"Expected 20 controls, got {n_ctrls}"

    print("Loading tokenizer and base model...", flush=True)
    tokenizer, base_model = get_tokenizer_and_base_model()
    base_model = base_model.to(device)
    base_model.eval()

    print("Building LoRA model...", flush=True)
    lora_model = build_lora_model(base_model)

    corpus_paths = _build_corpus_paths_fastapi(fastapi_dir)
    print(f"Fine-tuning LoRA on {len(corpus_paths)} FastAPI corpus files...", flush=True)
    checkpoint_dir = _ARTIFACTS_DIR / "codebert_lora_fastapi"
    lora_model = fine_tune_lora(
        lora_model,
        corpus_paths,
        tokenizer,
        device,
        checkpoint_dir=checkpoint_dir,
    )

    lora_model.eval()
    print("Scoring fixtures...", flush=True)
    scores_tuples: list[tuple[float, float, float]] = []
    for i, record in enumerate(records):
        label = "break" if is_break[i] else "ctrl"
        print(f"  [{i + 1:02d}/{len(records)}] {names[i]} ({label})", flush=True)
        s = score_hunk_mlm_from_record(record, tokenizer, lora_model, device)
        scores_tuples.append(s)

    break_scores_tuples = [s for s, b in zip(scores_tuples, is_break, strict=False) if b]
    ctrl_scores_tuples = [s for s, b in zip(scores_tuples, is_break, strict=False) if not b]

    break_max = [s[0] for s in break_scores_tuples]
    ctrl_max = [s[0] for s in ctrl_scores_tuples]
    overall_auc = auc_from_scores(break_max, ctrl_max)

    per_cat = _per_category_auc(scores_tuples, is_break, categories, ctrl_scores_tuples)

    print(f"contrastive_mlm (this run): {overall_auc:.4f}", flush=True)

    if overall_auc >= 0.85:
        print("FastAPI gate passed (≥ 0.85). Proceed to click.", flush=True)
    else:
        print(
            f"WARNING: FastAPI gate FAILED ({overall_auc:.4f} < 0.85). Stop and diagnose.",
            flush=True,
        )

    top3_by_fixture: list[tuple[str, list[tuple[int, str, float]]]] = []
    if out is not None:
        print("Computing top-3 argmax tokens per break fixture for report...", flush=True)
        for i, record in enumerate(records):
            if is_break[i]:
                top3 = _top3_argmax_tokens(record, tokenizer, lora_model, device)
                top3_by_fixture.append((names[i], top3))
        _write_report(
            out,
            overall_auc,
            per_cat,
            names,
            scores_tuples,
            is_break,
            categories,
            top3_by_fixture,
        )

    return overall_auc


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive-MLM experiment — FastAPI")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
