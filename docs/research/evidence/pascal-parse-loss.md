# Pascal loses ~29 % of its lines to the parser — diagnosed, and fixed

**Date:** 2026-07-28 → 2026-07-29 · **Status:** **fixed.** 29,16 / 29,09 / 29,52 /
16,14 % → **9,70 / 9,45 / 1,00 / 7,41 %**. Two source-level repairs were tried
before, measured, and reverted; the answer was one bug of ours and one grammar
fork, and neither is a source-level repair.

## The number

Share of lines inside a tree-sitter `ERROR` node — invisible to every rule:
imports, callees, shape, placement. Measured on the **real fit corpus**
(`collect_source_files`, so each repository's own `argot.toml` applies).

| corpus | before | conditional fix | + grammar fork |
|---|--:|--:|--:|
| mormot2 | 29,16 % | 19,14 % | **9,70 %** |
| castle-engine | 29,09 % | 28,42 % | **9,45 %** |
| uos | 29,52 % | 6,74 % | **1,00 %** |
| mseide-msegui | 16,14 % | 11,87 % | **7,41 %** |

Across every corpus the sweep now reads: pascal 7,00 % · typescript 3,19 % ·
cpp 1,30 % · csharp 0,84 % · c 0,72 % · the other seven ~0 %.

## It was two problems wearing one number

Breaking the loss down **by extension** — the first thing nobody had done —
splits it cleanly, and the two halves have nothing to do with each other:

| corpus | `.pas` share of loss | `.inc` share | `.dpr` share |
|---|--:|--:|--:|
| castle-engine | 21,2 % | **78,8 %** | 0,0 % |
| mormot2 | 95,4 % | 4,5 % | **0,2 %** |
| mseide-msegui | 95,6 % | 4,4 % | — |
| uos | 100 % | 0 % | — |

**`.dpr` was never the problem.** 87 of mormot2's 105 project files carry a
first-`ERROR` on a `uses … in '…'` line, which is why that construct topped the
frequency table in the first investigation — but the error is *local to the uses
clause*: `.dpr` lost **271 lines of 10 498**, = **0,15 % of the corpus loss**.
Counting files instead of lines pointed a whole investigation at a rounding
error. Lines are the unit.

## Half the loss was ours

The previous investigation named `TAes = object`, `TLineFeed = (` and
`TStrLen = …` as the constructs that broke mORMot. **All three parse perfectly.**
They fail only *after* `blank_pascal_directives` runs:

```pascal
{$ifdef USERECORDWITHMETHODS}
  TAes = record
{$else}
  TAes = object
{$endif}
```

Blanking every directive to spaces keeps **both** branches — a duplicate name and
an unterminated `record` — and `mormot.crypt.core.pas` lost all 10 643 of its
lines to one error node because of it. `TStrLen = {$ifdef FPC} SizeInt {$else}
integer {$endif};` became `TStrLen = SizeInt integer;`. Keeping both branches is
right for C, where they are two complete declarations and the path is measured
good, and catastrophic for Object Pascal, where a conditional routinely sits
*inside* one declaration.

`blank_pascal_directives` now lexes the source — comments, strings, `//`, so the
`//{$endif}` at `msedbedit.pas:17` no longer unbalances the branch stack — and
keeps only the **first** branch. First, not "whichever parses", so a repository
reads the same on every machine and run.

### The cost, found by a test and paid

Dropping a branch drops the units it names: mormot2 **73 of 229** — the whole
Delphi-only side, `jpeg`, `dbtables`, `midaslib`, the NexusDB family — castle 16,
mseide 5. Each would later read as a brand-new dependency: false alarms
manufactured by the parser, against the co-headline metric. An existing adapter
test caught it.

Resolved by splitting the two questions: **structure from one branch,
dependencies from all of them.** `parse_pascal_every_branch` gives the Pascal
adapter a second, offset-identical view and `extract_imports_with_spans` unions
the two, deduping on position. Re-measured: **0 units lost on all four corpora**,
every parse gain kept. The union is a strict superset of the old behaviour, so
imports can only improve.

## The other half was the grammar, and it is a fork now

`vendor/tree-sitter-pascal/` — upstream `Isopod/tree-sitter-pascal` 0.10.2 (MIT)
plus twelve rules, each with a one-line reproduction from a real corpus file, and
upstream's own 88-case corpus still passing unchanged. `src/parser.c` is
generated and committed, so the build still needs only a C compiler. Full list
and the regeneration recipe: `vendor/tree-sitter-pascal/README.md`.

The largest single rule is the **include fragment**. An `.inc` file is pasted
into the middle of another unit, so it may open inside a class body and be
nothing but method signatures, properties and `strict private` markers. Upstream's
`root` admitted `_definitions` for include files, which covers what may stand at
*unit* level and not at *class-member* level. Adding `declProp` and `declSection`
took castle-engine's `.inc` from **72,6 % to 26,3 %** lost and the corpus from
22,85 % to 9,45 %.

## `.inc` was also being handed to the wrong language entirely

The all-language sweep — run because "can other languages benefit?" was worth an
hour — found `pascal rocksdb 95,12 %` and `pascal curl 100,00 %`. Neither project
contains a line of Pascal. `.inc` is Object Pascal's include extension **and
equally C's**, and `EXT_TO_LANG` routed it to Pascal unconditionally: 28 RocksDB
files and 6 curl files, ~11 600 lines of C, through the Pascal grammar. Not
merely unreadable — *C learned as Pascal vocabulary*. The same files read at
10,6 % loss as C.

Fixed the way `.h` already was: `RepoLangs` carries what the repository actually
writes, and an `.inc` is Pascal's where there are Pascal units, C/C++'s where
there are only translation units, and nothing at all where there is neither —
better unscored than misread. One repo walk answers both questions.

## Three ways to be wrong about where a parse fails

Every one of these produced a confident, false answer in this investigation.

1. **Prefix-bisect with a synthetic `end.`** — invalid for a `.dpr`, which needs
   `begin … end.`, so it returned line 1 for 1 310 files and made a UTF-8 BOM
   look like the cause. It is not: BOM parses fine.
2. **Prefix-bisect cutting inside a comment.** Object Pascal units open with a
   20-line `{ … }` banner; cut at line 10 and the comment is unterminated, so the
   parser blames row 6. Thirty files reported a "culprit" that was prose. Cut
   only at boundaries a lexer says are outside `{…}`, `(*…*)`, `//` and `'…'`,
   and close the prefix with a candidate set.
3. **A CLI harness run from the wrong directory.** `tree-sitter parse` could not
   find the grammar, printed nothing, and a `grep -q ERROR` on empty output read
   as success — 14 of 14 constructs "already fixed upstream". Assert the tool
   produced output before believing what it did not say.

The reliable oracle throughout was the parser argot itself links, driven from a
throwaway Rust test over the real fit corpus: ~8 s for all four corpora.

## What is left, and what it is not

mormot2 9,70 % and mseide 7,41 % are now a genuine long tail — 41 and 55 files,
no single construct above a few hundred lines. castle-engine's residual 9,45 % is
still 80 % `.inc`, much of it **generated data** (`rotate_collider.glb.inc`,
11 915 lines of mesh converted to Pascal source; 409 of its 1 015 `.inc` open with
Emacs' `buffer-read-only` marker). Excluding generated files would move the
number without improving the product, so it was not done — that is the mute
system's job and the repository's choice, not the parser's.

Two rules were tried and reverted for cost: `declField` at fragment root, and
`repeat1` on `declVars`/`declConsts`/`declTypes`. Because those sections admit
zero entries, an eager `declField` takes `var x: integer;` apart into an empty
`var` and a loose field — 9 upstream tests regressed for ~0,7 % of one corpus.

`jnicall`, a JNI-only calling convention, is the one construct in the
reproduction suite still unparsed. It appears in `uos_jni.pas` and costs 239
lines.

The sweep also showed **TypeScript at 3,19 %** (outline 7,52 %, hono 11,74 %),
which no one has looked at. The reference figures "TypeScript 0,37 % / C 0,37 %"
in the first version of this document were each measured on a single corpus.
