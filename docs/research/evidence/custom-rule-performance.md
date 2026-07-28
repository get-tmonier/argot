# Scripted rules, measured — a scorecard, and two defects it found

**Date:** 2026-07-28 · **Status:** positive — the rules hold up; the sandbox
around them did not.

**Question:** the built-in rules have a bench. The scripted ones have a fixture
suite and nothing else. What is their real precision, and does the sandbox
behave when a rule meets a large file?

## Method

The four rules MSEide/MSEgui carries in `.argot/rules/` — three about its
platform-backend contract, one about its C ABI — measured three ways:

1. **Fixture suite** — `argot rules test`, the authoring loop.
2. **Whole tree** — every file scored as if added, with the built-in groups
   off, so each rule sees all 921 files:
   ```sh
   EMPTY=$(git commit-tree $(git hash-object -t tree /dev/null) -m e)
   argot check "$EMPTY..4233521f2" \
     --rule voice=off --rule semantic=off --rule architecture=off --rule integrity=off
   ```
3. **Real changesets** — the five branches and two demos, plus the 400-commit
   sweep, hand-adjudicated (`mseide-msegui-adjudication.md`).

## Scorecard

| rule | fixtures | whole tree | branches | 400-commit sweep | precision |
|---|--:|--:|--:|--:|--:|
| `backend-contract-coverage` | 2/2 | 2 fires | 7 fires | 0 | **9/9 true** |
| `contract-drift` | 1/1 | 0 | 2 fires | 0 | **2/2 true** |
| `platform-backend-contract` | 3/3 | 0 | 1 fire | 0 | **1/1 true** |
| `c-abi-managed-type` | 6/6 | 0 | 0 | 0 | — (no live occurrence) |
| **total** | **12/12** | **2** | **10** | **0** | **12/12 = 100%** |

**Twelve fires across every surface, twelve true, zero false alarms over 2 489
accepted hunks.** The zeros are checkable rather than hopeful:
`backend-contract-coverage` fires exactly twice on the whole tree because the
repository has exactly two GUI backends; `platform-backend-contract` is silent
because both include the shared contract; `contract-drift` needs a *change* to
a contract, so a whole-tree scan cannot produce one.

## Defect 1 — the sandbox disabled a rule and called it silence

The first whole-tree run said:

```
[argot] custom rule c-abi-managed-type: Too many operations (line 38, position 21)
        — rule disabled for this run
```

`lib/common/db/msedb.pas` is 9 439 lines. The rule exhausted its operation
budget there, and argot disabled it **for the rest of the run** — so it was
never applied to the other 920 files, and reported nothing. Its zero on the
400-commit sweep has to be re-read the same way: that sweep scores `msedb.pas`.

Two separate faults:

- **The budget was a cap on the input, not on the rule.** A flat 1 000 000
  operations gives a 9 439-line file about 106 per line; one tree-sitter query
  per declaration plus a few string tests exhausts it. It now scales — a flat
  allowance plus a per-line one, under a ceiling. A runaway loop still blows any
  linear budget on its first file whatever the size, which is the guard's job.
- **The blast radius was the whole run.** A trip now costs the file: skip it,
  carry on, and say so (`skipped N file(s) over budget — every other file was
  checked`). Only a rule that trips on **five separate files** is the rule that
  is wrong, and only then is it disabled. Silence that means *not checked* must
  not look like silence that means *nothing found*.

## Defect 2 — the rule's only fire in 924k lines was wrong

With the rule finally running everywhere, it produced exactly one finding:

```
c-abi-managed-type · lib/common/kernel/msearrayprops.pas:275
  ↳ `tobject` is compiler-managed — it cannot cross a C ABI
```

Line 275 is `fobjectlinker: tobjectlinker;` — a class field, not a signature.
Two causes, both in the rule: `contains(": " + m)` has no word boundary, so
`": tobject"` matches `": tobjectlinker"`; and the calling-convention window
looks two lines ahead, sweeping in a field that merely sits above a `stdcall`
method. Fixed in the fork, pinned as `silent-on-longer-type-name`.

Worth noting how the fix went: the first version was correct and walked each
line character by character, which in interpreted Rhai exhausted the budget on
five files — and the new per-file accounting reported exactly that, by name and
count, instead of going quiet. The guard caught its own author.

## Cost

Whole tree, 921 files, 924 048 lines:

| | |
|---|--:|
| every rule off, **including** the scripts | 47,0 s |
| the four scripted rules on | 48,7 s |
| **the scripts' own cost** | **1,7 s (3,5 %)** |

The scripts are not the expense. The 47 s is the diff and the parse, and it is
paid with every rule disabled — about 51 ms per whole-file hunk, consistent with
the real branches (sdl2: 91 hunks, 5,1 s *with* the semantic layer on).

**Left standing, and worth more than any of this:** that scan runs at 97 % CPU
— one core of eleven. The base check path is essentially serial, which is the
`score patches (statistical)` observation from the brief, unaddressed here
because it is a scoring-path change and needs the goldens and a bench behind it.
