# Break-fixture rubric (frozen — issue #92)

This rubric is fixed **before** any fixture is scored. A fixture that fails
to fire is a *finding to report*, never a reason to swap the fixture or the
corpus. Amendments to this rubric require a recorded rationale in
`docs/research/evidence/` and re-scoring of every existing fixture.

## Per-language catalog composition (12+ fixtures)

Every language catalog contains **at least 12** spliced fixtures with this
class distribution (the hard classes argot advertises, not just the easy
tripwire):

| Class | Count | What it is |
|---|---|---|
| `wrong_error_discipline` | ≥ 3 | Error handling the repo never uses: raw `panic!`/`exit()`/return-code checks in an exception-style codebase, string throws, swallowed errors, errno where the repo wraps errors, etc. |
| `wrong_concurrency` | ≥ 2 | A concurrency model foreign to the repo: raw threads where the repo is async/event-loop, manual mutexes where the repo uses channels/executors, busy-wait polling, etc. |
| `wrong_api_within_known_lib` | ≥ 3 | Misuse or off-voice use of a library the repo ALREADY imports: deprecated/legacy API of the same dependency, low-level API where the repo standardizes on a high-level wrapper, hand-rolled code duplicating a repo utility. |
| `naming_shape_break` | ≥ 2 | Identifier morphology/structure foreign to the repo: camelCase in a snake_case repo, Hungarian notation, single-letter public APIs, God-function shape where the repo is small-function. |
| `foreign_import` | ≤ 2 | A dependency the repo does not use (verified 0-usage at the pinned SHA). At most 2 — this is the tripwire class the import stage catches by definition. |

## Fixture construction rules

1. **Spliced, not whole-file**: every fixture declares `host_file` +
   `host_inject_at_line` pointing into a real corpus file at the pinned
   primary SHA. The scored hunk is the break body only (`hunk_start_line`..
   `hunk_end_line` inside the fixture file); surrounding decoy lines must be
   idiomatic corpus-style code.
2. **Corpus-authentic**: the break must be plausible in that repo's domain
   (a datastore repo gets a wrong-API break against a lib it really uses,
   not an image library import). Verify claimed "repo already uses X" /
   "repo never uses Y" against the pinned SHA (`git show <sha>:<path>`,
   `git grep <term> <sha>`), and record the verification in `rationale`.
3. **Compiles/parses in isolation**: the fixture file must parse with the
   language's tree-sitter grammar (no placeholder pseudo-code).
4. **No swapping**: once a fixture is committed under this rubric it is only
   removed if it is factually wrong (e.g. the "foreign" lib turns out to be
   used by the repo). Not firing is a recall miss to report.
5. **Meta-comments**: `// Break: ...` / `# Break: ...` design notes are
   stripped by the harness and never reach the scorer; use them to document
   the break inside the fixture.

## Measurement

Recall is measured through the **production path** (`argot-bench --mode
production`): fixture planted on disk at the pinned SHA, staged with real
git, judged by `argot fit` + `check --staged` — with the honest (LOO)
calibration. Caught = any hit on the host file. Report recall per class,
with the uncaught fixture ids named.
