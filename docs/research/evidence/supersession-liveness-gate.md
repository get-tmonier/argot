# Supersession liveness — the coverage gate, and why not recency

**Date:** 2026-07-28 · **Status:** positive — the bar is measured, not guessed;
one false migration removed, every real one kept.

**Question:** the miner drops a pair whose old side is *gone* from the corpus (a
completed migration, nothing left to enforce). What drops the mirror case — a
migration that **never happened**?

## The case that forced it

MSEide/MSEgui (924 048 lines of Object Pascal, 9 835 commits) mines exactly one
pair, and it is wrong:

| | |
|---|---|
| pair | `msestrings` → `msetypes`, import kind |
| replacement commits / files | 3 commits, **11 files**, 2017-07-13..14 |
| corpus files still importing `msestrings` | **272 of 508 (53.5 %)** |
| coverage = converted / (converted + leftover) | **11/283 = 3.9 %** |

Nine years on, more than half the tree still imports the "from" side —
including the shipped X11 backend. The rule fired on **six of seven** real
changesets for doing exactly what `lib/common/kernel/linux/` does.

Every existing guard passes it:

- support (≥3 commits, ≥3 files): 3 and 11.
- asymmetry, replacement-sink, churn caps: nothing to catch.
- **trend** (`net_since(old) < 0 && net_since(new) > 0`): tests the *sign*, not
  the magnitude. A net decline of −1 over nine years passes.
- **leftovers**: drops a pair whose old side is *gone*. There was no symmetric
  guard for a pair whose old side never moved.
- `CALLEE_UBIQUITY_FRACTION` (0.2) exists for callee pairs only; import pairs
  had no corpus-side liveness gate at all.

## Method

Ran the shipped miner — `argot fit` with the semantic layer off, so the fit is
voice-only — over every corpus clone in `benchmarks/data/` that carries git
history, and read `leftover_count` and `files` out of each
`scorer-config.json`. 18 corpora, 11 languages.

## Results

| corpus | pair | files | leftover | coverage |
|---|---|--:|--:|--:|
| ripgrep | `regex` → `regex_automata` (import) | 8 | 1 | **0.889** |
| scrapy | `spider.crawler.stats.inc_value` → `self.…` (callee) | 3 | 1 | **0.750** |
| ideU | `msefiledialog` → `msefiledialogx` (import) | 10 | 36 | **0.217** |
| ideU | `sysutils` → `SysUtils` (import) | 17 | 313 | 0.052 |
| ideU | `fo.show` → `fo.Show` (callee) | 4 | 20 | 0.167 |
| **mseide-msegui** | `msestrings` → `msetypes` (import) | 11 | 272 | **0.039** |
| composer · wagtail · excalidraw · fastapi · rich · hono · hugo · cobra · curl · redis · eslint · castle-engine · mormot2 · uos | — | | | **mine nothing** |

Two of the ideU rows are a separate defect, not migrations: Object Pascal is
case-insensitive, so `sysutils` → `SysUtils` and `fo.show` → `fo.Show` are
migrations *from a name to itself*. Both disappear now that the Pascal adapter
folds unit and identifier case; they are listed because they were in the data
when the bar was chosen, and excluding them does not move it.

## The bar

Coverage = `files converted / (files converted + corpus files still using the
old side)`. Real migrations sit at **0.217, 0.750, 0.889**; the false one at
**0.039**. The geometric midpoint of the two nearest, 0.039 and 0.217, is
0.092, so **`MIN_COVERAGE = 0.10`** — both sides clear it by about 2.5×.

An early real migration below the bar is not a gap: `[[migration]]` exists to
declare one before history shows enough signal, and the config comment already
says so. Mining is for migrations with momentum; declaration is for the ones
you want enforced early.

## Recency — considered and rejected

The obvious second gate is age: the MSEgui pair last moved in July 2017, in the
oldest ~4 % of a replay window spanning nine years. It was rejected on the
argument, before it cost a line of code:

- A migration that converted 200 files in 2017 and left 5 stragglers should
  fire **more** loudly nine years on, not less — new code written against the
  old side of a decision the repo finished long ago is a stronger signal, not a
  weaker one.
- A migration that *completed* is already dropped (no leftovers).
- A migration that reversed is already dropped (the trend guard).

So age does not separate the cases; **coverage does**. What looked like an
age problem — "a 2017 rename treated as live" — is a completeness problem: the
rename never propagated. Recorded here so the next person does not re-derive
the age gate and find it plausible.

## Cost

None at check time (the gate runs once per pair at fit, against the corpus scan
the leftover attachment already performs). No new configuration surface.

## Result

- mseide-msegui: 1 mined pair → **0**, and `superseded` fires 0 times over the
  seven real changesets (was 6) and 0 over the 400-commit sweep (was 1).
- ripgrep, scrapy: unchanged, both still mined and still fire.
- The 14 corpora that mined nothing still mine nothing.

Pinned by `attach_drops_a_migration_that_never_propagated` in
`crates/argot-rules-voice/src/scoring/supersede/tests.rs`, which encodes both
sides: 2 converted against 40 left behind is dropped, ripgrep's 8-against-1
ratio survives.
