# engine/argot/research/signal/phase13/experiments/contrastive_mlm_smoke.py
"""Phase 13: Contrastive-MLM smoke test — corpus fix validation.

Answers: "with the correct corpus, does A diverge from B in the right direction
on a single contrasting pair?"

Usage:
    uv run --package argot-engine python \\
        engine/argot/research/signal/phase13/experiments/contrastive_mlm_smoke.py \\
        --out docs/research/scoring/signal/phase13/experiments/\\
            contrastive_mlm_smoke_2026-04-21.md
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import torch
import torch.nn.functional as F  # noqa: N812

_CODEBERT_MODEL = "microsoft/codebert-base-mlm"
_FASTAPI_DIR = (
    Path(__file__).parent.parent.parent.parent.parent / "acceptance" / "catalog" / "fastapi"
)
_CORPUS_JSONL = _FASTAPI_DIR / "corpus_file_only.jsonl"
_FIXTURES_DIR = _FASTAPI_DIR / "fixtures" / "default"
_ARTIFACTS_DIR = Path(__file__).parent / ".artifacts"

_SMOKE_FIXTURES = [
    ("paradigm_break_flask_routing", "routing", True),
    ("control_router_endpoint", "routing", False),
]

_N_CORPUS = 500


def _load_corpus_texts(jsonl_path: Path, n: int) -> list[str]:
    texts: list[str] = []
    with jsonl_path.open(encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if i >= n:
                break
            try:
                rec: dict[str, Any] = json.loads(line)
            except json.JSONDecodeError as exc:
                print(f"ERROR: Failed to parse line {i} of {jsonl_path}: {exc}", flush=True)
                raise SystemExit(1) from exc
            if "hunk_source" in rec:
                texts.append(rec["hunk_source"])
            elif "hunk_tokens" in rec:
                tokens = rec["hunk_tokens"]
                if not isinstance(tokens, list):
                    print(
                        f"ERROR: line {i}: hunk_tokens is not a list — schema mismatch."
                        " Expected list of {{text: str, ...}}.",
                        flush=True,
                    )
                    raise SystemExit(1)
                texts.append(" ".join(t["text"] for t in tokens))
            else:
                print(
                    f"ERROR: line {i} has neither 'hunk_source' nor 'hunk_tokens'."
                    f" Keys found: {list(rec.keys())}",
                    flush=True,
                )
                raise SystemExit(1)
    if len(texts) == 0:
        print(f"ERROR: {jsonl_path} produced 0 training texts. Aborting.", flush=True)
        raise SystemExit(1)
    return texts


def _get_tokenizer_and_base_model() -> tuple[Any, Any]:
    from transformers import AutoModelForMaskedLM, AutoTokenizer

    tokenizer = AutoTokenizer.from_pretrained(_CODEBERT_MODEL)  # type: ignore[no-untyped-call]
    base_model = AutoModelForMaskedLM.from_pretrained(_CODEBERT_MODEL)
    return tokenizer, base_model


def _build_lora_model(base_model: Any) -> Any:
    import peft

    lora_config = peft.LoraConfig(
        r=8,
        lora_alpha=16,
        target_modules=["query", "value"],
    )
    return peft.get_peft_model(base_model, lora_config)


def _fine_tune_lora(
    lora_model: Any,
    corpus_texts: list[str],
    tokenizer: Any,
    device: torch.device,
    checkpoint_dir: Path,
    *,
    epochs: int = 1,
    lr: float = 1e-4,
    batch_size: int = 4,
) -> Any:
    lora_model = lora_model.to(device)
    lora_model.train()
    optimizer = torch.optim.AdamW(lora_model.parameters(), lr=lr)

    chunks: list[list[int]] = []
    for text in corpus_texts:
        ids: list[int] = tokenizer.encode(text, add_special_tokens=True)
        for start in range(0, len(ids), 512):
            chunk = ids[start : start + 512]
            if chunk:
                chunks.append(chunk)

    print(f"  Training chunks: {len(chunks)}", flush=True)

    pad_id: int = tokenizer.pad_token_id or 0
    special_ids = {
        tokenizer.cls_token_id,
        tokenizer.sep_token_id,
        tokenizer.pad_token_id,
    }

    for epoch in range(epochs):
        total_loss = 0.0
        steps = 0
        for b_start in range(0, len(chunks), batch_size):
            batch_chunks = chunks[b_start : b_start + batch_size]
            max_len = max(len(c) for c in batch_chunks)
            input_ids_list = [c + [pad_id] * (max_len - len(c)) for c in batch_chunks]
            input_tensor = torch.tensor(input_ids_list, device=device)
            labels = input_tensor.clone()

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
            total_loss += float(loss.item())
            steps += 1

        avg_loss = total_loss / max(steps, 1)
        print(f"  Epoch {epoch + 1}/{epochs} — avg MLM loss: {avg_loss:.4f}", flush=True)

    lora_model.save_pretrained(checkpoint_dir)
    return lora_model


def _score_fixture(
    fixture_name: str,
    tokenizer: Any,
    model: Any,
    device: torch.device,
) -> tuple[float, float, list[tuple[int, str, float]]]:
    fixture_path = _FIXTURES_DIR / f"{fixture_name}.py"
    hunk_source = fixture_path.read_text(encoding="utf-8", errors="replace")

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
        return 0.0, 0.0, []

    attn = torch.ones(1, n, device=device)
    per_token: list[tuple[int, float]] = []

    for pos in non_special:
        masked = input_ids.clone()
        masked[pos] = tokenizer.mask_token_id
        batch = masked.unsqueeze(0)
        true_token = int(input_ids[pos].item())

        with torch.no_grad():
            logits_a = model(input_ids=batch, attention_mask=attn).logits
            with model.disable_adapter():
                logits_b = model(input_ids=batch, attention_mask=attn).logits

        log_p_a = F.log_softmax(logits_a[0, pos, :], dim=-1)[true_token].item()
        log_p_b = F.log_softmax(logits_b[0, pos, :], dim=-1)[true_token].item()
        per_token.append((pos, log_p_b - log_p_a))

    scores = [s for _, s in per_token]
    max_score = float(max(scores))
    mean_score = float(sum(scores) / len(scores))

    per_token_sorted = sorted(per_token, key=lambda x: x[1], reverse=True)
    top3: list[tuple[int, str, float]] = []
    for pos, ratio in per_token_sorted[:3]:
        token_text = tokenizer.decode([int(input_ids[pos].item())])
        top3.append((pos, token_text, ratio))

    return max_score, mean_score, top3


def _verdict(delta: float) -> str:
    if delta > 0.5:
        return "PROMISING. Proceed to full run."
    if delta >= -0.2:
        return "WEAK SIGNAL. Signal may emerge with more training or more data. Marginal call."
    return (
        "INVERTED. Same failure mode as v1 despite corpus fix."
        " Root cause is not the corpus. Abandon MLM direction."
    )


def run(out: Path | None = None) -> None:
    from argot.jepa.pretrained_encoder import select_device

    device = select_device()
    print(f"Device: {device}", flush=True)

    print(f"Loading corpus from {_CORPUS_JSONL} ({_N_CORPUS} records)...", flush=True)
    corpus_texts = _load_corpus_texts(_CORPUS_JSONL, _N_CORPUS)
    print(f"Loaded {len(corpus_texts)} training texts.", flush=True)

    print("Loading tokenizer and base model...", flush=True)
    tokenizer, base_model = _get_tokenizer_and_base_model()
    base_model = base_model.to(device)

    print("Building LoRA model...", flush=True)
    lora_model = _build_lora_model(base_model)

    checkpoint_dir = _ARTIFACTS_DIR / "codebert_lora_fastapi_smoke"
    checkpoint_dir.mkdir(parents=True, exist_ok=True)
    print(
        f"Fine-tuning LoRA (1 epoch, lr=1e-4, batch=4, mask=15%) → {checkpoint_dir}",
        flush=True,
    )
    lora_model = _fine_tune_lora(
        lora_model,
        corpus_texts,
        tokenizer,
        device,
        checkpoint_dir,
    )
    lora_model.eval()

    print("\nScoring fixtures...", flush=True)
    results: list[tuple[str, str, bool, float, float, list[tuple[int, str, float]]]] = []
    for fixture_name, category, is_break in _SMOKE_FIXTURES:
        print(f"  Scoring {fixture_name}...", flush=True)
        max_s, mean_s, top3 = _score_fixture(fixture_name, tokenizer, lora_model, device)
        results.append((fixture_name, category, is_break, max_s, mean_s, top3))

    break_res = next(r for r in results if r[2])
    ctrl_res = next(r for r in results if not r[2])
    delta = break_res[3] - ctrl_res[3]
    verdict = _verdict(delta)

    _print_report(results, delta, verdict)

    if out is not None:
        _write_report(out, results, delta, verdict)

    print(f"\nVerdict: {verdict}", flush=True)


def _format_top3(top3: list[tuple[int, str, float]]) -> str:
    return ", ".join(f"pos {pos}: {repr(tok)} ({score:.3f})" for pos, tok, score in top3)


def _print_report(
    results: list[tuple[str, str, bool, float, float, list[tuple[int, str, float]]]],
    delta: float,
    verdict: str,
) -> None:
    header = (
        "\n=== Contrastive-MLM Smoke Test ===\n"
        f"Corpus: {_N_CORPUS} records from corpus_file_only.jsonl\n"
        "Training: 1 epoch LoRA, lr=1e-4\n"
    )
    col_w = 35
    print(header, flush=True)
    print(
        f"{'Fixture':<{col_w}} | {'max':>7} | {'mean':>7} | top-3 tokens (pos: token, score)",
        flush=True,
    )
    print("-" * 100, flush=True)
    for fixture_name, _cat, _is_break, max_s, mean_s, top3 in results:
        top3_str = _format_top3(top3)
        print(f"{fixture_name:<{col_w}} | {max_s:>7.3f} | {mean_s:>7.3f} | {top3_str}", flush=True)
    print(f"\nDelta (break − control max): {delta:.3f}", flush=True)


def _write_report(
    out: Path,
    results: list[tuple[str, str, bool, float, float, list[tuple[int, str, float]]]],
    delta: float,
    verdict: str,
) -> None:
    break_res = next(r for r in results if r[2])
    ctrl_res = next(r for r in results if not r[2])

    lines = [
        "# Phase 13 — Contrastive-MLM Smoke Test (2026-04-21)\n",
        "",
        "## Setup\n",
        "",
        f"- Corpus: {_N_CORPUS} records from `corpus_file_only.jsonl`",
        "- Training: 1 epoch LoRA, lr=1e-4, batch=4, mask=15%",
        "- Base model: `microsoft/codebert-base-mlm`",
        "- LoRA config: rank=8, alpha=16, target_modules=[query, value]",
        "- Fixtures: routing category only (break vs control, topic held constant)",
        "",
        "## Results\n",
        "",
        "| Fixture | max | mean | top-3 tokens |",
        "|---|---|---|---|",
    ]
    for fixture_name, _cat, _is_break, max_s, mean_s, top3 in results:
        top3_str = " / ".join(f"pos {pos}: `{tok}` ({score:.3f})" for pos, tok, score in top3)
        lines.append(f"| {fixture_name} | {max_s:.3f} | {mean_s:.3f} | {top3_str} |")

    lines += [
        "",
        f"**Delta (break − control max): {delta:.3f}**",
        "",
        "## Verdict\n",
        "",
        f"**{verdict}**",
        "",
        "## Notes\n",
        "",
        f"- Break fixture: `{break_res[0]}` — max={break_res[3]:.3f}, mean={break_res[4]:.3f}",
        f"- Control fixture: `{ctrl_res[0]}` — max={ctrl_res[3]:.3f}, mean={ctrl_res[4]:.3f}",
        "- v1 (broken corpus) AUC: 0.4645",
        "- BPE-tfidf baseline: AUC 1.0000",
        "",
    ]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Report written to {out}", flush=True)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Contrastive-MLM smoke test")
    parser.add_argument("--out", help="Path for markdown report (optional)")
    args = parser.parse_args(argv)
    out = Path(args.out) if args.out else None
    run(out=out)


if __name__ == "__main__":
    main()
