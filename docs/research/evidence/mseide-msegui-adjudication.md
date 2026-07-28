# Every finding argot raises on MSEide/MSEgui, judged by hand

**Date:** 2026-07-28 · **Status:** complete — seven changesets and the
400-commit sweep.

**Question:** on a 924 048-line, 20-year Object Pascal repository, is each thing
argot says worth a maintainer's minute — and where it is not, is that a defect
in argot or an exception in the repo?

The rule from the brief: *disabling a rule is legitimate only after proving the
rule right and this repo the exception.* Every finding below carries a verdict —
**true** (a maintainer should look), **arguable** (defensible either way), or
**wrong** (argot is mistaken). Every *wrong* is fixed in argot and pinned by a
test; none is muted.

## Bench, first — the changes that made this possible cost nothing

Nine defects were fixed getting to the numbers below, several of them in code
every language shares. `main` against the branch, same clones, same catalogs
(`pascal-case-folding-bench.md`):

| corpus | over-fire before | after | recall before | after |
|---|--:|--:|--:|--:|
| castle-engine | 0.99% | **0.85%** | 11/11 | **11/11** |
| mormot2 | 0.73% | **0.73%** | 11/11 | **11/11** |
| uos | **18.21%** | **2.28%** | — | — |
| ideu | 1.32% | **0.89%** | — | — |

**22/22 → 22/22 recall, every corpus equal or better on over-fire**, and the
full 39-corpus run holds the headline at **639/746 = 85.7%** against its ≥85%
gate. Nothing here was bought by trading recall away.

## Method

Repository `mse-org/mseide-msegui` at `4233521f2`, fitted at
`merge-base(main, origin/sdl2)` so no branch trains the model that judges it.
Configuration as committed in the fork, **with no rule disabled**. Seven real
changesets: the five branches the project actually carries (`sdl2`, `X11_xcb`,
`X11_clean`, `sieghard`, `powerpc_test`) and the two Wayland demo branches.

```sh
argot fit
for b in sdl2 X11_xcb X11_clean sieghard powerpc_test; do
  argot check "$(git merge-base main origin/$b)..origin/$b"
done
argot check argot-wayland-guardrail..origin/demo/wayland-ai-oneshot
argot check argot-wayland-guardrail..origin/demo/wayland-in-voice
```

## Where the count started and where it ended

The first run of this sweep, on the released binary with the two disabled rules
switched back on, produced **77 findings**. Seven defects later it produces
**32**, and the 45 that went were all argot's fault, not the repo's. The defects
and their fixes are in `.scratch/overnight-log.md` §02–§10 and in the three
commits on `fix/status-repo-flag`.

| changeset | at the start | now |
|---|--:|--:|
| sdl2 | 13 | 8 |
| X11_xcb | 3 | 4 |
| X11_clean | 6 | 4 |
| sieghard | 49 | 12 |
| powerpc_test | 1 | 0 |
| demo/wayland-ai-oneshot | 3 | 3 |
| demo/wayland-in-voice | 2 | 1 |
| **total** | **77** | **32** |

---

## 1. `sdl2` — a full GUI backend, +13 869 / −78 over 30 files

The closest thing in this repository to the platform port the guardrail is
meant for. Opened as PR #102 and #161, closed both times.

| # | rule | where | verdict |
|---|---|---|---|
| 1 | redundant | `sdl/msesysintflinux.inc:1293` duplicates `setsighandlers` (`linux/msesysintf.pas:1491`), 0.94 | **true** |
| 2 | redundant | `sdl/msesysintfwin32.inc:1091` duplicates `findservers` (`windows/msesysintf.pas:1250`), 0.91 | **true** |
| 3 | redundant | `sdl/msesysintflinux.inc:1018` duplicates `stattofileinfo` (`linux/msesysintf.pas:1344`), 0.89 | **true** |
| 4 | redundant | `sdl/mseguiintf.pas:1485` duplicates `gui_docktosyswindow` (`windows/mseguiintf.pas:3467`), 0.88 | **true** |
| 5 | redundant | `sdl/mseguiintf.pas:980` duplicates `WindowProc` (`windows/mseguiintf.pas:2352`), 0.78 | **true** |
| 6 | contract-drift | `kernel/mseguiintf.inc` changed, `linux/` and `windows/` did not | **true** |
| 7 | contract-drift | `kernel/msesysintf.inc` changed, `linux/` and `windows/` did not | **true** |
| 8 | backend-contract-coverage | `sdl/mseguiintf.pas` answers 69 of 100 | **true** |

**Precision 8/8.**

### The question the brief asks: duplication, or legitimate platform mirroring?

This is the distinction that decides whether `redundant` earns its place on a
repository of platform backends, and on this branch it is **real duplication in
all five cases**. Checked side by side:

```pascal
{ sdl/msesysintflinux.inc:1293 }        { linux/msesysintf.pas:1491 }
procedure doinit;                       procedure setsighandlers;
var  info: tsigaction;                  var  info: tsigaction;
begin                                   begin
 fillchar(info,sizeof(info),0);          fillchar(info,sizeof(info),0);
 with info do begin                      with info do begin
 {$ifdef FPC}                            {$ifdef FPC}
  sa_handler:= @sigdummy;                 sa_handler:= @sigdummy;
```

Byte-for-byte, renamed. And it is not platform-specific work: POSIX signal
handling is identical on Linux whether the GUI is X11 or SDL. Same for
`stattofileinfo` (a `stat` struct converted to the repo's file info) and
`findservers`. `WindowProc` is a Win32 message-pump callback copied into an
**SDL** backend.

Legitimate mirroring is the opposite shape: `linux/` and `windows/` both define
`gui_setwindowpos`, same name, same contract, *different bodies* — because one
calls X11 and the other Win32. argot does not flag those, and the reason is
structural rather than lucky: the rule compares what the code **does**, and two
implementations of one contract against two different platform APIs do not
resemble each other. A backend that copies another backend's
platform-independent helper does.

So on this repository `redundant` separates the two cases correctly, and the
five it reports are five functions that should have been shared.

### The three sentences a maintainer gets

1. **The port stops at 69 of 100**, and among the fourteen never written are
   `gui_addpollfd` / `gui_removepollfd` / `gui_setpollfdactive` — how an
   external file descriptor joins MSEgui's event loop. Any socket-driven
   backend needs that trio before anything else.
2. **Five functions were copied** rather than shared, three above 0.89.
3. **The shared contract was extended** (`+62` lines in `mseguiintf.inc`,
   `+19` in `msesysintf.inc`) and **neither shipping backend followed** — the
   diff touches zero files under `linux/` and `windows/`.

---

## 2. `sieghard` — 9 660 insertions of contributed dialog code

| # | rule | where | verdict |
|---|---|---|---|
| 1 | rare-tokens | `db/msebufdataset.pas:37` — `FieldTypeError (0×)` | **true** |
| 2 | unfamiliar-callee | `dialogs/msedialog.pas:274` — `Application.Screenrect, SizeTy, GetObjectProp` | **true** |
| 3 | unfamiliar-callee | `dialogs/msefiledialog.pas:1312` — `doPrepareDialog, doEvaluateDialog, ReadStatOptions` | **true** |
| 4 | foreign-import | `dialogx/msefiledialogx.pas:754` — `msefiledialogxbgra_mfm` | **true** |
| 5 | rare-tokens | `dialogx/msefiledialogx.pas:775` — `ExtIcons (0×), Ext (0×)` | **arguable** |
| 6 | rare-tokens | `dialogx/msefiledialogx.pas:1548` — `LabelCol (0×), ControlIn (0×)` | **arguable** |
| 7 | unfamiliar-callee | `dialogx/msefiledialogx.pas:2211` — `StringOfChar, Max` | **true** |
| 8 | rare-tokens | `dialogx/msefiledialogx.pas:2413` — `thestrext (6×), thestrnum (6×)` | **arguable** |
| 9 | redundant | `dialogs/msefiledialog.pas:748` duplicates `filedialogx1` (`dialogx/msefiledialogx.pas:762`), 0.97 | **true** |
| 10 | redundant | `dialogx/msefiledialogx.pas:916` duplicates `filedialog1` (`dialogs/msefiledialog.pas:658`), 0.96 | **true** |
| 11 | redundant | `dialogs/msefiledialog.pas:2303` duplicates `okonexecute` (`dialogx/msefiledialogx.pas:1969`), 0.77 | **true** |
| 12 | backend-contract-coverage | `linux/mseguiintf.pas` answers 94 of 100 | **true** |

**Precision 9/12 true, 3/12 arguable, 0 wrong.**

Notes on the ones that matter:

- **#4 is the best catch on the branch.** `msefiledialogx.pas` gains
  `{$ifdef BGRABITMAP_USE_MSEGUI} msefiledialogxbgra_mfm {$else} …` — and
  `msefiledialogxbgra_mfm` **exists nowhere in the repository**, on this branch
  or on `main`. Under that define the unit does not compile. Found by the same
  fix that taught the Pascal adapter to read units out of a conditional
  `uses` clause (§05).
- **#2, #3, #7** are Delphi RTL and RTTI reached from a tree that has its own
  vocabulary for all of it: `GetObjectProp` (`TypInfo`), `StringOfChar`,
  `Max`, `ExtractFilename`. Plus CamelCase method names (`doPrepareDialog`) in
  a tree written lower-case throughout. This is contributed code in a foreign
  idiom, and saying so is the whole job.
- **#9–#11**: `lib/common/dialogx/msefiledialogx.pas` is a near-clone of
  `lib/common/dialogs/msefiledialog.pas` at 0.97 and 0.96 similarity. This is
  the "real duplication accumulated over 20 years" case, and it is real.
- **#5, #6, #8 are arguable**: new CamelCase identifiers in a lower-case
  codebase (`ExtIcons`, `LabelCol`, `ControlIn`). Consistent with the repo's
  `STYLE.md`, so a maintainer would care; but they are style, not a defect,
  and a reasonable reviewer could wave them through. Counted as arguable
  rather than claimed as wins.

**Not a finding, worth recording:** `set_fontbackgnd` and `set_Background` in
the new `msefontdialog.pas` have byte-identical bodies. `redundant` does not
report it. That is a *missed catch*, not a false alarm — out of scope here, but
it belongs on the list.

---

## 3. `X11_clean` — 49 commits of X11 backend cleanup

| # | rule | where | verdict |
|---|---|---|---|
| 1 | rare-tokens | `linux/mxlib.pas` — `os2 (1×), AnyPropertyType (1×), XLookupNone (1×)` | **true** |
| 2 | redundant | `linux/mseguiintf.pas:925` duplicates `deleteitem` (`kernel/msearrayutils.pas:991`), 0.79 | **true** |
| 3 | backend-contract-coverage | `linux/mseguiintf.pas` answers 94 of 100 | **true** |
| 4 | backend-contract-coverage | `windows/mseguiintf.pas` answers 87 of 100 | **true** |

**Precision 4/4.**

**#2 is textbook reinvention.** The branch adds `deleteitemat` to
`mseguiintf.pas`; its body is byte-identical to `msearrayutils.deleteitem`,
which the repository already has:

```pascal
 if (index < 0) or (index > high(dest)) then begin
  tlist.Error(SListIndexError, Index);
 end;
 move(dest[index+1],dest[index],sizeof(dest[0])*(high(dest)-index));
 setlength(dest,high(dest));
```

**#1** is a new raw Xlib binding surface (`XLookupNone`, `AnyPropertyType`,
`AllocNone`) entering the kernel — a widening of the dependency surface, which
is what the rule is for.

---

## 4. `X11_xcb` — the XCB experiment

| # | rule | where | verdict |
|---|---|---|---|
| 1 | foreign-import | `graphics/msex11gdi.pas:14` — `xlib` | **true** |
| 2 | foreign-import | `graphics/msex11gdi_ori.pas:14` — `xlib`, `mxrender` | **true** |
| 3 | rare-tokens | `linux/mxlib.pas` — `XLookupNone (1×), AllocNone (1×)` | **true** |
| 4 | backend-contract-coverage | `linux/mseguiintf.pas` answers 94 of 100 | **true** |

**Precision 4/4.** The branch swaps the X11 GDI onto a new `xlib` unit that the
repository has never depended on, and leaves a copy of the old file
(`msex11gdi_ori.pas`) behind. Both are exactly what a reviewer wants flagged.

---

## 5. `powerpc_test` — 7 commits, 12 files

**0 findings.** Correct, and the most important row in this table: a small
platform tweak that touches nothing foreign produces silence. Before the
supersession fix it produced one finding, and that finding was wrong.

---

## 6. The two Wayland demos

`demo/wayland-ai-oneshot` — a backend written one-shot the way a language model
writes Object Pascal (Delphi CamelCase, `TWaylandWindow = class(TObject)`,
`TStringList`, `WriteLn`, static externs):

| # | rule | where | verdict |
|---|---|---|---|
| 1 | foreign-import | `wayland/mseguiintf.pas:11` — `cmem`, `unixtype` | **true** |
| 2 | platform-backend-contract | does not `{$include ../mseguiintf.inc}` | **true** |
| 3 | backend-contract-coverage | answers 5 of 100 | **true** |

`cmem` is the one to notice: it appears in standalone Wayland demos constantly,
and MSEgui the library never uses it — pulling it into the kernel swaps the
memory manager process-wide.

`demo/wayland-in-voice` — the same unit rewritten in the house idiom
(lower-case, `msedynload` + `funcinfoty`, `msectypes`, `{$packrecords c}`):

| # | rule | where | verdict |
|---|---|---|---|
| 1 | backend-contract-coverage | answers 7 of 100 | **true** |

**Precision 4/4 across both.** Two of the three findings disappear when the
idiom is fixed, and the one that remains is the progress bar, not a defect.

---

## Precision per rule, over all seven changesets

| rule | n | true | arguable | wrong |
|---|--:|--:|--:|--:|
| redundant | 9 | 9 | 0 | 0 |
| backend-contract-coverage | 7 | 7 | 0 | 0 |
| rare-tokens | 6 | 3 | 3 | 0 |
| foreign-import | 4 | 4 | 0 | 0 |
| unfamiliar-callee | 3 | 3 | 0 | 0 |
| contract-drift | 2 | 2 | 0 | 0 |
| platform-backend-contract | 1 | 1 | 0 | 0 |
| superseded | 0 | — | — | — |
| layering | 0 | — | — | — |
| misplaced | 0 | — | — | — |
| integrity | 0 | — | — | — |
| **total** | **32** | **29** | **3** | **0** |

**29 of 32 true, 3 arguable, 0 wrong** — 91% true, 100% not-wrong.

The three `rare-tokens` counted true are the two on `mxlib.pas` (a new raw Xlib
binding surface entering the kernel, on both X11 branches) and
`FieldTypeError` on `msebufdataset.pas` (a new unit whose CamelCase name the
tree never uses). The three arguable are the other sieghard hits — new
CamelCase identifiers in a lower-case codebase, consistent with the repo's own
`STYLE.md` but style rather than defect. Every one
of the seven defects that produced the original 45 false findings is fixed in
argot and pinned by a test; none is muted, and the fork's `argot.toml` disables
no rule.

---

## 7. The 400-commit sweep — five years of accepted history

```
$ argot audit --commits 400
  423 commits · 2 489 hunks · 25 findings · 0% carry AI markers
```

**25 findings over 2 489 hunks — 1.0%.** On the released binary the same sweep
produced 32, with `layering` silent and one `superseded` false alarm.

| rule | n | true | arguable | wrong |
|---|--:|--:|--:|--:|
| foreign-import | 8 | 8 | 0 | 0 |
| redundant | 8 | 7 | 1 | 0 |
| rare-tokens | 5 | 2 | 3 | 0 |
| unfamiliar-callee | 3 | 2 | 1 | 0 |
| layering | 1 | 1 | 0 | 0 |
| misplaced | 0 | — | — | — |
| superseded | 0 | — | — | — |
| **total** | **25** | **20** | **5** | **0** |

### Every widening of the dependency surface in five years, with its commit

All eight `foreign-import` findings are the *first* use of a module in this
repository's history, and all eight are true:

| module | where it entered |
|---|---|
| `bgrabitmap`, `bgrabitmaptypes`, `bgradefaultbitmap` | `dialogx/msefiledialogx.pas` |
| `cairo` | `kernel/linux/mcairoxlib.pas` |
| `mshape` | `kernel/linux/mseguiintf.pas` |
| `mx`, `mxutil` | `opengl/mseopengl.pas` |
| `process` | `apps/ide/make.pas` |
| `gettext` | `tools/POtools/MOdemo/modemo.pas` |
| `streamio` | `tools/POtools/POdemo/POtoMO.pas` |
| `msecwstring` | `kernel/linux/msesetlocale.pas` |

The first row is the one to show a maintainer: **a third-party graphics library
entered the tree through a file-dialog commit**, and the same commit's
`unfamiliar-callee` names the API it brought (`loadimagebgra`, `loadimage`).

### The `layering` finding — and it is real

```
layering · lib/common/graphics/msegraphics.pas
    ↳ editwidgets → graphics is this repo's direction — this import reverses it
```

Checked by hand: `msegraphics.pas:1453` has an implementation-section `uses`
pulling `mseedit` and `msegraphedits`, both under `lib/common/editwidgets/`.
The learned graph has `editwidgets → graphics` **15×** and `graphics →
editwidgets` **1×** — this is that one. A genuine layering inversion in the
repository's own history, and before the container-descent fix argot could not
see it, because every unit in the tree was in the same layer.

### `redundant` — the same near-clone pair, seven times

Seven of the eight are `lib/common/dialogs/msefiledialog.pas` ↔
`lib/common/dialogx/msefiledialogx.pas` at 0.96, 0.92, 0.89, 0.89, 0.88, 0.87,
0.87 (`filedialog1`, `filedialog` ×2, `backexe`, `listviewitemevent`,
`formoncreate`, `onformcreated`). One pair of files, seven duplicated
functions — the clearest single piece of technical debt the sweep surfaces.

The eighth is **arguable**: `msedbgraphics.storebitmap` against
`msebitmap.writetostream` at 0.86. The shared part is real (create a
`tmsefilestream`, `writegraphic`, rewind, free in a `finally`), the purpose is
not — one writes out, the other stores into a DB field. A maintainer could
reasonably extract the stream dance, or reasonably leave it.

### The five arguable ones, named

- `rare-tokens` on `msesysintf.pas` (`Filecreate (0×)`, `Fileopen (0×)`) and on
  `msedbedit.pas` (`tarightjustify (1×)` beside `textflags (164×)`): CamelCase
  RTL spellings and mostly-attested identifiers. Style, not defect.
- `rare-tokens` on `msefiledialogx.pas` (`dylib (0×)`, `zip (1×)`, `pyc (2×)`):
  file-extension literals — data the dialog recognises, not idiom.
- `unfamiliar-callee` on `tools/POtools/MOdemo/mo2arrays.pas` (`ParamStr`,
  `FindClose`, `system.pos`): RTL basics that genuinely appear nowhere else in
  the tree, because this is a standalone tool. Correct observation, thin value.

The two `rare-tokens` counted **true** are the ones that name new API surface
entering the kernel: `XSetErrorHandler`, `WhitePixel`, `wo_rounded` (X11) and
`dragonfly`, `ptm_magic`, `ptm_interlock` (libc internals plus a new BSD
target).

---

## Both sets together

| | findings | true | arguable | wrong |
|---|--:|--:|--:|--:|
| seven changesets | 32 | 29 | 3 | 0 |
| 400-commit sweep | 25 | 20 | 5 | 0 |
| **total** | **57** | **49** | **8** | **0** |

**86% true, 14% arguable, 0 wrong.** Nine defects in argot were found and fixed
getting here; not one finding is muted, and the fork's `argot.toml` disables no
rule.

---

## The three rules that fire zero times, and why that is honest

- **`superseded` — 0.** It fired six times out of seven before the coverage
  gate, always on the same 2017 rename that never propagated. With the gate the
  miner finds **no** live migration in this repository, which is the truth: 272
  of 508 files still import `msestrings` nine years on, and there is no
  migration to enforce.
- **`layering` — 0**, but no longer blind. Layer detection now resolves the
  tree to **27 layers and 183 edges** (it was 3 and 5), and the topology it
  finds is the real one — `kernel` the largest sink, `graphics → kernel`,
  `widgets → kernel/graphics`, `editwidgets → widgets/graphics/kernel`,
  `db → kernel`, `apps` on top. None of these seven changesets introduces an
  edge that reverses it, which is a defensible zero: they are all work *inside*
  `kernel/` and `dialogs/`, not new cross-layer wiring. That it *can* fire here
  is verified rather than assumed — one `uses` added to a kernel unit:

  ```
  $ argot check          # after adding `,msefiledialog` to kernel/msestatfile.pas
  error · layering · lib/common/kernel/msestatfile.pas:L13 · unusual
      ↳ dialogs → kernel is this repo's direction — this import reverses it
  ```

  Before the fix this produced nothing at all, on any input, because every unit
  in the tree was in the same layer.
- **`integrity` — 0**, and this one is honest by construction: the repository
  has no test suite, so there is nothing for the rule to protect. Nothing to
  fix in argot; it is a property of the repo, and the product should say so
  rather than imply the rules passed.
- **`misplaced` — 0**, after two gates that are both principled and both
  costly. A body that calls nothing has no architectural home to judge, and a
  callable recovered from inside a parse error has no known parent. The second
  costs real coverage here and the number is worth stating: the tree-sitter
  Pascal grammar fails at `lib/common/db/msedb.pas:1892` and the error region
  runs to the end of the unit, so **3 530 of 15 078 extracted functions —
  23.4%, across 111 of 333 files — are no longer judged for placement.** That
  is the honest reduction rather than a silent one: their structural context
  was never known. It also names the follow-up worth more than any tuning,
  since the same gap costs `redundant` the same quarter of the tree.
