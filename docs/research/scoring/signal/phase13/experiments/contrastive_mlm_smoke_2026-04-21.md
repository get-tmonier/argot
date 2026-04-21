# Phase 13 — Contrastive-MLM Smoke Test (2026-04-21)


## Setup


- Corpus: 500 records from `corpus_file_only.jsonl`
- Training: 1 epoch LoRA, lr=1e-4, batch=4, mask=15%
- Base model: `microsoft/codebert-base-mlm`
- LoRA config: rank=8, alpha=16, target_modules=[query, value]
- Fixtures: routing category only (break vs control, topic held constant)

## Results


| Fixture | max | mean | top-3 tokens |
|---|---|---|---|
| paradigm_break_flask_routing | 1.258 | -0.038 | pos 402: ` return` (1.258) / pos 196: ` abort` (0.963) / pos 293: ` 3` (0.901) |
| control_router_endpoint | 1.149 | -0.038 | pos 164: `
` (1.149) / pos 178: `
` (0.796) / pos 156: ` class` (0.764) |

**Delta (break − control max): 0.109**

## Verdict


**WEAK SIGNAL. Signal may emerge with more training or more data. Marginal call.**

## Notes


- Break fixture: `paradigm_break_flask_routing` — max=1.258, mean=-0.038
- Control fixture: `control_router_endpoint` — max=1.149, mean=-0.038
- v1 (broken corpus) AUC: 0.4645
- BPE-tfidf baseline: AUC 1.0000

