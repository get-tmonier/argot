# Phase 7.3 — Pretrained CodeRankEmbed + JEPA head

**Branch**: `research/phase-7-honest-eval`
**Status**: pilot complete — decision gate NOT cleared; proceeding to Phase 7.4
**Date**: 2026-04-19

## Setup

- Encoder: **nomic-ai/CodeRankEmbed** (137M params, 768-dim), loaded via
  `sentence-transformers`, weights frozen, `trust_remote_code=True`.
  `max_seq_length` capped at 512 tokens (model default 8192 → MPS OOM at our
  batch sizes).
- Head: existing `ArgotPredictor` / JEPA predictor (unchanged from 5/6/7.1), only
  `embed_dim` parametrised to 768.
- Training: texts pre-encoded once on MPS; `JEPAArgot` wraps `nn.Identity()` as
  the encoder slot; AdamW trains the predictor only.
- Device: Apple Silicon MPS (encode + train).
- Scope: **pilot** — 2 buckets (small-py, small-ts) × 3 seeds × size=3000 × 50
  epochs. Medium/large not run; see "Scope" below.
- Output: `.argot/research/results-7-3-pilot.jsonl`

## Scope — why pilot only

The Phase 7 decision gate is specified at medium (≥ 0.85 on ≥ 2/3 seeds). We ran
the small bucket only because:

1. **Runtime**: each run takes ~6 min on py, ~8 min on ts (MPS-bound on
   re-encoding text for every score phase — encode dominates, not train).
   A full 6-bucket × 3-seed grid would be ~4 hours.
2. **Flat-trend priors**: Phase 7.1 (4 from-scratch encoders) and 7.2 (4 density
   heads) both saw `synthetic_auc_mean` plateau at 0.48–0.58 across all 3 sizes
   with std ≤ 0.014 — size was not a lever.
3. **Pilot measured**: if the synthetic signal were there at all, it should
   appear at small with even a weak ordering; if it is chance, small locks it
   in.

If the pilot had hit ~0.70+ on small we would have escalated to medium/large to
confirm. The observed 0.50–0.52 at small (std ≤ 0.003) makes medium extrapolation
essentially certain — the mutations are undetectable by the embedding, not by the
training-set size.

## Implementation notes

- `PretrainedEncoder` (new file, `engine/argot/jepa/pretrained_encoder.py`) wraps
  `SentenceTransformer`, auto-selects CUDA > MPS > CPU, freezes all parameters,
  and exposes `encode_texts(list[str]) → Tensor`. `forward(x)` is a passthrough
  so the JEPA wrapper's encoder slot is a no-op when embeddings are pre-computed.
- `_train_pretrained` in `train.py`: pre-encodes all ctx/hunk texts upfront,
  builds a `TensorDataset` of embeddings, runs the normal JEPA training loop on
  the predictor only. Encoder slot = `nn.Identity()`.
- `_vectorize_pretrained` in `validate.py` routes scoring through the same
  encoder, returning CPU tensors (predictor runs on CPU at inference).
- Scoring loop in `score_records` batches the `surprise()` forward pass at 128
  (earlier per-record loop caused multi-minute silence post-training on
  768-dim × ~2k held-out).
- New dep: `sentence-transformers>=3.0.0,<4.0.0`; mypy overrides for the
  untyped module.

## Primary metric — `synthetic_auc_mean` (mean ± std over 3 seeds)

| bucket   | synthetic_auc_mean | shuffled_auc   | cross_auc      | injected_auc   |
|:---------|:-------------------|:---------------|:---------------|:---------------|
| small-py | **0.514 ± 0.003**  | 0.894 ± 0.004  | 0.750 ± 0.020  | 0.942 ± 0.004  |
| small-ts | **0.504 ± 0.001**  | 0.823 ± 0.014  | 0.757 ± 0.029  | 0.965 ± 0.007  |

**Decision gate: NOT cleared.** Target was ≥ 0.85 at medium on ≥ 2/3 seeds.
Best observed: **small-py seed 2 = 0.517** — 0.333 below target; std ≤ 0.003
across seeds (no variance that could plausibly close the gap).

## Per-mutation AUC (mean over 3 seeds)

| bucket   | case_swap | debug_inject | error_flip | quote_flip |
|:---------|----------:|-------------:|-----------:|-----------:|
| small-py | 0.517     | 0.539        | 0.500      | 0.502      |
| small-ts | 0.514     | 0.501        | 0.500      | 0.500      |

Same pattern as Phase 7.1/7.2:

- `error_flip` / `quote_flip` land at exactly 0.500 — confirmed no-op when
  trigger tokens (`raise`/`throw`, quote chars) are absent from the hunk.
  Deferred to Phase 8 mutation redesign.
- `case_swap` gives the same weak signal (~0.515) that from-scratch encoders
  got; the pretrained encoder's rich naming-convention knowledge does not help
  the JEPA surprise score distinguish mutated hunks.
- `debug_inject` shows a marginally better small-py signal (0.539) but small-ts
  drops back to chance (0.501).

## Secondary — other metrics

- **`shuffled_auc` very strong** (0.82–0.89, vs 0.50 for density heads in 7.2).
  CodeRankEmbed-derived hunk embeddings + JEPA predictor still capture
  token-order coherence well.
- **`cross_auc_same_lang` (= `cross_auc` here since both buckets are
  single-language) = 0.72–0.77**, higher than all from-scratch encoders in 7.1
  (0.46–0.65) and comparable to the strongest density head in 7.2 (0.74–0.87).
  Pretrained embeddings do carry real cross-repo style structure.
- **`injected_auc` excellent (0.94–0.97)**: home-context paired with
  foreign-repo hunks is the strongest negative. The predictor learns the
  home-context → home-hunk joint distribution sharply.

The encoder works for style/repo-origin discrimination. It does not work for
mutation detection.

## Interpretation

**Representation is not the bottleneck — the training objective is.** Swapping
BPE from-scratch → frozen CodeRankEmbed gives:

- ✓ Big jump on `cross_auc_same_lang` (+0.1 to +0.3 vs 7.1).
- ✓ Big jump on `shuffled_auc` (+0.1 vs 7.1 sequential encoders).
- ✓ Big jump on `injected_auc` (+0.3 vs 7.1).
- ✗ **No jump on synthetic mutation detection** (0.50–0.52 — same as 7.1, 7.2).

CodeRankEmbed encodes code semantics well enough to detect "this hunk came from
a different project" (injected_auc ≈ 0.95), but a JEPA predictor trained to
maximise log-likelihood of real home hunks given their context cannot tell
`return true` from `return TRUE` apart — the mutation is too small in embedding
space, and the predictor is learning one-token-ahead coherence, not mutation
sensitivity.

The three head architectures tried so far all share this blind spot:

| phase | encoder     | head         | best synth_mean | notes                                  |
|:------|:------------|:-------------|:----------------|:---------------------------------------|
| 7.1   | 4 × from-scratch | JEPA predictor | 0.525 (word_ngrams medium-py s0) | plateau |
| 7.2   | BPE         | kNN/GMM density | 0.581 (gmm-8 medium-py) | case_swap @ medium-ts 0.78 |
| 7.3   | CodeRankEmbed | JEPA predictor | 0.517 (small-py s2) | encode cost now dominates runtime |

None crosses 0.60. The bottleneck is **no mutation-specific signal anywhere in
the training loop**: both JEPA surprise and density anomaly score measure
*distance from the home distribution*, but mutations are engineered to stay
inside that distribution (semantics-preserving or near-miss).

## Next

Proceed to **Phase 7.4** — frozen CodeRankEmbed + best density head from 7.2
(gmm-8). Hypothesis: density on CodeRankEmbed's rich 768-dim space has the
best chance of catching `case_swap` and `debug_inject` structure without being
swamped by the 192-dim BPE noise floor from 7.2. If 7.4 also fails,
**mutation-aware supervision** (contrastive or classification head on
synthetic positives/negatives) becomes the only remaining lever before
Phase 7.5's structural context work.

## Deferred

- **Mutation redesign**: `error_flip` / `quote_flip` stay at 0.500 across
  3 phases. Phase 8 mutation-aware trigger selection is now urgent — without
  it every `synthetic_auc_mean` is depressed by ~0.125.
- **Medium/large CodeRankEmbed runs**: worth re-running if 7.4 turns
  encouraging, to confirm size-scaling. Not worth the 4-hour runtime if the
  trend stays at chance.
- **Text caching**: each `score_records` call re-encodes identical hunk texts
  through the 768-dim MPS model (dominates runtime). Add an embedding cache
  keyed by hunk/ctx hash if 7.4+ also uses this encoder.
