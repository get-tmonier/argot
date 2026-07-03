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

## Result (production-path recall + temporal-holdout FP)

| | catch (gated ≥85%) | bat FP | jellyfin FP |
|---|---|---|---|
| before (no gate) | 48/49, 8/8 corpora | 11.54% | 9.73% |
| **foreign-reach gate** | **48/49 (98%), 8/8 corpora** | **8.88%** | **6.79%** |

`call_receiver` false alarms: bat 33→24, jellyfin 21→7. Every gated foreign
catch survives (they all reach a foreign module); the cry-wolf on the repo's
own new functions is gone. The one miss (`laravel_foreign_respect_1`) is a
Validator name-collision, not gate-related — unchanged from before.

## Per-corpus realism, not corpus-specific production code

`redis`' foreign libevent catch depended on a corpus quirk: `event_base_new`
is attested via **vendored** `deps/hiredis` example code, so it looked
already-used. The fix is not a hardcoded `deps` in the core — it is the
`.argotignore` a **real redis maintainer would write** (`deps/`), applied by
the bench per corpus (`benchmarks/catalogs/<name>/argotignore`, installed into
the clone before fit by `sync_corpus_argotignore`). Voice is the repo's own
code; vendored trees are muted the way a user of that repo would mute them.
Corpus-specific knowledge lives in the bench, never in production scorers.
