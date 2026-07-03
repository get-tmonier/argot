# Break-fixture rubric (issue #92 · v2 novel-pattern / LLM-guardrail scope)

This rubric is fixed **before** any fixture is scored. A fixture that fails to
fire is a *finding to report*, never a reason to swap the fixture or the corpus.
Amendments require a recorded rationale in `docs/research/evidence/` and
re-scoring of every existing fixture.

## What argot is for (the north star)

**A guardrail against code that is foreign to your codebase's established
patterns — the "totally unknown to this repo" thing an LLM coding agent
introduces before it lands.** An AI agent that doesn't know your stack reaches
for a dependency, API, or construct your repo has never used; argot flags it at
`check`/pre-commit time, learned from the repo's own voice, zero-config.

That gives argot exactly **one job**, and its scorecard is **two numbers**:

1. **Novel-pattern catch rate** — of code that introduces something foreign to
   the repo (a dependency/API/callee 0-usage at the pinned SHA), what share does
   argot flag? *(The gated headline; ≥ 85%.)*
2. **False-alarm rate** — of real idiomatic commits, what share does argot flag?
   *(Temporal-holdout FP; existing ≤ 2%, new-file ≤ 5%. A guardrail that cries
   wolf is worse than none.)*

Everything else the old rubric measured (does it also catch a misused *builtin*
the repo already has — `die` vs `throw`; or a naming-morphology slip) is
**secondary coverage, not the metric** — reported for interest, never gated. It
proved a fundamental local limit (see the evidence) and, more to the point, is
not the danger an LLM poses: an agent doesn't subtly misuse your own vocabulary,
it drags in a whole foreign pattern. Rationale + full investigation:
`docs/research/evidence/issue92-investigation-capstone.md`.

Rubric discipline is unchanged: a fixture that fails to fire is a finding to
report, never a reason to trivialise it, and every fixture is 0-usage-verified
at the pinned SHA.

## Class distribution (≥ 12 fixtures / language)

| Class | Count | Tier | What it is |
|---|---|---|---|
| `foreign_import` | ≥ 3 | **gated** | A dependency the repo does not import (0-usage at the pinned SHA): a foreign package `use`/`import`/`require`/`#include`. The import stage catches this by design. |
| `foreign_api` | ≥ 3 | **gated** | A call into a **foreign library's** API — the hunk references a callee/symbol that is 0-usage in the repo (a foreign HTTP client, DB driver, serializer, logger, template engine) where the repo standardises on its own. The call-receiver stage catches the unattested callee. |
| `foreign_concurrency` | ≥ 2 | **gated** | A **foreign concurrency library/runtime** the repo does not use (a foreign thread pool, async runtime, parallel/coroutine lib) — an unattested foreign callee, not a raw language builtin. |
| `naming_shape_break` | ≥ 2 | *secondary* | Identifier morphology foreign to the repo (camelCase in a snake_case repo, Hungarian). Reported for interest, never gated. |
| `semantic_convention` | ≥ 2 | *secondary* | Misuse of the repo's **own / attested** vocabulary: a builtin the repo avoids (`die`/`exit`), a wrong value on an attested construct (`E_USER_ERROR`), or a deprecated API of an already-imported lib. A proven local limit; reported, never gated. |

**The metric = the novel-pattern classes** (`foreign_import` + `foreign_api` +
`foreign_concurrency`): each fixture's tell is a symbol (import or callee)
verified **0-usage in the repo at the pinned SHA** — genuinely foreign
vocabulary, exactly the "unknown to this codebase" thing an LLM drags in. Gate:
**catch rate ≥ 85%**, at **false-alarm ≤ 2%** (existing-file temporal-holdout).
The two `secondary` classes are reported but never gated — they are neither the
danger an LLM poses nor reliably local-detectable.

The novel-pattern test is not an *easier* test — a foreign-dependency break is a
real, corpus-authentic violation an AI agent genuinely produces; it is a *scoped*
one, matched to argot's one job.

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
the honest (LOO) calibration. Caught = any hit on the host file. **Two numbers:**

1. **Novel-pattern catch rate** (≥ 85%) — recall over the novel-pattern classes.
   *The headline.*
2. **False-alarm rate** (existing ≤ 2%, new-file ≤ 5%) — temporal-holdout FP on
   real commits (`--mode honest`/`holdout`). *The co-headline — a guardrail is
   only good if it stays quiet on idiomatic code.*

Secondary coverage (`naming_shape_break`, `semantic_convention`) is printed
underneath for interest, clearly marked not-gated.
4. **Temporal-holdout FP** (unchanged gate: existing ≤ 2%, new-file ≤ 5%) — the
   anti-inflation safeguard; "green" recall only counts with FP still honest.
