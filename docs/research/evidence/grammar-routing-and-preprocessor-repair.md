# The parser was losing whole files — three grammar defects, measured across 12 languages

**Date:** 2026-07-28 · **Status:** positive — three defects found and fixed, one
latent bug caught on the way, every language re-measured.

**Question:** the Pascal directive fix recovered 1 574 functions in MSEide/MSEgui
by blanking `{$…}` before parsing. Pascal is not special — is anything else
losing code to the same class of failure? And if so, how much?

## Method

For every catalogued corpus, parse each source file with the grammar `check`
routes it to, then count the distinct lines that fall inside an `ERROR` node.
A line inside an `ERROR` is a line no rule can read: imports, callees, shapes
and placement all go blind behind it.

```
lines in ERROR / total lines, per language, on the corpus argot benchmarks with
```

## What the sweep found

| language | corpus | lines lost | widest ERROR |
|---|---|--:|--:|
| typescript | excalidraw | **31,53 %** | 12 969 |
| pascal | mormot2 | **30,45 %** | 14 131 |
| c | curl | **9,33 %** | 3 348 |
| cpp | rocksdb | **4,90 %** | 6 365 |
| pascal | castle-engine | 2,53 % | 7 644 |
| python, javascript, go, rust, csharp, php, ruby | — | **0,00 %** | 0 |

Seven of twelve languages lose nothing. The four that do lose a lot, and each
for a different reason.

## Defect 1 — TSX is a separate grammar, not a superset

`LANGUAGE_TYPESCRIPT` cannot read JSX. `.tsx` files route to `Language::Typescript`
like `.ts`, so **191 of excalidraw's 200 `.tsx` files failed to parse**, against
1 of 200 `.ts`. One error node spanned 12 969 lines.

The extension is not carried into `parse`, so the grammar is chosen by outcome:
parse with TypeScript; if that reads the file, keep it. Only a file TypeScript
could not read is retried with `LANGUAGE_TSX`, and TSX is kept only if it does
better. A `.ts` file parses cleanly the first time and never pays for this.

The tie-break matters in both directions: `const el = <HTMLInputElement>x;` is
a TypeScript type assertion that TSX reads as a JSX element, so a tie keeps
TypeScript. Both directions are pinned by `jsx_is_read_by_the_tsx_grammar`.

**31,53 % → 0,37 %.** Widest error node 12 969 → 571 lines.

Plain JavaScript was checked too and needs nothing: the base grammar reads JSX,
so `.jsx` was never affected.

## Defect 2 — a preprocessor conditional swallowed the file

C and C++ conditionals are only representable where a declaration may stand.
Inside a constructor initialiser list or a parameter list they are unparseable,
and the failure is not local. On `curl.h` a single unrepresentable conditional
turned the **entire 3 347-line header into one `ERROR` node**.

Three distinct shapes, all real:

```c
// rocksdb/db/db_impl/db_impl.cc:189 — inside an initialiser list
#ifdef COERCE_CONTEXT_SWITCH
      mutex_(stats_, immutable_db_options_.clock, ...),
#else
      mutex_(stats_, immutable_db_options_.clock, ...),
#endif
```

The fix blanks the conditional *control* lines — `#if`/`#ifdef`/`#ifndef`/
`#elif`/`#else`/`#endif` — byte for byte, and keeps both branches. `#define`
and `#include` are deliberately kept: they declare the names and dependencies
the scorers read.

Keeping both branches means a file can declare the same name twice. That costs
nothing here — the scorers read names and shapes, and a duplicate is what a name
already seen looks like. Blanking byte for byte rather than character for
character matters because a directive comment may hold a multi-byte character,
which would otherwise shift every offset in the tree.

Like the TSX fix it is outcome-based: a file that parses is never rewritten.

## Defect 3 — `.h` routed by repo majority is wrong for the minority

`.h` is C or C++ and the extension cannot say which. `ext_to_lang_ctx` resolves
it with a repo-level translation-unit majority, which is right for the
repository and **necessarily wrong for its minority headers**. rocksdb is a C++
project with C headers in it, and under the C++ grammar `xxhash.h` loses 2 402
lines where the C grammar loses 274 — nine times worse.

Same outcome-based rule: only a file the routed grammar could not read is
retried with the sibling grammar, and the sibling is kept only if it reads more
of the file. A C++ source cannot be mistaken for C, because the C grammar
cannot read `class` or `namespace` and scores far worse on the same comparison.

## Results

| language | corpus | before | after |
|---|---|--:|--:|
| typescript | excalidraw | 31,53 % | **0,37 %** |
| c | curl | 9,33 % | **0,37 %** |
| cpp | rocksdb | 4,90 % | **1,95 %** |

curl's remaining 0,37 % and rocksdb's 1,95 % are the honest floor of parsing C
without a preprocessor. **77 % of what rocksdb has left is vendored
`third-party/gtest`** — excluding it, rocksdb's own code sits at ~0,45 %.

## The latent bug this uncovered

Choosing a grammar by outcome means `parse` may return a tree built with a
grammar the language does not nominally map to. The scripted-rules host compiled
its tree-sitter query against the *nominal* grammar:

```rust
let ts_lang = argot_lang::ts_parse::ts_language(lang);   // wrong tree
let Ok(compiled) = tree_sitter::Query::new(&ts_lang, query) else { ... };
```

A query compiled against one grammar's symbol table and run over another's tree
matches nothing, and says so silently — a community rule would simply stop
firing on `.tsx` files with no error anywhere. Fixed by asking the tree which
grammar built it (`tree.language()`), which makes the mismatch unrepresentable
rather than merely unlikely.

Everything else in the workspace compares `node.kind()` strings, which is
grammar-independent and was never at risk. `ts_query` was the only site.

## Measured and rejected

**Blanking macro qualifiers.** `CURL_EXTERN CURLcode curl_easy_setopt(…)` and
`XXH_PUBLIC_API void XXH32_copyState(…)` fail to parse: one unknown macro before
a return type is enough, and it is an everyday C-header idiom. Blanking a
leading run of ALL-CAPS identifiers in declaration position fixes the synthetic
case exactly, and corpus-wide it is worth **0,08 pp on curl (0,37 → 0,29) and
0,01 pp on rocksdb**.

Rejected. It is a guess about identifier casing rather than a fact about the
language — nothing distinguishes a macro qualifier from a constant by spelling
— and it can blank real code for a marginal gain. The two fixes that ship are
statements about the grammar, not about how a project names things.

**curl's residual** is a macro invocation used as an enum entry, with arguments
that are not C tokens at all (`CURLOPTDEPRECATED(CURLOPT_PUT, …, 7.12.1, …)` —
`7.12.1` is not a literal in any C grammar). Nothing short of a preprocessor
reads that, and it is a data table rather than logic.

## What is left

`mormot2` at 30,45 % is untouched by any of this: it is Pascal, already past
the directive fix, and its errors are a different construct. It is the largest
remaining parse loss in the benchmark and is written down rather than guessed at.

## The lesson worth keeping

The Pascal directive fix looked like a Pascal problem. It was an instance of a
general one — *the grammar cannot represent this construct, and the failure is
not local* — and asking the same question of the other eleven languages found
two more, one of them larger. A per-file `has_error` says nothing; **the share
of lines behind an `ERROR` node** is the number that ranks them, and it is cheap
to compute for every language at once.
