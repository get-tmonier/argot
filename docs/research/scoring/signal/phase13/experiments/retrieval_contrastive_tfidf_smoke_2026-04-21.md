# Phase 13 — Retrieval-Augmented Contrastive TF-IDF Smoke Test (2026-04-21)

## Setup

- Retrieval corpus: 2000 FastAPI records (`corpus_file_only.jsonl`)
- k = 20 nearest neighbors (cosine similarity, UnixCoder BPE embeddings)
- P_A: Laplace add-1 smoothed token frequencies over top-k retrieved hunks
- P_B: `generic_tokens_bpe.json` (generic code reference)
- Fixtures: routing category (break vs control)

## Results

=== Retrieval-Augmented Contrastive TF-IDF Smoke Test ===
Retrieval corpus: 2000 FastAPI records
k=20 nearest neighbors

| Fixture | max | mean | top-3 tokens |
|---|---|---|---|
| paradigm_break_flask_routing | 10.500 | 1.662 | ` _` (10.500) / ` _` (10.500) / ` _` (10.500) |
| control_router_endpoint | 9.901 | 0.756 | `]` (9.901) / `]` (9.901) / `]` (9.901) |

Delta (break − control, max): +0.599

**Verdict: MODERATE SIGNAL, worth Stage 2**

## Retrieved Neighbor Diagnostics

*(Top-3 retrieved neighbors per fixture — key diagnostic for embedding quality)*

### paradigm_break_flask_routing

1. sim=0.5017 — `def include_router ( self , router : Annotated [ routing . APIRouter , Doc ( " The `APIRouter` to in`
2. sim=0.4690 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `
3. sim=0.4690 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `

### control_router_endpoint

1. sim=0.6453 — `def include_router ( self , router : Annotated [ routing . APIRouter , Doc ( " The `APIRouter` to in`
2. sim=0.6120 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `
3. sim=0.6120 — `def delete ( self , path : Annotated [ str , Doc ( """                  The URL path to be used for `

