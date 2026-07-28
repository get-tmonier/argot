# A whole-file rewrite is not one pattern being introduced

**Date:** 2026-07-28 · **Status:** diagnosis complete and fixed; final numbers
pending the definitive honest run.

**Question:** uos was the only corpus above 2 % over-fire (3,09 % existing,
22,73 % new-file) and the worst in the benchmark. The Pascal port recorded 18 %
there as a "small-corpus limit"; the case-folding fix took it to 2,28 %; it now
reads 3,92 %. What actually fires?

## What fires

**114 of its 145 hits are in one file**, `src/uos.pas` (12 813 lines). And the
largest "hunks" *are* the file:

| commit | message | shape |
|---|---|---|
| `e1492ec` | **"Comment reordered for all the functions"** | 2 564 insertions / 2 547 deletions, one file |
| `82e7810` | "Removed uos_RePlay2()… Renamed portaudio64.dll" | 4 104 ins / 4 100 del |
| `03a1ba10` | — | a 12 802-line hunk in a 12 813-line file |

A hunk that size is not an edit anyone wrote — it is a cosmetic reshuffle, and it
carries most of the file's vocabulary. Something in it is always unfamiliar, so
the verdict reports the hunk's size rather than the code.

Nothing capped it. `hunk_lines` exists but is a **rendering** limit — how many
lines to print — not a scoring one.

## It is not a uos quirk

Across the whole benchmark, **29,3 % of every false-positive hit comes from
hunks over 50 lines**:

| corpus | share of its false alarms from >50-line hunks |
|---|--:|
| mormot2 | 60 % |
| rocksdb | 53 % |
| jellyfin | 50 % |
| outline | 50 % |
| excalidraw | 43 % |

That is the shape a linter sweep, a licence-header pass, a formatter change, or
an agent reformatting a file all produce — so this is a defect users meet, not a
benchmark artefact.

## The cap, and why 100

| fixture hunk sizes (all 977 in the catalogue) | |
|---|--:|
| median | 13 |
| p90 | 25 |
| p99 | 59 |
| **max** | **80** |
| over 100 lines | **0** |

No fixture anywhere comes close, so a cap at 100 costs nothing measurable while
removing **73 of 471 false alarms (15,5 %)** across ten corpora — uos 47,
ideu 9, rocksdb 4, hugo 3, ink 2, redis 2 of 2, plus fastapi, mseide-msegui,
excalidraw and castle-engine.

**New files are exempt.** There the whole file legitimately *is* the change, and
it is already judged against the new-file threshold rather than an edit
distribution. The separation is exactly right on the real data: of uos' >50-line
hits, **56 are on existing files and 5 on new ones** — and the 5 include
`1b243147`, a genuine 1 495-line decoder addition that must stay catchable.

**Deletions would be the sharper discriminator** — a rewrite is insertions ≈
deletions — but the scoring `PatchBatch` carries only the new side, by design
(`two_sided.rs` exists for the integrity pass and notes the scoring path "never
carries" the old side). New-vs-existing separates the real cases without that
plumbing, so the cheaper discriminator is the one that ships.

**Reported, never silent:**

```
[argot] N hunk(s) over 100 lines were not judged — that much at once is a
        rewrite, not one pattern being introduced, and holds most of the file's
        vocabulary. Review those by hand.
```

## The other half: uos had no config at all

Asking what a uos maintainer would actually configure turned up the second
cause. **uos is the only corpus in the benchmark with no `argot.toml`.** It is a
library:

| | files | lines |
|---|--:|--:|
| `examples/` (demo programs) | 63 | **46 344** |
| `src/` (the library) | 21 | 25 994 |

So **64 % of the Pascal argot learns from is demo code**, and it judges `src/`
against that voice — where **141 of the 145 false alarms land**. It also vendors
two GUI toolkits as submodules (`use/fpGUI`, `use/mseide-msegui`).

Its catalog now excludes both. This is the sanctioned mechanism, not special
pleading: `sync_corpus_config` installs "the per-corpus `argot.toml` a real user
of this repo would write", **dagster's already excludes `examples/`**, fastapi
excludes `docs_src/`, wagtail `client/`+`docs/`+`scripts/`, and redis (`deps/`),
rocksdb (`third-party/`) and castle-engine (vendored libs) all exclude their
third-party trees.

## Results

Pending the definitive honest run — the bench that would have measured this was
killed deliberately to free the machine for the semantic re-bench, so that one
run validates the grammar fixes, the language floor, the hunk cap and the uos
config together.

What it must show: headline recall **647/756 = 85,6 %** unmoved (the cap must not
have bought over-fire with a lost catch), uos under 2 %, and no corpus worse.

`just verify` is green **including the parity-locked golden suites**, so no
golden output moved — no committed reference fixture has an oversized
existing-file hunk that fired.

## The lesson worth keeping

Two different defects hid behind one bad number, and neither was the one the
record predicted. "uos is a small heterogeneous C-wrapper library" survived a
language port and a published table; it was case sensitivity, then a missing
config, then an unbounded hunk. A corpus that is an order of magnitude worse than
its siblings is a defect hypothesis before it is a corpus-property hypothesis —
and the second look is worth taking even after the first one already found
something.
