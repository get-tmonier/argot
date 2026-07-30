# tree-sitter-pascal — argot's fork

Upstream: <https://github.com/Isopod/tree-sitter-pascal> at `042119e` (v0.10.2),
MIT. `LICENSE` is upstream's, unchanged.

## Why a fork

Object Pascal was losing **~29 % of every line in the benchmark to an `ERROR`
node** — a third of the language invisible to every rule (imports, callees,
shape, placement), against 0,37 % for TypeScript and C. Roughly half of that was
argot's own directive handling and is fixed in `argot-lang`; the other half is
missing rules here, in constructs that mORMot, Castle Game Engine, MSEide/MSEgui
and uos use on nearly every page.

Nothing was invented: every rule below has a one-line reproduction taken from a
real corpus file, and upstream's own 88-case test corpus still passes unchanged.

| what | reproduction |
|---|---|
| property **redeclaration**, with or without `default` | `property Name;` · `property color default 1;` |
| **qualified** property accessor | `property PendingRead: PtrInt read fRd.Len;` |
| routine directive with **no semicolon before it** | `procedure Foo() override;` (MSEide writes every method this way) |
| bare **re-raise** | `raise;` inside an `except` block |
| **anonymous inline record** as a field or element type | `Union: record … end;` · `array[1..9] of record … end` |
| **calling convention on a function-typed field** | `xCreate: function(…): integer; cdecl;` |
| set over an ordinal **range** | `set of 0..31` |
| Delphi **codepage-parameterised** string type | `W = type AnsiString(1252);` |
| declaration **hint closing a class** | `T = class … end deprecated;` |
| Delphi **parameter attribute** | `procedure D(…; [ref] const Source: TVarData);` |
| project file naming a unit's **path** | `uses server in 'src\server.pas';` |
| **last field of a list** with no terminating `;` | `status: culong` immediately before `end;` |
| **labelled empty statement** closing a block | `endlab:` immediately before `end;` |
| `.inc` **include fragments** — see below | a bare list of methods, properties and `strict private` markers |

### The include fragment

An `.inc` file is pasted into the middle of another unit, so it is a *fragment*,
not a compilation unit: it routinely opens inside a class body. Upstream's `root`
already admitted `_definitions` for include files, which covers what may stand at
*unit* level and not what may stand at *class-member* level. Castle Game Engine
writes 1 015 of them and they were 92 % of everything the parser could not read
there. `root` now admits `declProp` and `declSection` alongside `_definition`.

`declField` is deliberately **not** in that set, and `declVars`/`declConsts`/
`declTypes` keep their `repeat` rather than `repeat1`: both changes were tried,
and because those sections admit zero entries, an eager `declField` takes
`var x: integer;` apart into an empty `var` section and a loose field. That
regressed 9 of upstream's tests for ~0,7 % of one corpus. `declSection` is
`prec.right` because inside a class the closing `end` bounds a section and at the
root of a fragment nothing does.

## Measured

Share of lines inside an `ERROR` node, over the real fit corpus
(`collect_source_files`, so each repository's own `argot.toml` applies). The
middle column is argot's directive fix alone, the last adds this grammar.

| corpus | before | + directive fix | + this grammar |
|---|--:|--:|--:|
| mormot2 | 29,16 % | 19,14 % | **8,49 %** |
| castle-engine | 29,09 % | 28,42 % | **8,93 %** |
| mseide-msegui | 16,14 % | 11,87 % | **7,41 %** |
| uos | 29,52 % | 6,74 % | **1,62 %** |

Three more Pascal corpora nobody had measured: ideu 4,01 %, strumpract 0,04 %,
swp 0,05 %.

### The two rules added 2026-07-30

Two constructs still cost mseide-msegui almost everything the earlier work had
saved, because each one desynchronises the parser for the *rest of the unit*:
a record whose last field omits its `;`, and a `goto` label sitting on the empty
statement before `end`. Both are ordinary Object Pascal and both are on nearly
every page of that codebase.

Measured over its 493-file fit corpus, 548 418 lines, through argot's real
pipeline (directive handling included), counting each file's widest `ERROR` span:

| | lines inside an `ERROR` | share |
|---|--:|--:|
| v0.2.118 as released | 36 136 | **6,59 %** |
| + these two rules | 260 | **0,05 %** |

The three worst files were lost whole and now parse clean:
`lib/common/db/msebufdataset.pas` (10 979 lines), `lib/common/report/msereport.pas`
(8 738), `lib/common/kernel/linux/mseguiintf.pas` (7 452 — 1 921 `ERROR` nodes,
21 of its 102 routines recoverable; now 0 and all of them).

Guarded by `a_final_record_field_may_omit_its_semicolon` and
`a_block_may_end_on_a_labelled_empty_statement` in
`crates/argot-lang/src/ts_parse/tests.rs`; both fail against the released
grammar. Upstream's 88-case corpus still passes unchanged.

## Regenerating

`src/parser.c` is generated and committed, so the build needs only a C compiler —
the same shape as every other tree-sitter crate in this workspace. Node is needed
only to regenerate, and only by whoever edits `grammar.js`:

```sh
cd vendor/tree-sitter-pascal
npm install tree-sitter-cli@0.25
./node_modules/.bin/tree-sitter generate   # must report no conflicts
./node_modules/.bin/tree-sitter test       # upstream's corpus: 88 pass, 0 fail
```

Then re-run argot's own guard: `cargo test -p argot-lang --lib`. (Not
`--lib pascal`: the parse-level guards live under `ts_parse::tests`, which that
filter does not match.)

Upstream ships a `parser.c` built by an older CLI (ABI 14, 2 715 states) than its
own `grammar.js` implies; regenerating alone changes neither the accepted
language nor any of the failures above, so do not expect it to.

## Sending this upstream

These are genuine gaps in a maintained grammar and belong upstream. Nothing here
is argot-specific, and every rule carries a reproduction ready to become a test
case.
