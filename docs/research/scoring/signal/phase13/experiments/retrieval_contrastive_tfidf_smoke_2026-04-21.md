# Phase 13 — Retrieval-Augmented Contrastive TF-IDF Smoke Test v2 (filtered) (2026-04-21)

## Setup

- Retrieval corpus: 2000 FastAPI records (`corpus_file_only.jsonl`)
- k = 20 nearest neighbors (cosine similarity, UnixCoder BPE embeddings)
- P_A: Laplace add-1 smoothed token frequencies over top-k retrieved hunks
- P_B: `generic_tokens_bpe.json` (generic code reference)
- Fixtures: routing category (break vs control)
- Token filter: len >= 3 AND has alphanumeric (BPE `##` prefix stripped first)

## Results

=== Retrieval-Augmented Contrastive TF-IDF Smoke v2 (filtered) ===
Retrieval corpus: 2000 FastAPI records
k=20 nearest neighbors
Filter: len ≥ 3 AND has alphanumeric (BPE ## stripped before length check)

| Fixture | n_tokens | n_filtered | max | mean | top-3 meaningful tokens |
|---|---|---|---|---|---|
| paradigm_break_flask_routing | 693 | 256 | 8.684 | 1.861 | `args` (8.684) / `args` (8.684) / `int` (8.365) |
| control_router_endpoint | 1219 | 515 | 9.045 | -0.308 | `file` (9.045) / `key` (8.548) / `key` (8.548) |

Delta (break − control, max):  -0.362
Delta (break − control, mean): +2.168

**Verdict: PUNCTUATION WAS THE ONLY SIGNAL. Retrieval does not capture paradigm. Abandon retrieval direction.**

## Retrieved Neighbor Diagnostics (top-3)

*(Top-3 retrieved neighbors per fixture — key diagnostic)*

### paradigm_break_flask_routing

1. sim=0.5017 — `def include_router ( self , router : Annotated [ routing . APIRouter , Doc ( " The `APIRouter` to in`
2. sim=0.4690 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `
3. sim=0.4690 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `

### control_router_endpoint

1. sim=0.6453 — `def include_router ( self , router : Annotated [ routing . APIRouter , Doc ( " The `APIRouter` to in`
2. sim=0.6120 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `
3. sim=0.6120 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `

