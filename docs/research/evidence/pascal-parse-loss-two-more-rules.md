# Two more Pascal rules take mseide-msegui from 6,59 % to 0,05 %

**Date:** 2026-07-30 · **Status:** **fixed.** Follow-up to
[`pascal-parse-loss.md`](pascal-parse-loss.md), which took the same corpus from
16,14 % to 7,41 % and stopped there.

## The number

Share of lines inside a tree-sitter `ERROR` node — invisible to every rule:
imports, callees, shape, placement. Measured over mseide-msegui's own fit corpus
(493 Pascal files, 548 418 lines, its committed `argot.toml` applying) through
the real pipeline, directive handling included, counting each file's widest
`ERROR` span.

| | lines inside an `ERROR` | share |
|---|--:|--:|
| v0.2.118 as released | 36 136 | **6,59 %** |
| + these two rules | 260 | **0,05 %** |

Three files were lost **whole** and now parse clean:

| file | lines, all of them lost |
|---|--:|
| `lib/common/db/msebufdataset.pas` | 10 979 |
| `lib/common/report/msereport.pas` | 8 738 |
| `lib/common/kernel/linux/mseguiintf.pas` | 7 452 |

The last of those is the X11 gui backend. It carried **1 921 `ERROR` nodes**, and
of its 102 routines only **21** were recoverable. After the fix: **0** and all
of them.

## The two constructs

Both are ordinary Object Pascal, both are on nearly every page of this codebase,
and both desynchronise the parser for the **rest of the unit** — which is why two
small gaps cost whole files rather than a few lines each.

**A record/class/object list may end without a terminating `;`.**

```pascal
 MwmHints = record
  flags: culong;
  status: culong        { <- no `;`, and this is legal }
 end;
```

`lib/common/kernel/linux/mseguiintf.pas:460`. `declField` required the `;`; it is
now `optional`.

**A block may end on a labelled *empty* statement.**

```pascal
 end;
endlab:                 { <- the label marks the empty statement before `end` }
end;
```

`lib/common/kernel/linux/mseguiintf.pas:2673`. This is the shape every
`goto`-based cleanup in MSEgui uses to jump to the end of a routine — the unit
turns `{$GOTO ON}` on in its header. `_statementsTr` required a statement after
the last label; it now accepts a label as the trailing element.

## The conflict the first rule needs

Making the field terminator optional is not free. Reached through a field whose
type is an anonymous class, a following `[` is either that class's guid or the
RTTI attribute list opening the *next* field:

```
kRequired  identifier  ':'  kClass  •  '['  …
```

That is the same bracket clash the neighbouring `rtti` conflicts already cover
(`declProcFwd`, `declVars`, `declConsts`, `declTypes`), so `declClass` joins
them. `tree-sitter generate` then reports no conflicts.

## What it changes downstream

Making 36 000 lines visible is not neutral for the scorer — it is the point, and
it moves calibration. Refitting mseide-msegui on the same 503-file corpus:

| | released grammar | fixed grammar |
|---|--:|--:|
| functions in the semantic index | 25 902 | **26 905** (+1 003) |
| calibration candidate hunks | 15 178 | **15 811** (+633) |
| calibrated pascal threshold | 5,4496 | **5,2465** |

So any repository already carrying a committed Pascal snapshot should be refit
after this ships. A grammar change does not move `model_hash`, so `argot status`
will not ask for it on its own — it is an explicit decision.

## Guards

- `a_final_record_field_may_omit_its_semicolon` and
  `a_block_may_end_on_a_labelled_empty_statement` in
  `crates/argot-lang/src/ts_parse/tests.rs` — beside the other parse-level
  guards (`only_one_branch_of_a_conditional_survives`,
  `a_compiler_directive_does_not_swallow_the_unit`), which is where a
  source-in/tree-out assertion belongs. Both **fail** against the released
  grammar, checked with
  `git checkout HEAD~1 -- vendor/tree-sitter-pascal/{grammar.js,src}` and
  re-running: 2 failed, 196 passed.
- Upstream's own 88-case corpus still passes unchanged.
- Workspace green at 933 tests with `semantic`, `arch`, `integrity`, `script`.

## Not done here

The recall/false-alarm harness (`argot-bench`) was **not** run for this change,
and by decision it does not gate it — the full matrix is too slow to sit in
front of the fix. It lands in a follow-up PR.

What that leaves open:

- the parse-loss measurement above **is** like-for-like and uses the same method
  as the earlier investigation, so 6,59 % → 0,05 % stands on its own;
- the calibrated Pascal threshold moved (5,4496 → 5,2465) because 36 000
  previously invisible lines now feed calibration. Whether that improves or
  degrades catch-rate on the Pascal corpora — mormot2, castle-engine,
  mseide-msegui, uos — is **unmeasured**;
- the PR's `benchmark` job runs the *quick* subset only: one corpus, `ink`, in
  TypeScript, recall-only, no false-positive columns. It exercises none of the
  above.

More code being visible should help rather than hurt, but that is a reasonable
expectation, not a measurement.

## How it was found

Setting Argot up on mseide-msegui, a custom rule reading the gui backend
contract reported the X11 backend as implementing *10 of 100* entry points when
it implements all of them. The rule was not wrong; the AST it read was. Bisecting
truncations of the file — each one closed with a valid `implementation`/`end.` so
the cut itself would not register as the error — located line 460, and the same
loop located 2673 once the first was fixed.
