# Foreign-structure gate: the irreducible floor (lever sweep, refutation)

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · status: **CLOSED — evidence-backed
floor. Not gatable; not ported.** Follows
[`foreign-structure-ast-pattern-signal.md`](foreign-structure-ast-pattern-signal.md)
(the first-signal memo). Harnesses: `benchmarks/struct_*_probe.py`, `struct_gate_*.py`
(Python `ast`, 7 corpora: scrapy · rich · faker · fastapi · wagtail · saleor · dagster).

## Question

Can argot's **structural** analog of the foreign-vocabulary gate — flag a hunk that uses
AST **patterns** the repo has never used (0-usage structural n-grams), the shape analog of
import/callee 0-usage — reach the base gate's bar: **≥85% catch of foreign-style code at
≤5% over-fire, on every corpus**? Or is there an irreducible floor?

The first-signal memo confirmed the signal exists (0-usage AST bigram AUC 0.74, scales with
codebase size) but is advisory-grade (~27% catch@5%FP). This memo runs the full lever sweep
the goal prescribed — IDF/rarity weighting, richer patterns, fire-on-one-strong,
combine-granularities, same-repo restyle — to either clear the bar or prove the floor.

## Method (cheap novelty-separation harness; no scorer change)

Domain-blind features only (tree-sitter/`ast` **node-kinds** — never identifiers, strings,
literals; matches `shape_primitive.rs` + the no-hardcoded-domain rule). Foreign-style proxy =
functions/statement-windows from *other* same-language repos (the "generically-styled paste").
Key refinement over the first memo: measure at **hunk granularity** (a 3-statement window,
what argot actually scores), not whole-function, and with **independent, de-circularized
positive sets**. Over-fire = the repo's own held-out hunks firing (the false-alarm half of the
metric split). `bg_df[pattern]` = fraction of *other* repos using the pattern (a domain-blind
"is this globally idiomatic" prior; the IDF/rarity lever).

## Results (every lever, win or loss)

### 1. Formulation: production rules + IDF weighting beat linear n-grams (whole-function AUC)
`struct_ngram_probe2.py` — 4 pattern types × 4 aggregations, cross-repo whole-function:

| pattern | agg | AUC | catch@5% |
|---|---|---:|---:|
| **prod** (node→ordered child kinds) | **idf_mean** | **0.83** | **40%** |
| trigram | idf_mean | 0.74 | 34% |
| bigram | frac (baseline) | 0.74 | 28% |
| prod / trigram / bigram | maxbg | 0.73–0.79 | 8–13% |

IDF (bg_df) weighting helps every pattern type — flagging *globally-common-but-locally-absent*
patterns (real foreign structure) beats flagging any 0-usage pattern (combinatorial noise).
Whole-function `maxbg` ("one strong pattern fires") is worst — too noisy at that granularity.

### 2. Granularity: at the hunk level the signal is much weaker — most code is mundane
`struct_hunk_probe.py` — scoring 3-statement windows (argot's real unit), prod+idf_mean:
AUC drops to **0.72, catch@5% ~25%**. Cause (decisive): **most hunks — foreign OR native —
are structurally mundane** (a simple assignment looks identical in every repo). The signal can
only fire on hunks containing a genuinely-distinctive foreign idiom, a *minority* of foreign
code. `struct_frontier_probe.py` stratifies by loudness (max bg_df of a hunk's native-absent
productions) at a fixed 5% over-fire:

| loudness bucket | mean catch |
|---|---:|
| [0.00, 0.25) mundane | **0%** (correctly ignored) |
| [0.50, 0.75) | 36% |
| [0.75, 1.00) loud | 64% |

### 3. Representation: **bigram wins on FP** — sparse productions self-generate novelty
`struct_repr_probe.py` — the over-fire driver found. Arity-sensitive ordered-child productions
read native variation as novelty (`Call→(Attr,Const)` in train vs `Call→(Attr,Const,Const)` in
test → "novel"). Native self-novelty rate per representation, and loud-catch@5%FP:

| repr | nat self-novelty | loud-catch@5%FP |
|---|---:|---:|
| **bigram** (parent→child) | **0.3%** | **80%** |
| prod_head2 | 0.9% | 63% |
| prod_uord (sorted unique kids) | 1.7% | 60% |
| prod (ordered, cap 8) | 2.7% | 53% |

Bigram's small, fast-saturating alphabet is the *correct* structural analog of the import gate:
a 0-usage-but-globally-common bigram is a clean, rare signal; sparse productions are not.

### 4. Fire rule + saturation: `maxbg` over-fires badly; saturation decays slowly
`struct_firerule_probe.py` — the "one loud production fires" rule (maxbg≥τ) on **productions**
gives **24–34% over-fire** (native code constantly uses globally-common productions absent from
a 70% sample). Growing train 50→90% drops over-fire only 34→24% — the random split is somewhat
pessimistic vs a full-history fit, but even extrapolated, young-repo over-fire stays ≫5%.

### 5. Honest catch (bigram gate, de-circularized) — the floor
`struct_gate_honest.py` / `struct_gate_kcross.py`. Gate = `#{bigrams 0-usage in repo with
bg_df≥τ} ≥ k`, τ=0.5, vocab attested at mindf=1. Distinctness defined on the **production**
family (independent of the bigram gate) → **non-circular cross-family catch**:

| k | over-fire (mean) | over-fire (worst) | catch of production-distinct idioms |
|---|---:|---:|---:|
| 1 | 4.1% | 9.4% (faker) | **28%** |
| 2 | 1.4% | 4.1% (fastapi) | **14%** |
| 3 | 0.3% | 0.8% | 6% |

The two families catch *different* idioms: a foreign sibling-arrangement (foreign production)
is often built from individually-native parent→child bigrams, so the bigram gate misses 72% of
production-distinct idioms even at k=1.

### 6. Union gate (last lever) — confirms irreducibility
`struct_gate_union.py`. Fire if `(#foreign bigrams≥kb) OR (#foreign productions≥kp)`. Positive
= **all** home-idiomatic foreign windows (fully independent, mundane included) — the honest
recall of a pasted real foreign idiom:

| (kb,kp) | worst-corpus over-fire | catch-any | all corpora ≤5%? |
|---|---:|---:|:--:|
| (1,1) | 31.7% | 40% | ✗ |
| (2,2) | 11.0% | 18% | ✗ |
| (3,2) | 10.5% | 17% | ✗ |
| **(3,3)** | **3.9%** | **8%** | **✓ (only cell)** |

**The only operating point with over-fire ≤5% on every corpus catches 8% of pasted foreign
idioms.** Every higher-recall cell over-fires 10–32% on the young repos.

## Verdict — irreducible, evidence-backed floor

Across 7 corpora and the complete lever sweep (IDF/bg_df weighting ✓, productions/trigrams/
vpaths ✓, maxbg vs idf_mean vs k-of-n ✓, mindf 1/2 & whole-repo vocab ✓, window sizes 1–4 ✓,
family union ✓, splice/injection as the same-repo restyle proxy ✓), **the structural-foreignness
gate cannot reach ≥85% catch at ≤5% over-fire — not on every corpus, not on any corpus.** The
honest recall ceiling, at an over-fire budget ≤5% on *every* corpus, is **~8–13%**.

Two fundamental, independent causes — both structural, neither a tuning miss:

1. **Only ~13% of foreign code is structurally distinct.** Programming languages have a small
   structural alphabet; genuine foreign idioms overwhelmingly reuse the repo's own node-kind
   bigrams. The remaining ~87% is universal structure a guardrail *correctly* ignores. So catch
   of *arbitrary* foreign-style code is intrinsically capped far below 85% — pushing past it
   means flagging idiomatic code (over-fire), the exact failure argot's co-headline forbids.
2. **Over-fire is corpus-size-dependent** — the "a voice needs a corpus" law, the same law that
   governs the base gate. Mature repos (saleor/dagster/wagtail) over-fire ≤2%; young repos
   (faker/fastapi) exceed 5% at any recall-useful threshold, because their structural vocabulary
   hasn't saturated. There is no single (k, τ) gatable on *every* corpus above ~8% recall.

This is **distinct from** the settled single-construct "convention" limit (`die`/`throw`; +1
catch/+45 FP — [issue-92 capstone](issue92-investigation-capstone.md)) — that was aggregate
style; this is 0-usage-*pattern* detection, a different mechanism — but it lands in the same
place: **a real but non-gatable signal at argot's FP discipline.** The first memo's advisory-
grade read was right; this memo proves the ceiling is a structural property, not a tuning gap.

## Decision

- **Do NOT port a structural gate into argot-core.** Per the method, porting is gated on the
  cheap harness clearing ≥85%/≤5%; it did not. Shipping an 8–13%-recall gate that over-fires on
  young repos would violate the co-headline FP discipline and the settled no-net-negative-
  structural-signal rule. The base guardrail stays byte-for-byte untouched (no `crates/` change).
- **No full bench run.** A full bench of a known-8%-recall signal spends the expensive resource
  the research workflow reserves, for zero decision value. The cheap harness already decided it.
- **Advisory variant** (surface the most structurally-foreign hunk as a non-gating F-finding,
  like the semantic layer) is *possible* on mature repos (AUC 0.74–0.83) but was not the ask
  (the goal wanted a gate) and duplicates the semantic layer's advisory role — left as an open
  product question for the maintainer, not built.

**Reproduce:** `source .venv/bin/activate && python benchmarks/struct_gate_union.py` (headline
floor) and `benchmarks/struct_gate_kcross.py` (cross-family catch). Intermediate exploratory
probes recorded here; scripts removed per the research-workflow (evidence survives, scripts don't).
