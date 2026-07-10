# Foreign-structure gate: the irreducible floor (lever sweep, refutation)

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · status: **CLOSED — evidence-backed
floor, validated on real infra. Ported feature-gated + NON-GATING (never shipped as a gate).**
Follows [`foreign-structure-ast-pattern-signal.md`](foreign-structure-ast-pattern-signal.md)
(the first-signal memo). Harnesses: Python proxy `benchmarks/struct_gate_{union,kcross}.py`
(7 corpora) + real-infra `argot-bench --mode structural` (`just bench-structural`, 30 corpora /
11 languages, real temporal holdout).

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

## Real-infrastructure validation (the winner ported into argot-core, full bench)

The sweep above is a Python-`ast` proxy on random 70/30 splits. To validate the floor on real
infrastructure, the winning formulation was ported into argot-core as a feature-gated
(`--features structural`), **non-gating, pure-Rust** sense
(`crates/argot-core/src/scoring/structural.rs`) — node-kind bigram vocabulary + the
`bg_df`-prior fire rule, domain-blind, language-agnostic across all 11 argot grammars. A
self-contained bench mode (`argot-bench --mode structural`,
`crates/argot-bench/src/structural.rs`; `just bench-structural`) then measured, over **all 30
corpora / 11 languages**, using **real tree-sitter extraction** and a **real temporal holdout**:
fit the structural vocabulary at `HEAD~window`, and reuse each corpus's post-fit clean-commit
added hunks as the unit — `over-fire(C)` = fire-rate of C's own clean hunks vs C's fit-SHA vocab;
`catch(A←B)` = fire-rate of B's clean hunks vs A's vocab (same language). Prior = per-language
leave-one-out repo document-frequency. Operating point τ=0.5, k∈{1,2,3}.

**Result — the floor holds, harder, on real infra.** No `k` yields over-fire ≤5% on every corpus:

| | k=1 | k=2 | k=3 |
|---|---:|---:|---:|
| mean over-fire | 19.6% | 7.0% | — |
| **worst-corpus over-fire** | **96.6%** (composer) | **68.9%** (jellyfin) | **22.8%** (jellyfin) |
| mean catch | 24.4% | 8.9% | — |

- **Catch and over-fire are inseparable** — they rise and fall together across every language:
  composer 98% catch / 97% over-fire; express 78% / 46%; and at the quiet end hugo 0.9% / 0.0%,
  laravel 0% / 0%, saleor 2.3% / 0.9%. There is no corpus with useful catch at low over-fire.
- **Wild corpus-dependence confirms the "voice needs a corpus" law on real data.** Mature/stable
  repos are quiet-but-low-catch (hugo, laravel, eslint 0.1%, outline 0.4%, rich 0.6%, saleor
  0.9%, wagtail 0.8%, homebrew 0.5%); young/small/fast-refactoring repos explode (composer 96.6%,
  jellyfin 81%, faker-js 62%, hono 52%, commander/express 46%, ripgrep 39%, curl 34%, guava 31%).
  The composer 96.6%→0.1% collapse from k=1→k=2 is the signature of a thin fit vocabulary: nearly
  every clean hunk has *one* globally-common-but-locally-absent bigram, almost none have two.
- The real temporal holdout is **harsher** than the proxy's random split (worst k=1 over-fire
  9% → 96.6%), because a repo's genuinely-new idiomatic commits routinely use bigrams absent from
  an earlier fit tree. The proxy, if anything, *under*-stated the FP problem.

Caveat, recorded honestly: the most extreme outliers (composer, jellyfin, faker-js) partly
reflect a thin fit vocabulary where `HEAD~window` is a large fraction of the repo's history; but
setting them aside changes nothing — well-populated corpora (commander, express, hono, ink,
ripgrep, curl, guava, rubocop) still over-fire far above 5% at any recall-useful `k`. Raw
per-corpus numbers: `benchmarks/results/structural/structural.{md,json}`.

## Verdict, restated with real-infra evidence

The structural-foreignness gate is **not gatable at argot's FP discipline — proven on a Python
proxy (7 corpora, full lever sweep) and re-proven on real infrastructure (30 corpora, 11
languages, real temporal holdout).** No single (τ, k) reaches ≥85% catch at ≤5% over-fire on
every corpus; the honest recall ceiling at ≤5%-over-fire-everywhere is single digits, and
catch/over-fire are inseparable. Two structural causes stand: foreign idioms overwhelmingly reuse
the repo's own node-kind bigrams, and structural-vocabulary saturation is corpus-size-dependent.

## Decision

- **Ported into argot-core as a feature-gated, NON-GATING, measurement-only sense** — not a gate.
  It is pure-Rust (no new deps), behind `--features structural`, **never wired into the base
  gating exit code**, and **off in the shipped binary** (unlike `semantic`). The base guardrail is
  byte-for-byte unchanged: `just verify` green, and the default build compiles none of it. This
  satisfies "port the winner + clean-commit temporal-holdout over-fire + one full bench" while
  respecting the co-headline FP discipline — the floor is *documented in code + validated*, not
  shipped as a net-negative gate.
- **Do not gate on it, and do not enable it in releases.** An 8–25%-recall signal whose over-fire
  reaches 30–97% on ordinary corpora would violate the co-headline and the settled
  no-net-negative-structural-signal rule. Left as a measurement harness for any future revisit.
- **Advisory variant** (surface the single most-foreign hunk as a non-gating finding, like the
  semantic layer) is *possible* on mature repos only and duplicates the semantic layer's advisory
  role — left as an open product question for the maintainer, not built.

**Reproduce:** real-infra — `just bench-structural` (all corpora) or
`./target/release/argot-bench --mode structural --corpus rich,faker` (scoped, after
`cargo build --release -p argot-bench --features structural`). Proxy — `python
benchmarks/struct_gate_union.py` (headline floor) and `benchmarks/struct_gate_kcross.py`
(cross-family catch). Intermediate exploratory probes are recorded in the tables above; their
scripts were removed per the research-workflow (evidence survives, scripts don't).
