# Phase 7 — Honest eval + pretrained encoder pivot

**Status**: Phase 7.0 complete — Phase 7.1 next
**Branch**: `research/phase-7-honest-eval`
**Parent design**: [`DESIGN.md`](DESIGN.md)
**Tracker**: [`ROADMAP.md`](ROADMAP.md)

## Why

Six phases of architecture experiments have produced a consistent picture:

- Current best shuffled AUC is **0.713** (combined TF-IDF at large) / **0.704** (BPE at large).
- Target for v1 is **AUC ≥ 0.85** on the slice that matters (style-outlier detection).
- The eval protocol has a structural flaw: every bucket pairs a TS repo with a Py repo,
  so `cross_auc` and `injected_auc` mostly measure language detection, not style
  discrimination. `shuffled_auc` is the only fully honest metric, and it measures
  token-order coherence — a weak proxy for the style violations users care about
  (`camelCase` in a `snake_case` repo, rogue `console.log`, inverted error handling).
- Every from-scratch technique tested so far (TF-IDF, char_ngrams, word_ngrams,
  token_embed, BPE) has hit a ceiling around 0.70. The big untried lever is a
  **pretrained code encoder** (CodeRankEmbed, CodeSage).

Phase 7 does two things:
1. Rebuild the eval so synthetic-outlier AUC is the primary, honest metric.
2. Search for an architecture that clears 0.85 on the new eval, stopping at the
   first experiment that clears the bar.

## Success criteria

- A rebuilt eval with same-language corpus pairs and synthetic outlier mutations.
- `synthetic_auc_mean` computed at small/medium/large; per-mutation AUCs logged.
- One of the architecture experiments (7.2–7.5) reaches `synthetic_auc_mean ≥ 0.85`
  at the **medium** bucket on ≥ 2 of 3 seeds, OR a defensible negative result
  ("style-outlier detection requires class X of model that we cannot run locally").

## Non-goals

- No shipping. Production code path stays on `research/combined-optimizations`
  defaults (char_ngrams + adaptive epochs + context_after) until Phase 8.
- No calibration work (rolling percentiles, thresholds, cold-start). That's
  Phase 8, conditional on a Phase 7 winner.
- No new target languages beyond TS + Python.
- No benchmark changes mid-phase — the eval is frozen after 7.0.

## Design

### 7.0 — Eval rebuild

**Same-language corpus pairs.** Replace the current TS+Py bucket pairs with two
parallel same-language sub-corpora per bucket. Candidates below are selected
on two criteria: (a) same domain so we control for domain effects, (b) strong
contrast on at least three style axes so `cross_auc_same_lang` has real signal
to learn rather than noise.

| bucket | Python pair        | domain             | TypeScript pair       | domain              |
|:-------|:-------------------|:-------------------|:----------------------|:--------------------|
| small  | httpx + requests   | HTTP client        | ky + chalk            | Small util library  |
| medium | fastapi + flask    | Web framework      | vite + rollup         | Bundler             |
| large  | pydantic + django  | Lib/framework, mature | effect + angular  | (extreme contrast)  |

**Style-contrast matrix.** Each pair is chosen to differ on multiple axes so
a well-trained model has discriminative signal to latch onto:

| pair              | types | OOP vs fun | docstring/comment | error handling | decorator | other              |
|:------------------|:-----:|:----------:|:-----------------:|:--------------:|:---------:|:-------------------|
| httpx + requests  | ✓     | (≈)        | ✓                 | ✓              | ✓         | async, line length |
| fastapi + flask   | ✓     | ✓          | ✓                 | (≈)            | ✓         | metaprogramming    |
| pydantic + django | ✓     | ✓          | ✓                 | ✓              | (≈)       | ORM patterns       |
| ky + chalk        | ✓     | ✓          | (≈)               | (≈)            | (n/a)     | ESM/CJS, exports   |
| vite + rollup     | (≈)   | (≈)        | ✓                 | (≈)            | (n/a)     | file organisation  |
| effect + angular  | ✓     | ✓ (extreme)| ✓                 | ✓              | ✓         | generators vs DI   |

**Selection notes:**

- **ruff dropped from small Py.** Ruff's Python surface is almost entirely
  integration tests — the library itself is Rust. Records from ruff's history
  don't represent "ruff style" in any meaningful sense for a Python eval.
  Replaced with `httpx + requests`, which are both HTTP clients and contrast
  sharply on every style axis we care about.
- **Large TS pair is domain-mismatched deliberately.** `effect + angular` are
  not the same domain (FP library vs UI framework), but the style contrast is
  so extreme that if argot can't distinguish them it will never distinguish
  anything subtler. Alternative: `effect + typescript-eslint` (both tooling,
  still strong FP vs OOP contrast) if the angular domain mismatch proves
  problematic during 7.0.
- **Existing extracts reused where possible.** `ky`, `vite`, `effect`,
  `pydantic`, `django` are already extracted under `.argot/research/datasets/`.
  Phase 7.0 will re-verify those SHAs still parse cleanly, then add the new
  partners.

**Record-count targets are hypotheses.** The bucket sizes (~3k, ~7k, ~20k)
were set for the original TS+Py pairs based on existing extracts. The new
same-language pairs may not cluster at these tiers:

- `httpx` and `requests` may both land in the 3k–8k range and force a merged
  "small–medium" bucket.
- `rollup` and `vite` may differ in scale by a factor of 2–3.
- `angular` is enormous; a 20k downsample is fine, but the sample may be
  dominated by a narrow time window.

Post-extraction outcomes that force a revision:
- If any candidate's sub-corpus is < 40% of its bucket target after extract,
  swap it out and log the swap in `15-honest-corpus.md` §Swap-out log.
- If the actual record distribution doesn't cluster at 3k/7k/20k, rename the
  buckets to match the distribution (e.g. 5k/15k/40k) rather than force-fit.
- Each bucket must still have ≥ 40% share per sub-corpus after downsample.

Exact SHAs, URLs, and record counts pinned during implementation in
[`15-honest-corpus.md`](15-honest-corpus.md). Swap-outs logged there.

Each bucket produces two files — `<bucket>-ts.jsonl` and `<bucket>-py.jsonl` —
and each is benchmarked independently. `cross_auc_same_lang` is computed
within-language only (no cross-language comparison).

**Style-axis reference.** The axes below are what `synthetic_auc_mean` tries
to target through mutations, and what `cross_auc_same_lang` measures broadly
across the pair. They are documented here so that mutation design (in
`mutations.py`) and corpus selection are aligned.

| axis                       | Python signal              | TypeScript signal           |
|:---------------------------|:---------------------------|:----------------------------|
| type annotation density    | `-> T` on every def        | strict tsconfig, no `any`   |
| class vs functional        | class density, `@staticmethod` | class density, `this.` usage |
| docstring / comment style  | Google / NumPy / reST      | JSDoc density, inline comments |
| error handling idiom       | `raise` density, `try/except` | `throw`, `Result<T,E>`, `never` |
| decorator density          | `@decorator` per def ratio | `@Decorator` on classes     |
| metaprogramming            | `__init_subclass__`, `@overload` | mapped types, conditional types |
| line length / wrapping     | ruff/black line length     | prettier printWidth         |
| quote / semicolon style    | `'` vs `"`                 | `;` on/off, `'` vs `"`      |
| import organisation        | absolute vs relative, grouping | ESM vs CJS, barrel files |
| naming convention          | snake_case strict          | camelCase + PascalCase types |

A mutation that targets an axis (e.g. `case_swap` targets naming; `error_flip`
targets error handling) should produce a measurable AUC on that axis if the
corpus pair genuinely contrasts on it. During 7.0 we sanity-check each
mutation produces `synthetic_auc_<mutation>` ≥ 0.55 on the re-baseline
(7.1); a mutation that is invisible even at 7.1 is either mis-designed or
targeting an axis the pair doesn't actually contrast on.

**Synthetic outlier mutations.** Deterministic, seeded transforms applied to the
held-out "normal" slice's `hunk_tokens` (context unchanged). Each mutation is
a distinct eval set with its own AUC.

**Extractor constraint:** the current tree-sitter extractor emits syntactic
tokens only. Comments and indentation whitespace are not present in the token
stream. Mutations must therefore operate on the token-text surface (identifier
names, punctuation tokens, string literals). Mutations that would require
comment or whitespace preservation are deferred to a possible future
"extractor-v2" study and are not in Phase 7 scope.

**Feasible mutation set** (4 mutations):

| id              | description                                                                 |
|:----------------|:----------------------------------------------------------------------------|
| `case_swap`     | Flip identifier convention for identifier-kind tokens in the hunk (snake ↔ camel ↔ Pascal). Context untouched. |
| `debug_inject`  | Insert a short debug-call token subsequence at a plausible position inside the hunk. Language-aware (`console.log("DEBUG")` for TS, `print("DEBUG")` for Py). |
| `error_flip`    | Where a `try` token appears, rewrite the adjacent error-handling tokens: `except`/`catch` → `finally`, `raise`/`throw` → `return` (Py) / `return null` (TS). Token-level regex on the surface stream. |
| `quote_flip`    | Swap the quote character on string-literal tokens: `"x"` ↔ `'x'`. Preserves content, flips style. |

Dropped from the original design:
- `comment_lang` — comments not in the token stream.
- `indent_flip` — whitespace not in the token stream.

Implementation: `engine/argot/benchmark/mutations.py`. Each mutation is a pure
function `(record, seed) -> record`. Mutations are applied to held-out normal
records; the original record scores a "normal" point and the mutated record
scores an "outlier" point. AUC is computed per mutation.

**Language dispatch:** mutations that need language context (`debug_inject`,
`error_flip`) read `record["language"]` (currently dropped by `_load_records`
during the benchmark slim — Phase 7.0 restores it). Mutations that don't
need language (`case_swap`, `quote_flip`) are language-agnostic.

**Metrics produced by the new harness**:

| metric                        | meaning                                                      | decision-relevant |
|:------------------------------|:-------------------------------------------------------------|:------------------|
| `synthetic_auc_mean`          | Average AUC across the 5 mutations (primary Phase 7 metric) | ✓ primary         |
| `synthetic_auc_<mutation>`    | Per-mutation AUC (diagnostic)                                | ✓ per-mutation    |
| `shuffled_auc`                | Token-shuffle AUC within a record (sanity)                   | ✓ sanity          |
| `cross_auc_same_lang_ts`      | Cross-repo AUC within the TS sub-corpus                      | ✓ secondary       |
| `cross_auc_same_lang_py`      | Cross-repo AUC within the Py sub-corpus                      | ✓ secondary       |
| `cross_auc` (TS+Py, legacy)   | Kept for continuity with Phase 2–6 docs; no decision weight | ✗ deprecated      |
| `injected_auc` (TS+Py, legacy)| Same                                                          | ✗ deprecated      |

**Harness changes** to `engine/argot/benchmark/`:
- `mutations.py` — the five mutation functions + a dispatcher
- `eval.py` — refactor AUC aggregation to compute per-mutation scores
- `corpus.py` — same-language sub-corpus loading, `--lang ts|py` flag
- `justfile` — `just research honest-benchmark` wraps the new protocol

**Deliverable**: [`15-honest-corpus.md`](15-honest-corpus.md) — corpus pins,
mutation spec, sample mutated records, harness usage.

### 7.1 — Re-baseline on new eval

Re-score the four existing encoders against the new eval (3 seeds each):

1. TF-IDF baseline (no techniques)
2. `char_ngrams` (Phase 3 winner)
3. `token_embed` (Phase 5 dense encoder)
4. `bpe` (Phase 6 subword encoder)

This establishes the honest from-scratch ceiling before we commit to pretrained.
No new code — just re-running existing pipelines on the new corpus/mutations.

**Deliverable**: [`16-rebaseline.md`](16-rebaseline.md) — one results table
with synthetic/shuffled/same-lang-cross per encoder per bucket.

**Decision gate 7.1**: If any of the four re-baseline encoders clears 0.85
synthetic AUC at medium, we skip to Phase 8 immediately. (Unlikely but worth
checking — the old eval may have been hiding real progress behind language
noise, or, more likely, confirming that from-scratch has indeed plateaued
around 0.70.)

### 7.2 — Density heads on BPE

Head-swap experiment. Keep the BPE encoder, replace the JEPA predictor with a
density estimator. Surprise = `-log p(embedding)` or mean kNN distance.

**Variants** (3 seeds each):
- `knn-20` — k=20 nearest neighbours on mean-pooled BPE embeddings, cosine distance
- `gmm-8`, `gmm-16`, `gmm-32` — Gaussian mixture, full covariance, scikit-learn

**Why this comes before pretrained**: it's cheap (~50 LOC, no new deps
beyond scikit-learn which we already have) and it separates two hypotheses
we've conflated. If kNN on our existing embeddings jumps AUC meaningfully,
then the "predictor distance" objective was the bottleneck, not the
representation. If it doesn't help, we know the encoder itself is the limit.

**Deliverable**: [`17-density-heads.md`](17-density-heads.md).

**Decision gate 7.2**: If kNN or GMM clears 0.85 synthetic AUC at medium on
≥ 2 of 3 seeds, Phase 7 closes immediately — we have a winner. If none
clears, record the best-performing head (highest mean synthetic AUC) and
carry it forward as input to 7.4.

### 7.3 — Pretrained encoder + current head

Frozen pretrained code encoder (no fine-tuning, no gradient flow into the
encoder weights); existing `ArgotPredictor` / JEPA predictor architecture
as the head (unchanged from Phase 5/6); same training loop as `_train_bpe`
with the encoder swapped out.

**Encoder**: `nomic-ai/CodeRankEmbed` (137M params, Apache-2.0, CPU-runnable).
Loaded via `sentence-transformers`. Cached to `~/.cache/huggingface/`.
Fallback candidate: `codesage/codesage-small-v2` if CodeRankEmbed has
licensing/download issues.

**Integration**:
- New file `engine/argot/jepa/pretrained_encoder.py` wraps
  `sentence-transformers` as an encoder exposing the same interface as
  `MeanPoolEncoder`.
- `train.py::_train_pretrained` mirrors `_train_bpe` but with frozen encoder.
- `corpus.py`: `"pretrained"` added to `--encoder` choices.

**New dep**: `sentence-transformers` added to `engine/pyproject.toml`.

**Deliverable**: [`18-pretrained-jepa.md`](18-pretrained-jepa.md).

**Decision gate 7.3**: If ≥ 0.85 synthetic at medium, Phase 7 closes; Phase 8
begins. Otherwise continue to 7.4.

### 7.4 — Pretrained encoder + best density head

Cross-product: winning encoder from 7.3 + winning head from 7.2.

**Deliverable**: [`19-pretrained-density.md`](19-pretrained-density.md).

**Decision gate 7.4**: 0.85 at medium → Phase 8. Else 7.5.

### 7.5 — Structural context (conditional)

Runs only if 7.2–7.4 all fail. Take the best encoder+head from Phase 7 so far;
add file-level structural context via tree-sitter:

- Prepend the file's imports
- Prepend the prior 3 function definitions within the same file
- Cap combined context at 256 tokens (truncate oldest signatures first)

**Deliverable**: [`20-structural-context.md`](20-structural-context.md).

**If 7.5 also fails**: Phase 7 concludes as a negative result. The next step
is a product conversation — either relax the local-first constraint, relax the
0.85 target, or accept that argot's style-outlier premise needs reframing.

## Working protocol

One experiment at a time. One variable per experiment. 3 seeds. Mean ± std
reported in every report.md. No cherry-picked seeds.

All experiments read the same frozen eval produced in 7.0. **The eval is
frozen at the end of 7.0** and does not change during 7.1–7.5. If we discover
a mutation bug mid-phase, we note it, we do not re-run prior experiments with
the fix — we fix it for Phase 8.

Decision gates (7.1 – 7.5) are STOP points. At each gate:
1. Summarise results in the phase report.
2. Update `ROADMAP.md`.
3. Wait for explicit approval before moving to the next experiment.

## Branch layout

- `research/phase-7-honest-eval` — branch for 7.0 (eval rebuild) + 7.1
  (re-baseline). Most Phase 7 work lands here.
- `research/phase-7-density-heads` — worktree for 7.2 only.
- `research/phase-7-pretrained` — worktree for 7.3 + 7.4.
- `research/phase-7-structural` — worktree for 7.5 if triggered.

All worktrees branch off `research/phase-7-honest-eval` after 7.0 lands.

## Open questions (to resolve during implementation)

- **Same-language corpus pin**: finalise URLs and SHAs in 15-honest-corpus.md
  during 7.0. Record counts above are hypotheses based on public commit
  activity; actual extracted counts may cluster differently. Acceptable
  outcomes: (a) candidates land on target, (b) buckets are renamed to match
  actual distribution, (c) individual candidates are swapped out (log in
  corpus doc §Swap-out log). Each sub-corpus must remain ≥ 40% of its
  bucket's downsample target after extraction.
- **Post-7.0 hypothesis check**: the candidate list above was chosen for
  *style-axis contrast*, not verified against extracted record counts.
  After 7.0 completes, revisit this design doc and either confirm the
  choices or record deviations in a §7.0 retrospective subsection.
- **Mutation authenticity**: `case_swap` and `comment_lang` risk producing
  records that are still syntactically/lexically valid but "obvious" to any
  character-level model. That's fine — the mutations are a detection test,
  not a Turing test. But `error_flip` needs care to avoid producing
  non-compiling code that a language-detector would trivially catch.
  Resolved during 7.0 with sample mutated records reviewed.
- **Pretrained encoder size budget**: CodeRankEmbed is ~500 MB on disk.
  Acceptable per the Q3 decision, but if we later move toward shipping,
  Phase 8 may re-test with CodeSage-small or a distilled variant.
- **xlarge bucket**: still parked (memory issues from Phase 3). Re-evaluate
  after Phase 7 — if the winning architecture has a lower memory footprint
  than the current JEPA, xlarge may become feasible.

## Infrastructure deltas

- `engine/pyproject.toml`: add `sentence-transformers ^3.0`
- `engine/argot/benchmark/mutations.py`: new
- `engine/argot/jepa/pretrained_encoder.py`: new (in 7.3)
- `engine/argot/validate.py`: extend `_vectorize_*` dispatch for pretrained
- `justfile`: `just research honest-benchmark` target
- `.argot/research/datasets-v2/`: new dir for rebuilt corpus (old
  `datasets/` kept for historical comparison)

## Testing

- Mutation functions get unit tests (pure functions with clear contracts).
- Encoder dispatch tests extended for `"pretrained"` in `corpus.py` choices.
- No unit tests for the experiments themselves — they ARE tests (they measure
  the model). Every experiment's `report.md` is the authoritative output.
- `just verify` must pass for every infra change (mypy strict, no-any, ruff).

## Phase 7 → Phase 8 handoff

On successful completion (any experiment clears 0.85):
- Freeze the winning encoder + head + pipeline in
  `docs/research/scoring/PHASE-7-WINNER.md`.
- Phase 8 design doc written separately: covers calibration (rolling
  percentiles), cold-start behaviour (minimum corpus thresholds), and
  production integration (replacing the current default in
  `research/combined-optimizations`).

On negative completion (7.5 fails):
- Phase 7 synthesis doc: `docs/research/scoring/21-phase-7-synthesis.md`.
- Product conversation: do we relax local-first? Relax 0.85? Reframe the
  use-case? No autonomous decision — user choice.
