# Issue #92 — per-corpus false-alarm reduction

**Date:** 2026-07-04 · **Branch:** `bench/92-temporal-holdout`. Follows the
[foreign-reach gate](issue92-foreign-reach-gate.md) and
[honest re-bench](issue92-honest-rebench.md). Goal: drive existing-file FP ≤2%
and new-file ≤5% on **every** corpus while holding the gated novel-pattern catch
at 48/49 (98%). Diagnose the exact driver of each residual block before touching
code; measure catch AND FP together; revert anything that costs a gated catch.

Baseline (commit 16018c1f, honest, 27 corpora): catch 48/49; existing FP 2.00%
aggregate but 8/27 corpora >2%. Residual 429 existing FP by stage: call_receiver
195, import 131, convention 54, bpe 49.

## 1 — Import stage: a bug + a mass-migration de-noiser

Reproduced fastapi's 59 and excalidraw's 24 import false alarms (holdout JSON →
git blob → the exact foreign module per hit, cross-checked against the fit-time
`import_modules` snapshot).

**(a) BUG — relative import mis-flagged foreign under error recovery.** Two
fastapi hits fired `import` on `fastapi/utils.py` whose only import is the
*relative* `from ._compat import v2`. Tree-sitter's error recovery on the
mid-function diff fragment splits the line and re-parses its tail `import v2` as
a standalone `import_statement` starting mid-line (col 22, not the line's indent
col 8) — leaking the imported **symbol** `v2` as a phantom top-level module. Fix:
`node_starts_line` guard in the Python adapter — an import statement is only
trusted when it owns the start of its line. (`crates/argot-core/src/scoring/adapters/python.rs`)

**(b) Mass-migration flooding.** The rest are genuine new third-party deps, but
they cluster: one commit adds the same foreign import across many files (excalidraw
`radix-ui` ×23 in one commit; fastapi `annotated_doc` across dozens). Each file
fires independently → the per-hunk FP rate is inflated by a single mechanical
decision. Fix: **per-changeset novel-import dedup** — a foreign dependency alerts
on its first appearance in a check run; later import-only hunks whose foreign
module set is already alerted are deduped to a non-fire (a genuinely new module
still fires). This is correct guardrail UX (one alert per novel dependency per
PR) and a no-op for recall (each catalog fixture is checked in its own run).
(`crates/argot-core/src/scoring/sequential.rs` exposes `foreign_import_modules`;
`crates/argot-core/src/check.rs` dedups per run.)

**REJECTED — ecosystem-popularity dampener via the shipped generic baseline.**
The plan was to dampen genuine-new-dep imports by how ubiquitous the package is
in the shipped `generic_tokens_bpe.json` prior. Measured directly: the baseline
is **BPE subword** token counts, and module names decompose into common subwords.
`annotated_doc` (niche) → tokens `annot`/`ated`/`_`/`doc` → mean 7909 ppm ("very
popular"); `typing_inspection` (niche) → 8012 ppm; while `express` (mainstream)
→ 0 ppm, `react` → 0.44 ppm, `numpy` → 1.1 ppm. No separation — the proxy ranks
niche packages as popular and mainstream ones as rare. There is no module-level
popularity data in the shipped artifact and no way to derive one without new data
or the network (both barred). Not implemented.

## 2 — call_receiver: a member-access fix; two rejected sweeps

Reconstructed rocksdb's 81 and ink's 36 call_receiver hits (fit at the holdout
SHA, replay each firing commit, dump the foreign-reaching callee).

**rocksdb** over-fires on C++ **member-variable method calls** — `bm_.StopSecondaryUpdateThread`,
where the `::`→`.` normalization (namespace vs member access collapse) makes the
member receiver `bm_` read as an unknown namespace. One such callee opens the
file-level foreign-reach gate for every hunk in the file. Fix: capture C++
`field_declaration` names (class/struct member fields) in `value_bindings`
(`crates/argot-core/src/scoring/adapters/cpp.rs`) so member receivers are
recognised as local. Limited by scope: fields declared in **headers** are invisible
to the per-file check, so the fix only recovers members declared in the same
`.cc` — a partial reduction, not a cure. The residual is internal
types/namespaces (`AlignedBuffer::`, `trie_index::`) declared cross-file — the
frozen model cannot tell a new *internal* namespace from a new *external* one
without a fit-time repo-type snapshot (future work).

**ink** over-fires on JavaScript/Node **builtins** the frozen model had not yet
seen called — `setImmediate` (×28), `parseInt` (×14), `URL`, `JSON.parse`, `Set`,
`TextEncoder`, `clearImmediate`. **bat** is a genuine `git2`→`gix`/`minus`/`tempfile`
**dependency migration** (arguably correct to flag) plus Rust std (`Cow::`,
`VecDeque::`, `mem::take`, `format_args!`). Both are the "language stdlib/builtin
looks foreign" class. **No data-free discriminator separates a ubiquitous
language builtin from a foreign library:** the catch-critical foreign APIs have
equally common-looking bare names (`event_base_new`, `strcat`, `setcookie`,
jQuery `$`). Left as documented residual.

**REJECTED — drop bare-unattested callees from the foreign-reach gate.** The
hypothesis was that a bare unattested callee (no `::`/import) is the repo's own
new code, not foreign. Measured: it **killed 14 catches across 7 corpora** — the
foreign-API classes are routinely called bare with no distinct import (`setcookie`,
`strcat`, jQuery `$`, `uthash`, `event_base_new`, `foreign_http`). The bare path
is load-bearing for catch. Reverted immediately.

**REJECTED — fit-time repo-defined-callable snapshot.** The refined version:
keep bare-foreign for catch, but exclude bare callees the repo *defines* anywhere
in its corpus (`ResetState`, `rocksdb_*_create` — rocksdb's own methods, called
bare across `.cc`). Implemented (`CallReceiverModel.defined_callables`, populated
from `callable_definitions` over the corpus); catch held (redis 8/18, rocksdb
7/17). But **zero FP effect** on any corpus: the foreign-reach gate is *file-level*
(any one foreign callee opens it for every hunk), and rocksdb files always retain
a residual foreign callee the snapshot doesn't cover — an uncaptured method
(`GetDbPath`), a qualified internal type (`AlignedBuffer::isAligned`), or a Python
builtin (`SystemExit`). Recognising the bare internal calls doesn't close the
gate. Reverted (11k-string-per-corpus model bloat for no measured benefit).
Closing rocksdb needs the gate itself to stop amplifying one foreign callee to
the whole file, plus a repo-type/namespace snapshot for the qualified cases —
larger work than this pass.

## 3 — Convention stage: demoted to automatic-off

jellyfin's 22 convention FP **all** fire on **syntax surprisal at ratio 1.185**
(a rare C# construct — pragma directives, tuple syntax, Hungarian properties —
exceeding the calibration sample's max bar by 18%; pure temporal drift, not a
dependency reach). The two catches the stage carries — `faker_js_threading_2`,
`background_tasks_1` — fire at **lower** ratios (1.067, 1.119): the FP are
*stronger* than the catches, so no margin/threshold separates them. Both catches
are `other`/legacy-tier fixtures, **not** in the gated novel-pattern set
(`foreign_import`/`foreign_api`/`foreign_concurrency` — RUBRIC), so demoting the
stage leaves the gated 48/49 headline intact.

Convention is now **off by default and automatic** — no user-facing flag. It
survives as an internal `CalibrateOptions.enable_conventions` (default false)
the benchmark flips to measure the trade-off; production `fit`/`check` never
expose it. (`crates/argot-core/src/scoring/calibration.rs`)

## Results — honest re-bench, all 27 corpora

| Metric | Baseline (16018c1f) | Now |
|---|---|---|
| Gated novel-pattern catch (`foreign_import`/`api`/`concurrency`) | 48/49 (98%) | **48/49 (98%)** — held |
| Existing-file FP (aggregate) | 2.00% (429/21416) | **1.38% (296/21416)** |
| Corpora ≤ 2% existing FP | 19/27 | **23/27** |
| Gated corpora ≥ 85% | 8/8 | 8/8 |

**Zero FP regressions** (no corpus got worse). New passes: jellyfin 6.79→1.81
(convention demote), fastapi 5.18→2.21, rich 2.30→0.51, excalidraw 2.76→1.04,
fmt 2.41→1.42; homebrew 4.59→0.22, guava 1.16→0.74, hono 0.28→0.00, hugo/junit5
also down. rocksdb new-file FP 40.0→20.0.

**Catch:** gated 48→48. Two `other`/legacy fixtures dropped
(`saleor/foreign_http_1`, `dagster/dagster_py_validation_2`) — but both were
*spurious* baseline catches: they fired `import` score 1.00 only via the phantom
`from __future__ import annotations` → mid-line `import annotations` the
error-recovery guard now rejects. `urllib` is attested in saleor
(`from urllib.parse import …` ×40), and the dagster fixture is a *semantic*
validation break — neither is a real foreign import. The guard removing them is
a correctness fix, not a novel-pattern regression. The two convention-carried
catches (`faker_js_threading_2`, `background_tasks_1`) still fire — via
`call_receiver` once the convention bonus is gone — so demoting convention cost
zero catches.

**Residual > 2% existing (4 corpora), all call_receiver-dominated:**

| corpus | FP | driver |
|---|---|---|
| ink | 10.58% (cr 36) | JS/Node builtins first-used post-fit (`setImmediate`, `parseInt`, `JSON`, `Set`) |
| bat | 8.58% (cr 24) | genuine `git2`→`gix`/`minus` dependency migration + Rust std (`Cow`, `VecDeque`, `mem`) |
| rocksdb | 4.04% (cr 78) | cross-file C++ internal types/namespaces (`AlignedBuffer::`, `trie_index::`) — header-declared, invisible per-file |
| fastapi | 2.21% (cr 18 + import 14) | `annotated_doc`/`pwdlib` adopted across separate post-fit commits (frozen model) |

These are the "language stdlib/builtin or genuine dependency adoption looks
foreign" class. No data-free discriminator separates them from the
catch-critical foreign APIs, which have equally common bare names (proven: the
bare-drop sweep that would silence them cost 14 catches). Closing them needs a
fit-time repo-type/namespace snapshot (rocksdb) or a shipped per-language
builtin/ecosystem-callee prior (ink/bat) — neither derivable from the current
artifacts without new data. Documented, not chased.

## Follow-up: repo-declared symbols + a hunk-level foreign-reach gate

The rocksdb residual (4.04%, call_receiver-dominated) was two coupled bugs:

1. **The frozen model doesn't attest the repo's own cross-TU symbols.** rocksdb
   calls hundreds of its own bare functions (`ResetState`, `rocksdb_*_create`)
   and its own types (`AlignedBuffer::isAligned`, `trie_index::…`) across `.cc`
   files. None were *called* at fit, so none are in `attested`, so they read as
   foreign. Fix: a fit-time `CallReceiverModel.defined_symbols` snapshot — every
   name the repo **declares** (functions, methods, classes, structs, enums,
   traits, via each adapter's `callable_definitions`). A bare callee that is a
   declared name is not foreign; a `Type::method` / `ns::func` whose leading
   segment is a declared type/namespace is not foreign. Foreign libs
   (`event_base_new`, `absl::`, `boost::`) are never repo-declared, so they still
   fire. (Rust `callable_definitions` was extended to capture `struct`/`enum`/
   `union`/`type`/`trait` names, which it had missed.)

2. **The foreign-reach gate was file-level — one foreign callee cried wolf across
   the whole file.** `defined_symbols` alone did nothing (measured): a file
   always retains *one* uncaptured foreign-looking callee (a template method, a
   header-only type), and the file-level gate then flagged every hunk whose new
   code was entirely the repo's own. Fix: check foreign reach at **hunk**
   granularity — the hunk's own callees, not the whole file's. The file-level
   foreign *import* condition stays (a foreign `#include` legitimately colours the
   file — libevent), so cross-hunk import→call catches are unaffected.

**Beware concurrent benches on one corpus clone.** An early "rocksdb 0.04%"
reading was a race: a holdout and a production run fit into the same
`benchmarks/data/rocksdb/.repo/.argot` simultaneously and corrupted the model.
Two isolated runs both reproduce the real numbers. Never run two benches that
touch the same corpus at once.

**Result — honest re-bench, all 27 corpora (both changes):** existing-file FP
**1.38% → 1.05%** (296→224), corpora ≤2% **23 → 24** (rocksdb 4.04→1.52 now
passes), gated novel-pattern catch **48/49 held**, **zero catches lost anywhere**,
zero FP regressions. ink 10.58→8.73 and bat 8.58→7.40 also improved (the gate
de-amplifies their dependency-migration spikes) but remain > 2% — genuine
dependency/builtin adoption, the inherent limit.

**REJECTED — a per-language stdlib/builtin set.** ink's residual is JS/Node
globals (`setImmediate`, `parseInt`, `JSON`, `Set`), bat's is Rust std (`Cow::`,
`mem::take`). Seeding the foreign-reach check with a curated per-language builtin
set (language grammar, not corpus knowledge) is the obvious fix. Measured: ink
8.73→7.41, bat 7.40→6.80 — modest, and **both still fail** (the rest of their
over-fire is the repo's own new functions called across commits, which no
static set covers). Worse, it **cost a catch**: ripgrep `conc_busywait_1` (a
busy-wait using std concurrency primitives) — marking `thread`/`time`/`Duration`
as builtin suppressed the very std call the concurrency break rides on. The
`language builtin` and the `wrong_concurrency`/semantic catch classes both live
in the standard library and cannot be cleanly separated. Reverted: marginal
gain, catch cost, and embedded literals for a problem it doesn't solve.

The token-frequency **popularity proxy was re-tested at the callee level** too
(not just modules) and fails identically: `parseInt`/`Promise` (globals) score 0
ppm while `event_base_new`/`setcookie` (foreign) score 150+/38 ppm — the shipped
BPE baseline cannot separate a language builtin from a foreign API by frequency.

ink/bat/fastapi remain the inherent limit: genuine new dependencies, language
builtins, and the repo's own new functions across commits — all of which a
frozen, never-re-fitted model correctly reads as "not seen before."

**REJECTED — base-repo / incremental attestation.** The proposed methodology
fix: attest imports/callees the repo already uses in ≥K files of the state the
change applies to (leak-safe — the PR's base), so an adopted dependency goes
quiet after the first PR. Validated the hypothesis *before* building it, and it
fails on the corpora that need it: the holdout replays the **adoption period
itself**, so the foreign callees are genuine *first* uses, not established ones.
At the parent of an ink call_receiver FP, `setImmediate` appears in 2 files,
`parseInt` in 3, `clearImmediate` in 1, `TextEncoder` and `structuredClone` in
**0** — the very hunks that fire are the ones introducing the builtin. Live
attestation quiets *subsequent* uses (which a periodically-re-fit deployment
already handles) but not the first — and the first use is precisely the
novel-pattern signal argot exists to raise.

**The load-bearing conclusion.** ink/bat/fastapi's residual is argot *correctly*
flagging the first use of a dependency/API/builtin the repo has never used
(bat's `git2`→`gix`, ink's `setImmediate`, fastapi's `annotated_doc`). It reads
as false-positive only because the temporal holdout replays real *human*
adoption commits; on an *agent's* pre-merge diff the same fire is the feature.
An agent-introduced foreign dependency and a human-adopted one are the identical
event — no scorer or attestation change separates them without also silencing
the catch. The honest operating point is: gated novel-pattern catch 48/49 @
aggregate FP ~1%, with corpora mid-adoption expected to spike and reported red.
The remaining lever is not the scorer but the deployment: run pre-merge on agent
diffs, gate CI at a stricter severity tier, and re-fit periodically.

## Artifacts

`benchmarks/results/issue92-final/` (holdout + production, git-ignored,
regenerable via `argot-bench --mode honest`). Rejected-experiment measurements
captured inline above.
