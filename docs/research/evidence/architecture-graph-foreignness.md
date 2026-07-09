# Architecture-graph foreignness: a discrete, low-FP "has no place here" signal

> **⚠️ CORRECTION (2026-07-09, later) — the headline catch numbers below are SUPERSEDED.**
> A host-backed re-measurement found the published catch (85–90%) was **coverage-inflated**:
> the coverage loop counted `(a→b)` pairs whose SOURCE layer `a` is a *target-only namespace
> layer no file lives in* — violations no real hunk can introduce. Restricting the coverage to
> SOURCE layers that map to a real HEAD file (the *authorable* space) drops mean catch to
> **~52%** (10 corpora, one per language). Two further findings: (1) the Python fixtures that
> gave "84% real recall" were authored with the OLD `py_file_edges` resolver, which *missed*
> relative/grouped cross-package imports; the more-correct multi-language resolver
> (`py_targets`) resolves them, so those edges are attested and the fixtures are now **invalid**
> (saleor 10/12) — the same text-grep gap that invalidated the multi-lang fixtures, now known to
> hit Python too. (2) Rust (`ripgrep` 3/55 host-mapped) and C# (`powershell` 54/91 but
> source/target vocabularies disjoint) read artificially-low catch because their layer
> assignment splits (layer ≠ directory) — a resolver bug, not signal absence.
> **What HELD:** over-fire ≤2.7%/corpus (0.49% agg, 2656 commits) and control-FP **0/25 = 0%**.
> The low-false-positive property is real and robust; the *catch* is modest and uneven.
> See the "Host-backed re-measurement" section at the bottom for the corrected table + diagnosis.

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · status: **VALIDATED — ported
non-gating into argot-core; real-holdout over-fire ≤2.7% on 2690 commits, mean catch 85% out-of-box / 88% with realistic
per-corpus argot.toml (coverage, mute-system voice files, NO hardcoded excludes) — ≥85/≤5 met.
Catch-on-injected-fixtures + multi-lang resolvers remain.** Harnesses:
`benchmarks/arch_graph_{probe,xlang,temporal}.py` + `argot-bench --mode arch`. Opened after the
node-kind n-gram *shape* gate hit an irreducible floor
([`foreign-structure-gate-floor.md`](foreign-structure-gate-floor.md)).

## The idea

The base gate catches a foreign **dependency** (an external import 0-usage in the repo). This
catches a foreign **relationship**: an *internal* module-dependency edge the repo's own topology
never has — a layer it never crosses, or a dependency **direction** it never uses (a `models/`
file importing `views/`). Two properties make it attractive where the shape gate failed:

- **Discrete, high-information** — an edge either crosses a boundary / reverses a direction or it
  does not, exactly the property that makes the import gate ~98%. (Node-kind n-grams are
  continuous and low-information; that is why they diluted to an 8–13% ceiling.)
- **Invisible to the base gate** — the imported module is the repo's *own* code, so vocabulary
  detection sees nothing. Non-overlapping signal.

Domain-blind: "layer" = the path component under a package root (never a hardcoded layer name).
Internal edges only (external deps are the base gate's job).

## Method (cheap, no scorer change)

Per corpus: detect package roots (any dir with `__init__.py` whose parent has none; skip
test/example/infra trees), map every file to its layer, resolve each internal import to a target
layer, and build the weighted cross-layer edge graph. Then:

- **Q1 signal exists?** directional asymmetry `asym%` = share of cross-layer edge-mass in the
  dominant direction of each pair. High ⇒ a near-DAG layering to violate.
- **Q2 FP** (over-fire proxy, 70/30 file split): of held-out files' cross-layer edges, the share
  that are the **clean tell** — `reversal` (novel edge whose reverse is attested) ∪ `sink-out`
  (novel edge from a fit-graph *sink* layer: imported-but-never-imports). "Any novel edge"
  (`novel%`) is the noisy upper bound (organic growth adds new edges constantly).
- **catch** (coverage estimate): of cross-layer edges the repo does *not* have, the share the
  reversal∪sink rule would flag if an LLM created one (uniform-violation model).

## Result

| corpus | layers | x-edges | asym% | novel% | **FP%** (clean tell) | **catch%** (est.) |
|---|--:|--:|--:|--:|--:|--:|
| scrapy | 36 | 122 | 95% | 13.8% | 0.7% | 68% |
| rich | 70 | 69 | 100% | 14.7% | 0.0% | 100% |
| faker | 14 | 23 | 100% | 9.5% | 0.0% | 77% |
| fastapi | 32 | 50 | 98% | 35.7% | 0.0% | 67% |
| wagtail | 38 | 158 | 94% | 17.0% | 2.1% | 56% |
| saleor | 34 | 298 | 90% | 7.1% | 3.1% | 40% |
| dagster | 239 | 828 | 95% | 10.8% | 2.5% | 54% |
| **mean** | | | **95%** | 15.5% | **1.2%** | **66%** |

- **Signal exists, strongly:** module graphs are 90–100% directional — real layering to violate,
  unlike the node-kind alphabet which had no clean discrete tell.
- **FP is low and gatable:** the clean tell (reversal ∪ sink-out) over-fires **≤3.1% on every
  corpus** (mean 1.2%), well inside the ≤5% budget. Firing on *any* novel edge (`novel%` up to
  36%) would not be gatable — the direction/sink discrimination is what makes it clean.
- **Catch is high (estimated ~66%)** at that FP — a different universe from the shape gate's
  8–13%. Even the weakest corpus (saleor, a big flat app with looser layering) is 40% catch at
  3.1% FP.

## Honest limits (what the cheap probe does NOT yet show)

1. **Catch is a coverage model, not measured recall.** It assumes a violation is uniform over the
   repo's missing cross-layer edges; real LLM violations cluster on plausible-but-wrong targets.
   Needs authored/synthetic architectural-violation fixtures (the bench way) for a true recall.
2. **FP is a 70/30 file split, not a real temporal holdout.** The honest over-fire is on the
   repo's own future *commits*; needs the real fit-at-`HEAD~window` + replay measure (as built
   for the structural sense in `argot-bench --mode structural`).
3. **Python only.** Import resolution differs per language (JS/TS re-exports, Go packages, Java
   packages, C includes). Cross-language validation required before any claim of generality.
4. **Coverage gap by construction:** the rule catches reversal/sink violations; a *novel-forward*
   edge (a new edge in a legal-ish direction from a non-sink layer) is not cleanly gatable — that
   is the ceiling on catch, and why catch is ~66% not ~100%.

## Why this is the right lever (vs the closed shape gate)

The shape gate failed because node-kind n-grams are the weakest representation of foreignness —
continuous, low-information, and every foreign snippet is built from shapes the repo also uses.
The signal that code "has no place here" is **relational**: it lives in the *edges* between
modules, not the *shapes* within a function. A dependency-direction reversal is discrete and rare
in healthy code (≤3%), so it can gate — the same reason a foreign import can.

## De-risk round (cheap, before the full build) — both unknowns held

**1. Realistic catch model (`arch_graph_probe.py`).** Replaced the uniform-over-non-edges
assumption with a realistic one: an LLM reaches for a *real* internal module (one some layer
already imports), weighted by that module's popularity (import mass), from a layer the repo
forbids. Catch **rose** to **76% mean (57–100%)** at the same ~1.2% FP — realistic violations
target popular modules, which have the strongest directional structure, so they are *more*
catchable than random edges.

**2. Cross-language spot-check (`arch_graph_xlang.py`, heuristic regex extractors).**

| corpus | lang | layers | asym% | FP% | catch% |
|---|---|--:|--:|--:|--:|
| hugo | go | 38 | 93% | 2.4% | 43% |
| gh-cli | go | 7 | 97% | 0.0% | 57% |
| hono | ts | 21 | 99% | 1.6% | 57% |
| excalidraw | ts | 110 | 97% | 1.3% | 91% |
| outline | ts | 5 | 100% | 0.0% | 14%* |

The two make-or-break properties **generalize**: module graphs are **93–100% directional** in Go
and TypeScript, and the reversal∪sink tell stays **≤2.4% FP**. Catch is more extraction-sensitive
(*outline resolved only 4 edges — a heuristic-parser artifact, not a signal failure; the real
per-language extractor via `import_graph.rs` will do better). Verdict: the cheap probe cleared the
bar to justify a full validation.

## Decisive: REAL temporal-holdout FP (not a file split) — `arch_graph_temporal.py`

The clincher. Fit the layer graph at `HEAD~150`, replay every non-merge commit after it,
attribute the edges each commit *adds* (file edges at `sha` minus at `sha^`), count those that
are the clean tell (reversal ∪ sink) vs the fit graph. Commit-level over-fire = the honest
false-alarm rate a maintainer feels:

| corpus | real commits | clean-tell fires | over-fire% |
|---|--:|--:|--:|
| scrapy | 200 | 2 | 1.0% |
| rich | 305 | 0 | 0.0% |
| faker | 150 | 0 | 0.0% |
| fastapi | 150 | 0 | 0.0% |
| wagtail | 150 | 0 | 0.0% |
| saleor | 151 | 3 | 2.0% |
| **total** | **1106** | **5** | **~0.5%** |

**The low FP holds under a real temporal holdout** — 0–2% per corpus, ~0.5% aggregate over 1106
actual clean commits. The file-split proxy (~1.2%) was accurate, not optimistic. This is a
genuinely gatable false-alarm profile — the property the node-kind shape gate never had (30–97%).

## Standing evidence summary

| property | shape gate (closed) | **architecture graph** |
|---|---|---|
| signal exists | weak (small alphabet) | **strong — 90–100% directional, 3 langs** |
| real-holdout FP | 30–97% | **0–2% (~0.5% agg over 1106 commits)** |
| catch @ that FP | 8–13% | **~76% (realistic coverage model)** |
| generalizes | — | **Python + Go + TS confirmed** |

The one number still estimated (not measured) is **catch/recall on real violations** — the
coverage model, not injected fixtures. That is what the full Rust port + bench measures next.

## Built + validated through the real argot-core module (`--mode arch`)

The winning formulation is now ported into argot-core as a feature-gated (`--features arch`),
pure-Rust, **non-gating** sense (`crates/argot-core/src/scoring/arch_graph.rs`: `RepoLayering`
fit + the reversal/sink `classify`), and a self-contained bench mode
(`crates/argot-bench/src/arch.rs`, `argot-bench --mode arch`) validates it over real corpora with
a **real git temporal holdout** — the same rigour as the base metric, driven through the real
module (not a Python proxy). v1 resolves Python imports; other languages are a graceful no-op.

Two file-collection scopes, both via the mute system (no hardcoded excludes):
**out-of-box** (recommended defaults only) → mean catch **85%**; **realistic setup** (a per-corpus
`argot.toml` excluding peripheral trees a maintainer would mute — docs_src/examples/etc., authored
per the `argot-setup` skill, in `benchmarks/catalogs/*/argot.toml`) → mean catch **88%**. Over-fire
is identical either way (0.26% agg, ≤2.7% worst). The realistic-setup table:

| corpus | layers | edges | real commits | fires | over-fire | catch |
|---|--:|--:|--:|--:|--:|--:|
| fastapi | 24 | 48 | 1200 | 2 | 0.2% | 94% |
| rich | 70 | 70 | 360 | 0 | 0.0% | 99% |
| faker | 14 | 23 | 150 | 0 | 0.0% | 82% |
| dagster (multi monorepo) | 235 | 812 | 150 | 1 | 0.7% | 71% |
| saleor | 35 | 306 | 150 | 4 | 2.7% | 88% |
| wagtail | 40 | 162 | 250 | 0 | 0.0% | 80% |
| rocksdb | 9 | 8 | 150 | 0 | 0.0% | 100% |
| scrapy | 36 | 123 | 280 | 0 | 0.0% | 90% |
| **total / mean** | | | **2690** | **7** | **0.26% agg (≤2.7% worst)** | **88%** |

**≥85% catch at ≤5% over-fire — met.** Mean catch **85% out-of-box / 88% realistic-setup**,
over-fire ≤2.7% on every corpus (0.26% aggregate over 2690 real clean commits), through the real
Rust module, **voice files collected by the mute system — no hardcoded path exclusions anywhere.**
Excluding only genuinely-peripheral trees (`fastapi/docs_src` 73→94; dagster's examples/integration/
helm 70→71; wagtail's client/docs 80→80 — the latter two barely move, honestly, because they are
JS/non-Python or the missing-edge space is legit foundational imports). Two tuning steps got the
base rule from 77% → here, with FP essentially unchanged (the huge headroom — 0.26% agg vs the 5%
budget — is the lever):
1. **Near-sink generalization** (`NEAR_SINK_RATIO = 0.5`): a *net-importee* layer (imported at
   least as much as it imports out — not only a strict out-degree-0 sink) importing outward is the
   tell. Real over-fire is **flat** across ratios 0.25→0.5 (aggregate 0.45%→0.54%) because the
   repo's own commits don't create these edges regardless of threshold — so the catch gain is free.
   Ratios > 0.5 flag net-*exporters* (app layers) — over-aggressive, so 0.5 is the boundary. The
   rank-gradient lever was tried and dropped (near-sink subsumes it; it *hurt* popular targets).
2. **Catch on the HEAD (production) graph** (not the thin holdout fit-SHA that understates coverage).

**Honesty note (why not 90%).** An earlier pass hit 90% by hardcoding a `starts_with("doc")`
scaffolding filter in the code — which *excludes fastapi's `docs_src/` tutorial tree that production
actually includes*, inflating the number. Removed: exclusions now come only from the mute system
(`EXCLUDE_DIRS` + `.argotignore` + `[exclude].paths`), and the honest out-of-box mean is **85%**.
The three sub-85 corpora are honest: **fastapi 73%** is dragged by `docs_src/` (tutorial code a real
maintainer would `.argotignore` — doing so via config, not code, lifts it to ~94%); **dagster 70%**
(299-layer monorepo) and **wagtail 80%** are large, deeply-layered repos where much of the
"missing edge" space is *legitimate* foundational imports the rule correctly leaves alone (the
coverage metric understates true violation-recall there).

## REAL measured recall (authored 0-usage fixtures, not the coverage estimate)

The coverage number is an estimate. To measure real recall the way the base gate earns its 98%,
three parallel agents authored **architectural-violation fixtures** — a real import line added to a
real host file, creating a cross-layer edge **verified 0-usage** at HEAD (saleor/scrapy/wagtail,
~12 violations + 5 clean controls each, `benchmarks/catalogs/*/arch_violations.yaml`). The bench
scores each through the **real resolver** (`file_edges` → `classify`) on the HEAD graph:

| corpus | valid violations caught | control-FP | note |
|---|---|---|---|
| saleor | 2/2 = 100% | 0/5 | 10/12 authored "violations" were edges saleor **already has** (loose layering + relative imports) → correctly not fired |
| scrapy | 9/11 = 82% | 0/5 | 1 invalid, 2 novel-forward misses |
| wagtail | 10/12 = 83% | 0/5 | `models→admin` is *deliberately* avoided (circular-import comments) — genuinely 0-usage |
| **total** | **21/25 = 84%** | **0/15 = 0%** | real recall on genuinely-0-usage violations |

**84% real recall, 0% control false-positives** — measured, not estimated. The 4 misses are
novel-*forward* edges (cross a boundary but neither reverse nor leave a sink — the honest ceiling;
some are arguably legit). The 11 "invalid" fixtures are a *feature*: hand-picked "obvious
violations" that the repo actually has as dependencies, which the detector correctly ignores — the
same discrimination that gives it 0% control-FP. This replaces the coverage estimate as the
headline catch number and is measured with the same rigour as the base gate's fixtures.

## Multi-language: all 11 resolvers + full bench (25/31 corpora)

The architecture-graph is now language-agnostic (per-language resolver behind a shared
`detect_context` + `<lang>_targets` seam): Python, Go, TypeScript/JS, Rust, Java, PHP, C#, Ruby,
C/C++. Full `argot-bench --mode arch` over every corpus (real temporal holdout):

**Over-fire is uniformly excellent across every language — 0.25% aggregate over 6343 real commits,
≤2.7% worst per corpus.** The FP discipline holds regardless of language. Coverage tracks how
*layered* the repo is:

| tier | corpora (catch coverage) |
|---|---|
| strong | ripgrep 98% · bat 97% (rust) · rich 99 · fastapi 94 · scrapy 90 · saleor 88 (py) · curl 95 (c) · composer 84 (php) · rubocop 84 (ruby) · eslint 84 (js) · rocksdb 81 (c++) · faker-js 79 (ts) · gh-cli 75 (go) |
| moderate | hugo 68 · excalidraw 70 · redis 86(small) · hono 48 · laravel 24 |
| weak/flat (few layers) | ink · outline · fmt · commander · express — small/flat repos with little layering to violate (honest) |

**Gap found (the "evaluate" step): 5 corpora produced NO edges** — guava/junit5 (Java),
powershell/jellyfin (C#), homebrew (Ruby):
- **Java/C# — a real multi-module bug.** guava has *two* src trees (`android/guava/src/…` and
  `guava/src/…`); the base-package detection ("longest common package prefix") collapses to empty,
  so no layers form. Fix: detect the base package **per src-root**, not globally.
- **Ruby — inherent.** Zeitwerk/Rails autoloading means few explicit `require`s (rubocop, which
  uses requires, works at 84%; homebrew, autoloaded, is empty). Honest weak spot.

## Multi-language fixtures — a methodology finding (the "evaluate" step)

Authored violation fixtures for composer (PHP), eslint (JS), ripgrep (Rust) — same method as the
Python ones (0-usage verified by `git grep`). Scored through the real resolver:

| corpus | lang | valid violations | control-FP | note |
|---|---|---|---|---|
| composer | php | 0/10 (10 invalid) | 0/5 | every authored "violation" edge already exists |
| eslint | js | 0/10 (10 invalid) | 0/5 | " |
| ripgrep | rust | 0-caught/1 valid (9 invalid) | 0/5 | " |

**Finding:** text `git grep` verification does **not** match the resolver's edge detection — the
resolver counts edges from **relative imports** (`use super::`, `../x`) and **grouped imports**
(`use crate::{a, b}`, `use App\{A, B}`) that the authors' absolute greps miss, so the "0-usage"
edges are actually attested. (Same cause as saleor's 10/12 invalid.) So **real recall is measured
only on Python (84%)**; PHP/JS/Rust fixtures need **resolver-grounded 0-usage verification** (dump
the resolver's actual 0-usage reversible/sink edges, author against those) — a bench helper to add.

**What IS validated multi-language:** coverage (composer 84 · eslint 84 · ripgrep 98) and,
importantly, **control-FP = 0/30 = 0% across PHP/JS/Rust/Python** — the detector correctly stays
silent on legit new edges in every language (the no-false-positive property generalizes).

**Also still open:** the Java/C# multi-module resolver fix (an agent attempted it but its changes
did not persist) and the weak-corpus investigation (laravel/hono).

## Status + honest remaining work

**Validated:** the signal exists (90–100% directional, 3 langs), real-holdout FP is 0–2.7% (proxy +
real Rust module, 2690 commits), catch **90%** (coverage, ≥85% met). Ported non-gating into
argot-core; base byte-for-byte unchanged, `just verify` green.

**Done since:**
1. **Real recall measured** (84%, 0% control-FP — see the section above) — replaces the coverage
   estimate.
2. **Wired end-to-end** (`--features arch`): `argot fit` persists `.argot/layering.json` (built
   from the mute-system voice files); `argot check` emits a `layering` finding (advisory
   "unusual" tier, honors inline/mute suppression) for any added import that creates a
   reversal/near-sink edge. Verified on a synthetic layered repo (reversal fires, legit edge quiet).
   Base byte-for-byte unchanged (all `#[cfg(arch)]`; parity suites green).

**Remaining before public release:**
1. **v1 is Python-resolver only.** Go/TS/Java resolvers (validated by the cheap probe) plug into
   `RepoLayering::file_edges`; C/C++/others follow.
2. **Broaden the fixture set** (more corpora/languages) to tighten the 84% recall CI.
3. **Turn the feature on in releases** + publish the benchmark to the landing (deferred until
   multi-language + broader fixtures, per the earlier honesty call).

**Reproduce:** real module — `cargo build --release -p argot-bench --features arch &&
./target/release/argot-bench --mode arch`. Cheap probes —
`python benchmarks/arch_graph_{probe,xlang,temporal}.py`.

---

## Host-backed re-measurement — the honest catch (2026-07-09, supersedes the above)

The catch numbers above are **coverage over ALL layer-pairs**. Building the resolver-grounded
0-usage candidate dumper (`argot-bench --mode arch-candidates`) surfaced the flaw: the coverage
loop iterates `(a, b)` over *every* layer in the graph, but a real hunk can only introduce an
edge **from a file that exists** — i.e. from a SOURCE layer that maps to a real source file.
Many layers in `graph.layers()` are *target-only* (a namespace/vendored package some file
imports, but no source file lives in). Counting `(target-only-a → b)` pairs inflates catch,
badly where the resolver splits source and target vocabularies (layer ≠ directory).

**Fix:** restrict the coverage numerator/denominator to SOURCE layers that map to a real HEAD
file (`host_layers`), computed while the tree is at HEAD (before the holdout `graph_at(fit_sha)`
moves it). The `layers` column is now `host-mapped / total`.

| corpus | lang | layers (host/total) | edges | over-fire | **catch (host-backed)** | recall(fixtures) | ctrl-FP |
|---|---|--:|--:|--:|--:|--:|--:|
| saleor | python | 30/35 | 306 | 2.7% | 84% | 2/2 (10 **invalid**) | 0/5 |
| scrapy | python | 15/36 | 123 | 0.0% | 41% | 9/11 = 82% (1 inv) | 0/5 |
| wagtail | python | 23/40 | 162 | 0.0% | 46% | 10/12 = 83% | 0/5 |
| composer | php | 25/36 | 228 | 1.8% | 69% | — | 0/5 |
| ripgrep | rust | **3/55** | 57 | 0.7% | 50% | 0/1 (9 inv) | 0/5 |
| guava | java | **7/39** | 60 | 0.0% | 19% | — | — |
| powershell | csharp | 54/91† | 570 | 0.0% | **0%** | — | — |
| rubocop | ruby | 5/5 | 7 | 0.0% | 84% | — | — |
| gh-cli | go | 9/9 | 21 | 1.1% | 66% | — | — |
| excalidraw | typescript | 19/20 | 52 | 1.3% | 57% | — | — |
| **mean** | | | | **0.7%** | **~52%** | 21/26 = 81%‡ | **0/25 = 0%** |

holdout 13/2656 commits = **0.49%** (worst 2.7%).

† powershell is host-mapped for 54/91 layers, yet catch is 0: its .NET flat-dotted directories
(`src/Microsoft.Management.Infrastructure.CimCmdlets/` — one dir, dotted name) make
`namespace_source_root` overshoot, collapsing *every* source file to layer `src`; the source
layer `src` is not a sink and nothing imports it, so no host-mapped layer participates in a
reversal/sink. ‡ the fixture recall is on a badly-thinned valid set (saleor 10/12 now invalid) —
not a reliable number until fixtures are re-authored against the dumper.

### What this changes

1. **FP discipline is the real, robust win** — over-fire ≤2.7%/corpus (0.49% agg over 2656 real
   clean commits), control-FP **0/25 = 0%** across 6 languages. The reversal/sink discrimination
   genuinely does not false-fire. This is the hard part and it holds honestly.
2. **Catch is modest (~52% mean) and uneven**, NOT the 85–90% previously published — that number
   counted non-authorable target-only pairs. On an honest, authorable-only basis the gate does
   **not** clear ≥85% catch on most corpora.
3. **Two resolver bugs suppress catch** (fixable, filed as work items):
   - **C# (`namespace_source_root`)** collapses flat-dotted .NET projects to one source layer →
     0% authorable catch. Needs namespace-derived source-layer assignment (a fit-time path→layer
     map), consistent with how `cs_targets` derives target layers.
   - **Rust (`ripgrep` 3/55)** — a cargo workspace (`crates/*/src/**`) maps almost no layers to
     files under the current root detection. Same class of layer≠directory split.
4. **Fixtures are stale** — the more-correct resolver invalidated the absolute-grep-authored
   fixtures (Python included). Real recall must be re-measured by authoring fixtures against
   `--mode arch-candidates` (resolver-grounded 0-usage), mixing realistic reversal/sink/forward
   so the number stays non-circular.

### The dumper (`--mode arch-candidates`)

Enumerates every cross-layer non-edge the *real resolver* classifies as reversal/sink-out, keeps
only those with a real host file in the source layer, writes a popularity-sorted menu per corpus
(`candidates-<corpus>.md`) plus `N layers · M host-mapped · K candidates`. This closes the
text-grep gap: fixtures target edges the resolver *actually* sees as 0-usage. Host-backed counts:
composer 418, ripgrep 54, excalidraw 130, hugo 677, guava 42, gh-cli 29, rubocop 11, outline 4,
**powershell 0** (the C# collapse).

**Verdict for the "evaluate" step:** the architecture-graph is a **genuinely low-false-positive**
signal (its defensible strength) with **modest, uneven catch** that is partly suppressed by two
fixable resolver layer-assignment bugs and cannot be honestly quantified until fixtures are
re-authored against the dumper. It is not, on honest measurement, the "≥85/≤5 gatable win" the
pre-correction sections claim.

## After the resolver fixes (2026-07-09) — C# + Rust layer assignment

The two layer-assignment bugs are fixed (`fix(arch): namespace/module-derived source layer`):

- **C#** — the source layer now comes from the file's *namespace* (via the same `layer_after_base`
  used for targets), stored in a fit-time file→layer map (persisted; new files parse the hunk).
  Flat-dotted .NET directories no longer collapse to `src`. Regression test:
  `csharp_flat_dotted_dirs_use_namespace_layer_not_directory`.
- **Rust** — a *file* module directly under `src` (`src/color.rs`) is layer `color`, matching the
  `use crate::color::…` target vocabulary, not `__root__`. Regression test:
  `rust_file_modules_use_stem_layer_not_root`.

Re-measured (12 corpora; `layers` = host-mapped-in-graph / total after the display fix):

| corpus | lang | layers | edges | over-fire | **catch (host-backed)** | recall(fixtures) | ctrl-FP |
|---|---|--:|--:|--:|--:|--:|--:|
| saleor | python | 30/35 | 306 | 2.7% | 84% | 2/2 (10 inv)* | 0/5 |
| scrapy | python | 15/36 | 123 | 0.0% | 41% | 9/11 = 82% | 0/5 |
| wagtail | python | 22/40 | 162 | 0.0% | 46% | 10/12 = 83% | 0/5 |
| composer | php | 25/36 | 228 | 1.8% | 69% | — | 0/5 |
| **ripgrep** | rust | **41/56** | 107 | 0.0% | **62%** | **9/10 = 90%** | 0/5 |
| **bat** | rust | 24/32 | 72 | 0.0% | **68%** | — | — |
| guava | java | 7/39 | 60 | 0.0% | 19% | — | — |
| **powershell** | csharp | 5/38 | 90 | 0.0% | **37%** | — | — |
| **jellyfin** | csharp | 17/18 | 81 | 0.0% | **73%** | — | — |
| rubocop | ruby | 5/5 | 7 | 0.0% | 84% | — | — |
| gh-cli | go | 8/9 | 21 | 1.1% | 66% | — | — |
| excalidraw | typescript | 17/20 | 52 | 1.3% | 57% | — | — |
| **mean** | | | | **0.6%** | **~59%** | 30/35 = 86% | **0/25 = 0%** |

holdout 12/3309 commits = **0.36%** (worst 2.7%). *saleor's Python fixtures are still the stale
absolute-grep set (10/12 attested); Rust's ripgrep fixtures became genuinely valid once the vocab
aligned → real 90% recall — a second measured language.

**What the fixes bought:** mean catch **52% → 59%**; Rust from a broken 3/55-host-mapped 50% to
**43/56, 62–68% catch and 90% real fixture recall**; C# from a hard **0% to 37–73%**. Over-fire and
control-FP are unchanged (0.6% mean, 0% control-FP) — the fixes only recovered suppressed catch,
they did not touch the false-positive profile. **Remaining:** (1) re-author the Python fixtures
against the dumper (task #18) for a refreshed Python recall — the coverage suggests ~high-40s to
80s depending on how loosely layered the repo is; (2) **guava/Java 19%** (7/39 host-mapped) looks
like the *same* class of layer-assignment issue — Java's self-layer could also be package-derived;
worth a look. Honest headline now: **strong low-FP (0% control-FP, ≤2.7% over-fire), ~59% mean
authorable catch, 90% measured recall on the one re-validated non-Python language (Rust).**
