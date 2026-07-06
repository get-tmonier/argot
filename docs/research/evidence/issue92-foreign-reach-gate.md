# Issue #92 — the foreign-reach gate: cut call_receiver false alarms without losing catches

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout`. Follows
[the false-alarm diagnosis](issue92-false-alarm-diagnosis.md): a call into a
foreign library and a call to a new local function are the *same* signal (an
unattested callee), so `call_receiver` cried wolf on the codebase's own new
code. This is the fix.

## The gate

`call_receiver` may flag a hunk on its own (`cr_fired`) **only when the hunk's
file reaches a module foreign to the repo**. Foreign reach (per file, cached)
is any unattested callee that is either:

- **namespace-qualified into an unknown module** — a `::`/`\` path
  (`tokio::spawn`, `\Doctrine\ORM\EntityManager.create`) whose leading segment
  is neither a local receiver, `self`/`this`, nor a namespace the corpus
  already attests. This wins even when the leaf method is a common attested
  name (`\React\EventLoop\Loop.get` — `get` is everywhere, `React` is foreign);
  or a single-`.` receiver (`viper.GetString`) whose receiver namespace is
  unknown, *unless* its method is corpus-known (a fresh receiver + in-voice
  method is not foreign); **or**
- **a bare foreign symbol** — an unqualified callee the repo never attested
  (`event_base_new`, a C library function). New local functions are excluded
  first via the change's own callable/value bindings.

A file that only reaches known modules (`String::with_capacity`,
`self.render`, `output.push_str`, a new local helper) has no foreign reach, so
the codebase's own new code stays quiet. The file-level scope means a foreign
dependency spread across hunks (the `\React` assignment in one, the calls
through a local receiver in another) flags every hunk, not just the naming line.

## Result — full honest re-bench, all 27 corpora

Production-path recall (catch) + temporal-holdout FP, `--mode honest`:

| | before (no gate) | **foreign-reach gate** |
|---|---|---|
| novel-pattern catch (gated ≥85%) | 48/49, 8/8 corpora | **48/49 (98%), 8/8 corpora** |
| existing-file FP (aggregate, 27 corpora) | 3.50% (750/21416) | **2.00% (429/21416)** |
| `call_receiver` false alarms (total) | 529 | **195** (−63%) |
| corpora at ≤2% existing FP | 11/27 | **19/27** |

**Every corpus improved or held flat — zero regressions.** Standouts:
rubocop 6.96→1.27% (cr 48→2), homebrew 4.59→0.22% (cr 21→1), hugo 5.89→1.38%,
gh-cli 2.30→0.00% (cr 14→0), outline 2.99→0.53%, rocksdb 6.23→4.26% (cr
134→81), fastapi 6.58→5.18% (cr 42→18), bat 11.54→8.88%, jellyfin 9.73→6.79%
(cr 21→7). Every gated foreign catch survives (they all reach a foreign
module); the cry-wolf on the repo's own new functions is gone. The one miss
(`laravel_foreign_respect_1`) is a Validator name-collision, not gate-related.

Residual worst corpora are the next levers, in a different stage or genuinely
novel: ink 10.85% (still call_receiver-heavy — a React/TS lib whose new code
reaches many libraries), bat 8.88% (largely the git2→gix / minus **dependency
migrations** — arguably correct to flag), jellyfin 6.79% (now mostly the
**convention** stage, 22), fastapi 5.18% (**import** stage, 59 — new deps).

## Per-corpus realism, not corpus-specific production code

`redis`' foreign libevent catch depended on a corpus quirk: `event_base_new`
is attested via **vendored** `deps/hiredis` example code, so it looked
already-used. The fix is not a hardcoded `deps` in the core — it is the
`.argotignore` a **real redis maintainer would write** (`deps/`), applied by
the bench per corpus (`benchmarks/catalogs/<name>/argotignore`, installed into
the clone before fit by `sync_corpus_argotignore`). Voice is the repo's own
code; vendored trees are muted the way a user of that repo would mute them.
Corpus-specific knowledge lives in the bench, never in production scorers.
