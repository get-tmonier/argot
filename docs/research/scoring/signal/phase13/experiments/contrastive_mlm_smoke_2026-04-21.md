# Phase 13 — Contrastive-MLM Smoke Test (2026-04-21)

## Setup

- Corpus: 500 records from `corpus_file_only.jsonl`
- Training: 1 epoch LoRA, lr=1e-4, batch=4, mask=15%
- Base model: `microsoft/codebert-base-mlm`
- LoRA config: rank=8, alpha=16, target_modules=[query, value]
- Fixtures: routing category only (break vs control, topic held constant)

## Results

*To be filled after running the smoke script.*

Run with:
```
uv run --package argot-engine python \
    engine/argot/research/signal/phase13/experiments/contrastive_mlm_smoke.py \
    --out docs/research/scoring/signal/phase13/experiments/contrastive_mlm_smoke_2026-04-21.md
```

## Verdict

*Pending run.*

## Notes

- v1 (broken corpus) AUC: 0.4645
- BPE-tfidf baseline: AUC 1.0000
- v1 corpus bug: trained on 20 evaluation control fixtures (data leakage)
- v2 fix: trains on 500 records from `corpus_file_only.jsonl` (git history)
