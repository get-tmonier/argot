# Pascal case folding — an 8× over-fire drop, and a "limit" that was a bug

**Date:** 2026-07-28 · **Status:** positive — A/B measured, recall unchanged,
one documented corpus limit retracted.

**Question:** the Pascal adapter compared unit and identifier names
case-sensitively in a case-insensitive language. Fixing that, recovering units
the conditional-compilation directives hid, and masking prose by span rather
than by line all change what the scorers read. What does that cost, and what
does it buy?

## Method

`main` (v0.2.109) against the branch, **same corpus clones, same catalogs, same
pinned SHAs, same windows** — the only variable is the code. Four Pascal
corpora: the two catalogued (`castle-engine`, `mormot2`) and the two extras
(`uos`, `ideu`). Temporal-holdout false positives plus novel-pattern recall.

```sh
# before
git worktree add /tmp/argot-main-baseline main
/tmp/argot-main-baseline/target/release/argot-bench \
  --corpus castle-engine,mormot2,uos,ideu \
  --data-dir benchmarks/data --catalogs-dir benchmarks/catalogs
# after
just bench            # the full 39-corpus run; the same four rows read off it
```

## Results

**False positives (temporal holdout, leak-free):**

| corpus | before | after | change |
|---|--:|--:|---|
| castle-engine | 0.99% (7/705) | **0.85%** (6/705) | −0.14 pp |
| mormot2 | 0.73% (5/688) | **0.73%** (5/688) | unchanged |
| **uos** | **18.21%** (640/3514) | **2.28%** (80/3514) | **−15.93 pp** |
| ideu | 1.32% (49/3707) | **0.89%** (33/3707) | −0.43 pp |

**Novel-pattern recall, identical fixtures:**

| corpus | before | after |
|---|--:|--:|
| castle-engine | 11/11 (100%) | 11/11 (100%) |
| mormot2 | 11/11 (100%) | 11/11 (100%) |

**22/22 → 22/22. Nothing lost, every corpus equal or better.**

The full 39-corpus run on the branch scores **639/746 = 85.7%** novel-pattern
catch against the ≥85% gate, with no corpus erroring.

## What the uos number means

`uos` sat at 18% over-fire, and the Pascal port recorded that as a property of
the corpus: *"small heterogeneous C-wrapper lib, the RUBRIC small-corpus
limit"*. It was accepted, written down, and it was wrong.

uos writes Pascal unit and identifier names in mixed case — the language is
case-insensitive and its authors used that freedom. Every spelling of one name
read as a separate unknown, so roughly one hunk in five carried something
"never seen". Nothing about the corpus's size or heterogeneity was responsible.

Three changes took it to 2.28%:

- **`unit_identity`** — unit names fold to one identity. On MSEide/MSEgui the
  same defect made 30 of 907 learned specifiers case-variants of another entry,
  and made the supersession miner produce `sysutils` → `SysUtils`: a migration
  from a unit to itself.
- **`uses_identifiers`** — recover units the grammar leaves in `ERROR` nodes
  when a conditional-compilation directive sits inside the clause
  (`uses {$if defined(darwin)} cwstring {$else} msecwstring {$endif}`). Without
  it `cwstring` never entered the model, and every later use read as a new
  dependency.
- **`ProseMask`** — blank a comment's *span*, not its line, so a unit commented
  out in place (`uses msedynload{,mseguiintf};`) no longer takes the whole
  clause with it. This one is language-agnostic; all twelve adapters had it.

## The lesson worth keeping

A benchmark number that is explained rather than fixed becomes part of the
record, and the explanation gets cited afterwards as a known limit. This one
survived a full language port, a capstone, and a published table. It was a
case-sensitivity bug the whole time.

When a single corpus is an order of magnitude worse than its siblings, that is
a defect hypothesis before it is a corpus-property hypothesis — and the two are
distinguishable by the same A/B that would have caught it here.
