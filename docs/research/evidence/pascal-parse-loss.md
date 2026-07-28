# Pascal loses ~29 % of its lines to the parser, and it is the grammar

**Date:** 2026-07-28 · **Status:** diagnosed, **not fixed.** One source-level
repair was built, measured, and reverted for not paying. The structural answer
is the grammar.

**Question:** the grammar sweep recorded "mormot2 30,45 %" as the largest
remaining parse loss. Is it still there, and is it one corpus?

## It is not one corpus

Measured on the **real fit corpus** — what `collect_source_files` returns, so
each repository's own `argot.toml` exclusions apply:

| corpus | files | lines lost | widest ERROR |
|---|--:|--:|---|
| mormot2 | 525 | **29,16 %** | 14 131 (`mormot.core.base.pas`) |
| castle-engine | 2 179 | **29,15 %** | 11 915 (`rotate_collider.glb.inc`) |
| uos | 22 | **29,52 %** | 1 820 (`uos.pas`) |
| mseide-msegui | 505 | 16,14 % | 12 760 (`msedbedit.pas`) |

**Roughly a third of all Pascal sits inside an `ERROR` node** — invisible to
every rule: imports, callees, shape, placement. For scale, the same day's fixes
took TypeScript from 31,53 % to 0,37 % and C from 9,33 % to 0,37 %.

An earlier figure of "castle-engine 2,53 %" was **wrong**: that probe walked the
whole tree ignoring exclusions and counted a different file set.

## What actually breaks

`mormot.core.base.pas` is a single 14 131-line `ERROR` starting at row 1 — the
parse fails at the top and never recovers, the same shape as `curl.h`. Bisecting
the first failing prefix names the construct.

**Line 58 — a directive standing where a value must be:**

```pascal
SYNOPSE_FRAMEWORK_VERSION = {$I ..\mormot.commit.inc};
```

This one is **our own doing**. `blank_pascal_directives` blanks `{$…}` to
spaces, which here leaves `SYNOPSE_FRAMEWORK_VERSION =      ;` — a const with no
value. Confirmed minimally:

| | parses |
|---|---|
| `V = {$I ..\x.inc};` (as written) | ✗ |
| blanked to spaces → `V =   ;` | ✗ — the fix does not help |
| placeholder → `V = 0 ;` | **✓** |
| a directive between statements | ✓ either way |

**Line 213, once that is repaired — a Delphi codepage-parameterised type:**

```pascal
WinAnsiString = type AnsiString(CP_WINANSI);
```

Dotted unit names (`unit mormot.core.base;`) were the first hypothesis and are
**fine** — the grammar handles them.

## The repair that was built and reverted

Substituting a byte-length-preserving placeholder (`0` plus spaces) when a
directive follows `=` is correct and demonstrably works: the first failing line
in `mormot.core.base.pas` moved from **58 to 213**.

It does not pay at corpus level:

| corpus | before | with the repair |
|---|--:|--:|
| mormot2 | 29,16 % | 29,17 % |
| castle-engine | 29,15 % | 29,21 % |
| uos | 29,52 % | 29,58 % |
| mseide-msegui | 16,14 % | 16,18 % |

Flat to marginally *worse*. Extending it to argument positions (`(`, `,`, `[`)
was worse still — a conditional spanning several arguments gets a placeholder
per branch and the call reads as nonsense.

Reverted. A change that adds a special case and moves no number is complexity
bought with nothing.

## Why it does not pay, and what would

Each file fails on the **first** unsupported construct, and the `ERROR` node
then swallows everything after it. Repairing that construct only exposes the
next one in the same file — 58 → 213 → whatever follows. The loss is a **long
tail of gaps in `tree-sitter-pascal`**, not one defect, so source-level repairs
are whack-a-mole: every one costs a special case in a hot path, and the file
stays unparsed.

What would actually pay, in rough order of cost:

1. **Fix the gaps upstream in `tree-sitter-pascal`** — `type AnsiString(CP)` and
   whatever the tail holds. Slowest, and the only one that makes the number
   fall for everyone.
2. **Evaluate an alternative Pascal grammar.** Whether a better-maintained one
   exists is unknown and worth an hour before anyone writes grammar rules.
3. **Measure the tail first.** Bisect the first failing line across a few hundred
   files and cluster the constructs. Two or three may cover most of the 29 %, and
   nobody should start patching before knowing that.

Step 3 is the honest next move, and it is cheap. Everything above is guesswork
about the distribution until it is done.

## The lesson worth keeping

A repair that works on the construct in front of you and does not move the
corpus number is not a fix — it is a special case. The measurement that mattered
was not "does line 58 parse now" (it does) but "does the corpus lose fewer
lines" (it does not). Prefer changing the structure — here, the grammar — to
working around it.
