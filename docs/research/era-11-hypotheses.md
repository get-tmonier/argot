# Era 11 — Hypothesis Space

## The Unsolved Problem

Six corpora at the era-10 baseline: avg recall 85.27%, max FP 1.4%, all CVs ≤3%.
The remaining gap is dominated by faker-js's "uncaught" cluster: 8 fixtures across
categories `foreign_rng`, `http_sink`, `runtime_fetch`, `error_flip`. These fixtures
missed at era-10 because the called functions (`Math.random`, `fetch`,
`Promise.resolve`, `crypto.randomBytes`, `navigator.sendBeacon`) ARE attested in the
faker-js source tree — they appear in jest test files and locale utilities elsewhere
in the codebase. The break is **contextual** ("X called in a file where X doesn't
belong") rather than **categorical** ("X is foreign to the repo"). No call-receiver
weighting variant can address this: any approach that asks "is this callee attested
globally?" will answer yes and contribute zero penalty, regardless of how the weight
formula is parameterized.

## What's Been Tried — Do Not Retry

| Approach | Era | Outcome | Why this approach can't reach the faker-js cluster |
|:---|:---|:---|:---|
| Import-graph (Stage 1) | 4 | shipped | Faker-js break hunks introduce no foreign imports — `fetch`/`Math`/`Promise` are JS globals or imported elsewhere |
| BPE log-ratio (Stage 2) | 4 | shipped | Tokens are familiar; BPE 0.5–4 well below threshold |
| Typicality filter | 5 | shipped | Hunks aren't structurally atypical; filter doesn't apply |
| Call-receiver presence | 6 | shipped | Counts unattested callees; faker-js missed fixtures' callees ARE attested |
| Complex-chain canonicalization | 8 | shipped | Targets `Router().route().get()` patterns; not the cluster |
| Alpha tuning | 9 | shipped | Tunes penalty weight; doesn't change attested-vs-not relation |
| Multi-seed median calibration | 10 Phase 1 | shipped | Threshold-variance fix; orthogonal to the recall gap |
| Root-conditional weighting | 10 Phase 2 | shipped | Catches `{foreign method, known root}`; faker-js roots ARE attested so scorer skips callees entirely |
| Per-callee log-rarity weighting | 10 Phase 3 v1 | failed (saturation) | With vocab ~5000, every callee weights identically — documented structural bound |
| Per-callee fraction-of-unattested | 10 Phase 3 v2 | failed (zero on attested) | fraction=0 when every callee attested, even when combination signals break — documented structural bound |
| AST-shape / CFG fingerprint | 2, 3 | failed (×4 cross-domain collapses) | AST-structural signals are FastAPI-tuned; collapse on rich, click |
| MLM surprise | 2 | failed | Inverted; no corpus-specific signal |

## Hypotheses Worth Testing for Era 11

### A. Path-Aware / File-Cluster Attestation (PRIMARY CANDIDATE)

**Hypothesis**: callees that are "OK in this kind of file" but "weird in that kind of
file" become detectable when the attested set is file-cluster-conditional rather than
repo-global. faker-js's `Math.random` is attested in test files (likely) and dev
tooling but absent from provider files. A path-aware scorer that scores each hunk
against its file's cluster's attested set would surface this.

**Mechanism**:

1. At fit time, cluster files by callee-bag similarity (Jaccard or cosine) into K
   clusters automatically.
2. Each cluster gets its own attested set / counts.
3. At score time: identify hunk's file's cluster, compute `weighted_contribution`
   against the cluster's attested set instead of the global one.

**Risks**:

- Cluster definition is hyperparameter sensitive (K choice). Need to derive K from
  corpus size or use silhouette/elbow.
- Compute: pairwise file similarity is O(n²); for ~10k files per corpus that's ~50M
  comparisons. Subset to top-k MinHash neighbors to bound.
- Path-pattern hardcoding is **FORBIDDEN** — clustering must be derived entirely from
  callee statistics, not file paths.

**Pre-registered question**: does cluster-conditional attestation reduce faker-js
missed fixtures from 8 to ≤5 without regressing other corpora?

**Implementation hint**: see `engine/argot/scoring/scorers/call_receiver.py`'s
attested set construction. Cluster step inserts between fit-time file enumeration
and attested-set construction.

**EV estimate**: highest of all era-11 candidates. Probability of clearing gates: 40–60%.

### B. Cross-File Cohort Scoring (Nearest-Neighbor Variant)

**Hypothesis**: instead of fixed clusters, each file gets its own cohort = top-K most
similar files by callee-bag. Cohort attested = union of callees in cohort. Same
general idea as A but per-file granularity.

**Mechanism**:

- Per file, find top-K nearest neighbors by callee-bag similarity.
- `cohort_attested[file]` = ∪ callees(neighbor) for neighbor in top-K.
- Score hunk against `cohort_attested[hunk.file]`.

**Risks**: similar to A. K is a knob. More compute (per-file rather than per-cluster).

**EV**: similar to A. Try as fallback if A fails.

### C. In-Hunk-Definition-Aware Weighting (Limited Scope)

**Hypothesis**: distinguish "newly defined helper" (legit) from "external identifier"
(suspicious) by parsing the hunk for definitions.

**Mechanism**:

- Parse hunk for `def`, `class`, `function`, `const`/`let`/`var` declarations.
- If callee root is in-hunk-defined set: weight 0 regardless of attestation.
- Else: standard call-receiver weighting.

**Status**: doesn't address faker-js cluster — `Math`/`fetch`/`Promise` are globals,
never in-hunk-defined. The existing scorer already treats them as "external" but they
are attested so weight=0 anyway. Worth considering if path-aware (A) doesn't deliver,
as it might reduce FP risk for legitimate helper-add PRs in rich/fastapi.

**EV**: moderate. Probability faker-js gain: ~10%; probability FP reduction: ~30%.
Use only as era-12 candidate.

### D. (FORBIDDEN) Per-Corpus Path-Pattern Hardcoding

**Hypothesis**: explicitly hardcode `src/locales/*` → strict scoring,
`src/test/*` → loose scoring per corpus.

**Status**: BANNED. Per-framework hardcoding violates project rules established in
eras 1–5. Not implementable as era 11. Do not retry.

### E. Rare (file_dir, callee) Pair Scoring

**Hypothesis**: count[(file_dir, callee)] across repo. A pair attested globally but
absent in this directory = signal.

**Mechanism**: weight = −log P(callee | file_dir). Combinatorial.

**Status**: subsumed by A/B with less elegance. Skip.

### F. Pivot to Non-Research Work

Options if A and B both fail:

- **Multi-language support** (Rust/Go/Ruby adapters): each adapter costs ~1 era of
  work but expands argot's reachable market.
- **IDE integration** (VS Code language server, JetBrains plugin): UX step.
- **CI integration** (GitHub Action that posts argot check results to PR comments):
  adoption step.
- **Speed**: profile and optimize the BPE tokenization pass for large repos.

These aren't research eras. They're product/engineering eras. Reasonable if
path-aware scoring (A) doesn't unlock the faker-js cluster — at that point, recall on
real-world repos is already very strong, and the marginal recall lift from one more
research era is dwarfed by the marginal value from a Rust adapter or IDE integration.

## Recommendation

**Era 11 primary: Path-aware / file-cluster attestation (A).**

- Addresses the documented faker-js cluster directly
- Uses statistics derived purely from the repo (no hardcoding)
- Mechanism is interpretable (which cluster catches/misses)
- Compute is tractable with MinHash approximation

If A's pre-registered gates fail: try B (cohort) as era-11 fallback.

If both fail: era 12 = F (pivot to non-research).

Do NOT bother with C, D, E for era 11. They are either subsumed, forbidden, or
off-target.

## Era-10 → Era-11 Transition

Strict 91+ fixture verdict parity is the standing rule. Apply at era-11 dispatch.

Era-11 dispatch is a single short prompt: "Read
`docs/research/era-11-hypotheses.md` and implement Hypothesis A per the
specification there."

## End of Document
