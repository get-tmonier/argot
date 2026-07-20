# Pascal — 12th language port

Object Pascal (Delphi + FreePascal dialects) added as argot's 12th scored
language, benchmark-driven, following the issue-#92 language-port recipe.

## Grammar & adapter

- **Grammar:** `tree-sitter-pascal` 0.10.2 (Isopod), modern `LANGUAGE:
  LanguageFn` binding via `tree-sitter-language` 0.1 — ABI-accepted by the
  workspace's tree-sitter 0.26 (same shape as every other grammar). Covers
  classes/records/interfaces/helpers, generics (Delphi + FPC flavored),
  anonymous methods, inline asm, extended RTTI attributes.
- **Extensions:** `.pas` `.pp` `.dpr` `.lpr` `.inc` → `pascal`.
- **Adapter** (`argot-lang/src/adapters/pascal.rs`): the key design decisions,
  each a consequence of Pascal having a **flat `uses` clause with no
  relative-import syntax**:
  - `extract_imports` returns the **top-level segment** of each used unit
    (`mormot.core.json` → `mormot`, `SysUtils` → `SysUtils`) — matches the
    scorer's "top-level module only" rule and collapses dotted Delphi
    namespaces so a new submodule of an already-used dependency isn't
    false-flagged.
  - `internal_import_bindings` is always empty (no relative form); repo-internal
    units are recognised by `resolve_repo_modules`, which scans each source
    file's `unit`/`program`/`library` declaration and registers the top segment
    — the Java/Go pattern, not the Ruby `require_relative` one.
  - `callable_definitions` = `declProc` (proc/func/constructor/destructor, incl.
    a `defProc` header's `TClass.Method` `genericDot`) + `declType` names.
  - Comments (`//`, `{ … }`, `(* … *)`) all parse to one `comment` node; the
    auto-generated detector scans them directly (the shared Python-grammar
    detector can't see Pascal comment syntax — same as Java/Go/C).
  - Callee extraction: `exprCall` with an `identifier` or `exprDot` `entity`.
- **Rule layers wired:** voice (base composite + typicality/shape node-types),
  integrity (`test_inventory/pascal.rs` — FPCUnit/DUnit `procedure Test*` +
  `Assert*`/`Check*`/DUnitX `Assert.*`), architecture (`uses` → unit-name→layer
  index, persisted in the layering artifact), scripted rules (language-agnostic),
  CLI, and the bench harness.

## Corpora

Two large, tight-voiced primaries + the maintainer's uncle's two
FreePascal/Lazarus/MSEgui repos as extra real-world validation:

| Corpus | Role | Domain | Pascal files |
|---|---|---|---|
| castle-engine | primary | 3D/2D game engine (FPC+Delphi) | ~thousands |
| mormot2 | primary | client-server / ORM / SOA / crypto framework | 543 |
| uos | extra (non-gated) | audio library (fpGUI/MSEgui/LCL) | 99 |
| ideu | extra (non-gated) | MSEide-based IDE | 1149 |

Pins in `benchmarks/targets.yaml`.

## Over-fire (temporal-holdout FP, `--mode holdout`)

Fit at an old SHA, replay only later commits; over-fire = the `bpe`/`convention`
half of the reason split (fires on the repo's own tokens).

| Corpus | Pure over-fire (existing) | New-file | Notes |
|---|---:|---:|---|
| **castle-engine** | **0.61%** (4/657) | 0.00% (0/48) | Well under the ≤2% bar. (Total existing-file FP 1.07% incl. 0.46% novel-pattern detection, not counted against.) |
| **mormot2** | **0.75%** (5/666) | 0.00% (0/22) | Well under the ≤2% bar — large, self-contained, tight voice. |
| uos | 13.83% (18.07% incl. detection) | 40.9% | **Small heterogeneous wrapper lib** — thin Pascal shims over many C audio libs (portaudio, mpg123, opus, flac, soundtouch…), each bleeding its own naming in. This is the RUBRIC-documented small-corpus limit (a low calibrated threshold → over-fire creep), which is exactly why uos/ideu are extra, non-gated corpora. Honest finding, not a regression. |

## Novel-pattern recall (`--mode production` / honest)

Fixtures authored under `benchmarks/catalogs/RUBRIC.md` (≥3 `foreign_import`,
≥3 `foreign_api`, foreign-concurrency folded into import, + secondary
naming/semantic), each foreign symbol verified 0-usage at the pinned SHA
(`--mode production`, real disk-plant → git stage → `argot fit`/`check`).

| Corpus | Gated novel-pattern catch (≥85%) | Secondary (naming/semantic, not gated) |
|---|---:|---:|
| **castle-engine** | **11/11 (100%)** — 4 import · 4 api · 3 concurrency | 0/4 (honest misses) |
| **mormot2** | **11/11 (100%)** — 4 import · 5 api · 2 concurrency | 0/4 (honest misses) |

Both primaries clear both RUBRIC bars: **100% gated novel-pattern recall** (bar
≥85%) at **≤0.75% over-fire** (bar ≤2%). The 8 secondary misses (naming +
semantic across both corpora) are the proven fundamental local-detection limit —
authored correctly, scored 0.0, reported, never gated (identical to every other
language).

Foreign libs (all 0-usage-verified at the pinned SHA, vendored trees excluded):
Castle — Synapse (`httpsend`/`ftpsend`/`blcksock`), Zeos, superobject, FPC
`sqldb`, Graphics32 (GR32), OmniThreadLibrary, MTProcs, AsyncCalls. mORMot2 —
Indy, Synapse, Zeos, DCPcrypt, OmniThreadLibrary, Delphi PPL (`System.Threading`).
Libs Castle actually uses/bundles were rejected as decoys: Indy, SDL2, fpjson,
fphttpclient, PasMP, Kraft, Box2D, RegExpr, Vampyre.

**Vendored-tree exclusion (Castle):** `benchmarks/catalogs/castle-engine/argot.toml`
excludes `src/vampyre_imaginglib/`, `src/physics/kraft/`, `src/scene/load/pasgltf/`
— bundled third-party libraries with their own voice. Without it the Vampyre
GR32 bridge would falsely attest `Color32`/`Gray32` and the model would learn
alien naming (the standard per-corpus `argot.toml` the bench applies).

**Top-segment import property, recorded:** because `extract_imports` keys on the
top namespace segment, a Delphi `System.*` unit is only foreign if the repo never
used any `System.*` unit. `System.Threading` (the Delphi PPL) is therefore not
catchable as a foreign import on a repo that already uses `System.SysUtils` —
`System` is attested. This is the deliberate trade-off that stops a new submodule
of an already-used namespaced dependency (`mormot.core.json` next to
`mormot.core.base`) from false-firing, and mirrors Ruby's attested-root-namespace
may-misses. Both subagents independently hit and documented it (mORMot2 swapped
its Delphi-PPL fixture to non-namespaced MTProcs, which fires).

**Pascal call-receiver (foreign_api) property, recorded:** the call-receiver
path fires on a **bare foreign free-function** callee (`HttpGetText(...)`,
`FtpPutFile(...)`, `SO(...)`), not on `TForeignClass.Create` construction or a
method on a local receiver (`client.Get(...)`) — those are neighbourhood
behaviour by design (identical to every other language's call-receiver gate).
Foreign_api fixtures are therefore authored as bare foreign free-functions, and
some host clusters need ≥2–3 distinct 0-usage callees to clear the rare-threshold.

## Architecture + integrity capstones (Pascal joins both)

**Architecture** (`arch_violations.yaml`, resolver-verified via `--mode arch-candidates`): both
corpora, 10 authored violations each (mix sink_out / reversal / transitive_reversal across the
`src/<layer>/` topology — mORMot2 16 layers, Castle 19) + 4 forward controls each.

| Corpus | violation recall | control-FP | over-fire (holdout) |
|---|---:|---:|---:|
| mormot2 | 10/10 | 0/4 | 0.0% (0/150) |
| castle-engine | 10/10 | 0/4 | 0.0% (0/156) |

New arch capstone aggregate: **25 corpora / 12 languages · 264/272 = 97.1% · control-FP 0/148 ·
over-fire mean 0.37% (2.7% worst)**.

**Integrity** (`integrity_fixtures.yaml`): Castle uses FPCUnit (`TCastleTestCase`, `procedure
Test*`, `AssertEquals`/`Check*`) — **11/11 caught across 6 tactics** (assertion_deletion ×3,
tautologization ×1, comparison_widening ×3, skip_disable ×2, body_gutting ×1, test_deletion ×1),
0/4 controls, gating-FP 1/62 = 1.6% (1 test_deleted on the replay). mORMot2 is **N/A** — its
bespoke `TSynTestCase` framework (RTTI-discovered, non-`Test`-prefixed methods) is outside the
detectable FPCUnit/DUnit convention. New integrity capstone aggregate: **23 corpora / 12
languages · 155/164 = 94.5% · 0/106 controls · 45/3602 = 1.25% gating-FP**.

**Two integrity wiring gaps fixed** (`test_inventory/mod.rs`), both additive + Pascal-scoped:
- `tautology_capable()` was case-sensitive lowercase-only (`assertEquals`), never matching Pascal's
  PascalCase `AssertEquals`/`AssertTrue` — added the FPCUnit/DUnit names, so `tautologization` fires.
- `defined_symbols()` matched only other grammars' node kinds — added Pascal `defProc` (name via the
  header's `genericDot rhs`/identifier), `declProc`, `declType`, so `test-deleted` can confirm a
  deleted test's production subject still exists. Both unit-tested.

## Bench fix — CRLF corpora

mORMot2 (and most Delphi-heritage source) is **CRLF**. The bench's fixture
splice joined host + break lines with `"\n"`, silently rewriting a CRLF host
LF-only — every line read as changed and the planted diff drowned the fixture
(0% recall on a fixture that fires cleanly under `argot check`). Fixed
`fixture_scoring_input` to re-join with the host's own terminator
(`argot-bench/src/run.rs`). First argot corpus to exercise CRLF.

## End-to-end proof

`argot fit` + `argot check --staged` on uos flags a foreign HTTP client
(`uses IdHTTP` — Indy, 0-usage; the repo standardises on FreePascal's
`TFPHTTPClient`) with full evidence: _"IdHTTP, IdComponent — 0 of 145 module
specifiers in repo · common here: SysUtils 61×, Classes 52×."_
