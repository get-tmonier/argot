# Pretrained CodeRankEmbed + JEPA: representation works, mutations don't

## Setup

Phase 7.3 froze **nomic-ai/CodeRankEmbed** (137M params, 768-dim),
loaded via sentence-transformers with `trust_remote_code=True` and
`max_seq_length=512`. The existing `ArgotPredictor` / JEPA head ran on
top, with texts pre-encoded once on MPS and the predictor trained on
the resulting embeddings. Scope was pilot only: small-py and small-ts ×
3 seeds × 50 epochs, because runtime (~6–8 min/run) and the flat-trend
priors from 7.1/7.2 made a full 4-hour grid unjustified. Decision gate
unchanged: `synthetic_auc_mean ≥ 0.85` at medium on ≥ 2 of 3 seeds.

## Results

Pilot primary metric: **0.514 ± 0.003 (small-py)** and 0.504 ± 0.001
(small-ts) — 0.333 below gate, std ≤ 0.003, no plausible variance to
close the gap. Per-mutation AUC showed the same failure pattern: same
two mutations were no-ops (`error_flip` 0.500, `quote_flip` 0.500–0.502),
`case_swap` weak (0.514–0.517), `debug_inject` marginal (0.501–0.539).

Secondary metrics flipped the story. The pretrained encoder worked
extremely well on every discrimination task *except* the synthetic
mutations:

- `injected_auc` **0.942 ± 0.004 (small-py)** and **0.965 ± 0.007
  (small-ts)** — home-context + foreign-repo hunks were sharply
  separable.
- `cross_auc_same_lang` **0.75** (0.750 small-py, 0.757 small-ts) —
  stronger than all from-scratch encoders in 7.1 and comparable to the
  best density head in 7.2.
- `shuffled_auc` **0.82–0.89** (0.894 small-py, 0.823 small-ts) —
  token-order coherence preserved from the pretrained embedding.
- Synthetic mutations stayed at **0.50–0.52**.

## Interpretation

Representation was never the bottleneck. CodeRankEmbed already knew
what `camelCase` and `console.log` looked like — it discriminated repo
origin, token order, and foreign-hunk injection sharply with no
fine-tuning. But no training signal in the JEPA loop targeted
mutations, and the mutations are engineered to stay inside the home
distribution. A third architectural swing against a distance-based
objective was ruled out; the next era had to ask whether the hunk
carried any detectable single-token signal at all.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/phase-7/18-pretrained-jepa.md`. Re-written here
for clarity, not copied.*
