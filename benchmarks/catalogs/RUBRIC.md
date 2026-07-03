# Break-fixture rubric (issue #92 · v2 foreign-dependency scope)

This rubric is fixed **before** any fixture is scored. A fixture that fails to
fire is a *finding to report*, never a reason to swap the fixture or the corpus.
Amendments require a recorded rationale in `docs/research/evidence/` and
re-scoring of every existing fixture.

## Product scope (v2, 2026-07-03)

argot is a **foreign-dependency / foreign-API linter**: it flags code that
reaches for a dependency, API, or library **that is not the repo's own voice**
(verified 0-usage at the pinned SHA). It is *not* a semantic reasoner — it does
not judge whether the repo's *own* attested vocabulary is used in a subtly wrong
way. The catalog **gates** on the first capability and **reports** the rest, so
the headline reflects what argot actually ships. Rationale + evidence:
`docs/research/evidence/issue92-investigation-capstone.md` (and the 8 scouts it
links). This supersedes the v1 five-class rubric; every retained fixture is
re-scored, none is trivialised, and the honest temporal-holdout FP gate is
unchanged (the safeguard that keeps "green" meaning it works on real PRs).

## Class distribution (≥ 12 fixtures / language)

| Class | Count | Tier | What it is |
|---|---|---|---|
| `foreign_import` | ≥ 3 | **gated** | A dependency the repo does not import (0-usage at the pinned SHA): a foreign package `use`/`import`/`require`/`#include`. The import stage catches this by design. |
| `foreign_api` | ≥ 3 | **gated** | A call into a **foreign library's** API — the hunk references a callee/symbol that is 0-usage in the repo (a foreign HTTP client, DB driver, serializer, logger, template engine) where the repo standardises on its own. The call-receiver stage catches the unattested callee. |
| `foreign_concurrency` | ≥ 2 | **gated** | A **foreign concurrency library/runtime** the repo does not use (a foreign thread pool, async runtime, parallel/coroutine lib) — an unattested foreign callee, not a raw language builtin. |
| `naming_shape_break` | ≥ 2 | *reported (best-effort)* | Identifier morphology foreign to the repo (camelCase in a snake_case repo, Hungarian). Signal quality depends on the repo's morphology purity; reported, not gated. |
| `semantic_convention` | ≥ 2 | *reported (out-of-scope)* | Misuse of the repo's **own / attested** vocabulary: a language builtin the repo avoids (`die`/`exit`/`errno`), a wrong-value on an attested construct (`E_USER_ERROR` where `trigger_error` is attested), or a deprecated API of an **already-imported** lib. Needs semantic reasoning argot categorically lacks — a documented fundamental limit. |

**Gate:** recall **≥ 85%** over the three **gated** foreign-symbol classes
(`foreign_import` + `foreign_api` + `foreign_concurrency`). `naming_shape_break`
and `semantic_convention` are reported with their own numbers, never gated.

The distinction is mechanical and checkable from the break's construction: a
**gated** fixture's tell is a symbol (import or callee) verified **0-usage in the
repo at the pinned SHA** — genuinely foreign vocabulary. A `semantic_convention`
fixture's tell is a construct the repo's own vocabulary already contains, used
wrongly. This is *not* an easier test — a foreign-dependency break is a real,
corpus-authentic violation (a contributor really does reach for the wrong lib);
it is a *scoped* test, matched to what the product claims.

## Fixture construction rules

1. **Spliced, not whole-file**: every fixture declares `host_file` +
   `host_inject_at_line` into a real corpus file at the pinned SHA. The scored
   hunk is the break body only (`hunk_start_line`..`hunk_end_line`); surrounding
   decoy lines are idiomatic corpus-style code.
2. **Corpus-authentic + verified-foreign**: the foreign dependency/API must be
   plausible for the repo's domain **and** verified 0-usage at the pinned SHA
   (`git grep <term> <sha>`, `git show <sha>:<path>`), recorded in `rationale`.
   A gated fixture whose "foreign" symbol turns out to be used by the repo is
   factually wrong and must be fixed or reclassified.
3. **Compiles/parses in isolation**: the fixture file parses with the language's
   tree-sitter grammar (no placeholder pseudo-code).
4. **No trivialising**: a gated break must be a genuine foreign-dependency
   violation, not a decorative import no contributor would write. Not firing is a
   recall miss to report, not a reason to soften the fixture.
5. **Meta-comments**: `// Break:` / `# Break:` notes are stripped by the harness
   before scoring.

## Measurement

Production path (`argot-bench --mode honest`): fixture planted on disk at the
pinned SHA, staged with real git, judged by `argot fit` + `check --staged` with
the honest (LOO) calibration. Caught = any hit on the host file. Headlines,
reported side by side (none hidden):

1. **Gated recall** (≥ 85%) — foreign-symbol classes; the shippable headline.
2. **Naming recall** (best-effort) — `naming_shape_break`, reported.
3. **Semantic recall** (out-of-scope) — `semantic_convention`, reported as the
   documented fundamental limit.
4. **Temporal-holdout FP** (unchanged gate: existing ≤ 2%, new-file ≤ 5%) — the
   anti-inflation safeguard; "green" recall only counts with FP still honest.
