# Foreign-pattern classification: decoupled to authored per-fixture `class`

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · scope: `benchmarks/catalogs/*/manifest.yaml` (per-fixture `class:`), `crates/argot-bench/src/{catalog,production,run,dashboard}.rs`, `benchmarks/foreign_consolidate.py`. Supersedes the mechanism of `foreign-concurrency-reclassification.md` (its classification decisions still stand).

## The problem (measured)

The bench scores 968 fixtures against **5 canonical RUBRIC classes** (`foreign_import`,
`foreign_api`, `foreign_concurrency` = gated; `naming_shape_break`,
`semantic_convention` = secondary), but the catalogs use **59 category names** — 54 of
them free-text (`jquery`, `xhr_network`, `vue_idioms`, `raw_sql`, …). The 59→5 mapping
was done by **name heuristics in two divergent places**:

- Python `norm_class` (drives the landing): a greedy `else → foreign_api` fallback.
- Rust `production::tier_of` + a hardcoded legacy allow-list in `is_novel_pattern`
  (drives the CI dashboard): unknown → `"other"`.

Consequences, all confirmed against the data:
- **Coupled + non-general:** 38 of 54 descriptive categories are used by **one corpus**.
  A new corpus's invented names fall through the heuristics and silently mis-gate.
- **Divergent:** the two classifiers disagreed on unknowns (Python gated them, Rust
  dropped them; Rust even listed `xhr_network`, a raw builtin, as novel-pattern).
- **Per-corpus-ambiguous, so no global table can be right:** `raw_sql` is `psycopg2`
  (foreign) in wagtail but hand-built SQL strings (builtin) in outline; `framework_swap`
  is Flask (foreign) in wagtail but JS-posing-as-TS (naming) in dagster — even within one
  corpus (dagster `framework_swap_4`).

## The fix: author the class on each fixture; code reads it

The gated-vs-secondary decision is a property of the fixture (does it introduce a
0-usage foreign symbol?) and only its author knows it — so it now lives **on the
fixture**, not in shared code:

- Every fixture carries an optional **`class:`** (one of the 5 canonical classes). The
  free-text `category:` stays for reporting. When `category` is already canonical,
  `class` is omitted (`class()` falls back to it). 259 descriptive fixtures got an
  explicit `class:` (per-fixture, so the `raw_sql`/`framework_swap` ambiguity and the
  `framework_swap_4` outlier resolve correctly).
- **Rust `catalog.rs`** validates `class() ∈ CANONICAL_CLASSES` at load — a descriptive
  category with no `class:` is a **hard error**, so a new mislabel can never silently
  gate. `tier_of`/`is_novel_pattern` collapse to a direct function of the class (the
  legacy allow-list is deleted); the bench writes `class` into every result.
- **Python `foreign_consolidate.py`** reads the authored `class` (from the result, or a
  lazy manifest lookup for pre-refactor result files). All name heuristics deleted.

No code in either language now carries a corpus's category vocabulary. Adding a corpus
requires zero shared-code edits — the author labels each fixture's class, and the load
check enforces it.

## Numbers unchanged in intent, more precise in fact

The decoupling is not a number move — it re-homes the *same* classification. Gated
visible aggregates: **import 99.5% · api 94.9% · concurrency 100%.** (api is 94.9 vs the
94.2 of the interim category-set because per-fixture resolution correctly pulls dagster's
JS-as-TS `framework_swap_4` and outline's string-SQL `raw_sql` out of gated — the
category-level set couldn't split them.)

**The real gaps stay visible, by construction:** dagster (70%) and outline (80%) still
sit below 85 on `foreign_api` — their genuine foreign misses (React-Router `<Switch>`,
react-redux `connect`, MobX, styled-jsx via JSX) remain gated. A relabel that hid
weaknesses would have cleared them; this doesn't. The residual is the one real
call-receiver recall question (a foreign symbol reached via JSX or a collided receiver),
documented and left for a base-scorer investigation, not chased here.

Base guardrail untouched (argot-core/cli unchanged); bench + consolidation + manifests
only. `just verify` green; argot-bench clippy-clean.

## Follow-on: `foreign_concurrency` folded into `foreign_import` (5 → 4 classes)

Surfacing the masked tier per-corpus made a second thing obvious: gated
`foreign_concurrency` was a **flat 100% on all 31 corpora** — because a foreign
concurrency lib is a foreign *import* (`import trio`, `#include <tbb>`,
`using Akka`), caught by the same import stage that makes `foreign_import` ~100%.
It measured nothing distinct from import (the only gated class with real spread is
`foreign_api`, 70–100%, the callee path). So the class was folded: the 194
concurrency fixtures now carry `class: foreign_import` (descriptive
`category: foreign_concurrency` kept, so the concurrency slice stays recoverable),
`CANONICAL_CLASSES` drops to 4, and the landing shows **two** gated columns —
*foreign import / dep* (99.7% visible) and *foreign API* (94.9%) — under one
headline, **foreign-pattern catch ≈ 98% visible** (argot's one job). The masked
(hard) tier is surfaced on every cell so no column reads as a clean 100% while its
hard cases miss (import/dep ~22%, API ~20% masked). RUBRIC.md updated.
