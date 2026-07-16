# argot

A guardrail that flags code foreign to a repo's own patterns — the dependencies, APIs, and constructs an AI coding agent reaches for that the codebase has never used — learned from the repo's git history (north star + metric: `benchmarks/catalogs/RUBRIC.md`; novel-pattern catch rate @ low false-alarm). A single statically-linked Rust binary (`crates/argot-{core,cli}`) — no Python, no Node, no runtime dependencies. (Previously a TS/Bun CLI + Python engine; ported to Rust with verified byte-for-byte parity — see `docs/rust-port/`.)

## Guiding principle

**In doubt, optimise for code that's easy to change.** The Pragmatic Programmer / craftsmanship lens: the right design is the one a future contributor (human or agent) can extend, refactor, or revert without archaeology. When two options look equally correct, pick the one with the smaller blast radius and clearer seams. Don't add abstractions before the second use case shows up; don't keep dead code "just in case"; don't suppress a check when the underlying code is the real fix. Strict tooling (clippy `-D warnings`, the parity golden suites) exists to surface change-cost early — work with it, not around it.

## Task runner

Always use `just` — it's the canonical interface for all dev commands.

```
just verify       # cargo fmt --check + clippy -D warnings + cargo test
just test         # cargo test --workspace
just extract .    # run extract on this repo → .argot/dataset.jsonl
just dogfood      # run full pipeline against argot itself (or any path) — fast monorepo check
just build        # cargo build --release -p argot → target/release/argot
just bench-quick  # ~1 min bench smoke (one fixture per category + 50 controls)
just arch-verify  # ~25 s architecture-layer fixture-recall regression guard
just integrity-verify  # gaming-fixture recall + control guard for the integrity rules
```

`just dogfood` exercises extract → train → calibrate → check end-to-end and asserts both Python and TypeScript rows landed in `dataset.jsonl` plus a `scorer-config.json` was emitted. It's a **dev loop, not a CI gate** — informational signal that monorepo handling didn't silently break. Drift is the contributor's responsibility; nothing forces it to run.

## Architecture

One Cargo workspace, hexagonal: a rule-blind engine, one crate per rule
group (a **vertical slice** — deletable without touching the core), and a
facade holding the composition root.

```
crates/
  argot-lang/            # LEAF language substrate: the 11 LanguageAdapter impls,
                         #   tree-sitter parsing/grammars (ts_parse), tokenize + BPE
                         #   (embedded unixcoder tokenizer), callee extraction,
                         #   text utils, dataset wire format, ext→language routing.
  argot-engine/          # RULE-BLIND engine (zero cargo features, zero slice
                         #   knowledge): the Detector contract (detector.rs —
                         #   lifecycle: vocabulary → load → fit_begin/fit_language/
                         #   fit → check), Finding + RenderEvidence (finding.rs),
                         #   the rule registry (rules.rs: built-in vocabulary +
                         #   runtime custom overlay), check orchestration
                         #   (check/{orchestrate,collect,render}.rs), config.rs,
                         #   suppress/ (incl. the shared FileSuppressions
                         #   classifier), output.rs, git/corpus walking, artifact
                         #   writes, health/timing/cache/stats.
  argot-rules-voice/     # The base statistical group (always ships): sequential
                         #   composite (BPE + import + call-receiver + conventions
                         #   + typicality — arbitration is ONE slice, do not split),
                         #   calibration, train/extract, model loading/RepoScorers,
                         #   inspect, ignore-suggest. Feature `structural` (research
                         #   AST-bigram signal, NON-GATING, off in releases).
  argot-rules-semantic/  # `redundant` + `misplaced` (embeddings; llama-cpp-2 deps
                         #   live HERE). See "Semantic layer".
  argot-rules-arch/      # `layering` (module-dependency graph). See below.
  argot-rules-integrity/ # test-deleted/-disabled/-weakened. See below.
  argot-rules-script/    # RUNTIME community rules: `.argot/rules/<name>/`
                         #   (rule.toml manifest + check.rhai), sandboxed Rhai host
                         #   API v1 (ts_query, learned-model facts, report), the
                         #   `argot rules test` harness. See "Scripted rules".
  argot-core/            # FACADE + COMPOSITION ROOT — exactly two files:
                         #   lib.rs (re-exports every historical path; the 18
                         #   parity/integration suites live in its tests/) and
                         #   compose.rs (which rule groups this build registers;
                         #   deleting a group = deleting a crate + its lines here).
                         #   Cargo features semantic/arch/integrity/script are
                         #   optional slice-crate deps; structural forwards to voice.
  argot-cli/             # clap CLI → the single `argot` binary (package name: argot);
                         #   per-command modules: mcp.rs (MCP server) · review.rs ·
                         #   audit/ · voice_diff.rs · describe.rs (describe-voice) ·
                         #   auto_refit.rs · update_check.rs · uninstall.rs · worktree.rs
  argot-bench/           # research harness (never shipped; publish = false)
```

Dependency direction is strict: `lang ← engine ← rules-* ← core ← cli/bench`.
A rule crate never imports another rule crate; the engine never names a slice
(grep-enforced: no `cfg(feature` and no slice references in argot-engine).
Every group implements the engine's `Detector` trait and is registered in
`argot-core/src/compose.rs` with an explicit **order table**:
execution_rank (additive passes first, voice last — stderr interleave) and
merge_rank (voice's findings first — stdout order); both are parity-locked
by the check goldens.

The full pipeline is `train` → `calibrate` → `check` (`fit` = train + calibrate, one-shot; `argot init` = fit + health report; `extract` is bench plumbing — the fit → check flow never consumes the dataset). Everything runs in-process in the one binary — no subprocess, no external files. Release binaries build with `features = ["self-update", "semantic", "arch", "integrity", "script"]` (`dist-workspace.toml`); dev/CI base loops build with none of them.

### Semantic layer (`--features semantic`)

A second, embedding-based sense layered on the base statistical guardrail. It
builds a per-repo `SemanticIndex` (embed every function at fit, query at check)
and emits two rules — `redundant` (F1 reinvention — "you already have this") and
`misplaced` (F2 placement — "this doesn't belong here"), group `semantic` in the
rule registry (`argot-engine/src/rules.rs`) — plus nearest-code evidence (F4) on both.
Embedder = llama.cpp statically linked via `llama-cpp-2` (same in-process C-dep
shape as git2/tree-sitter), model = jina-embeddings-v2-base-code Q4 GGUF
fetched-on-first-use to `~/.cache/argot/models` (sha256-pinned; the artifact
records the model identity and a stale index is rejected loudly). Contributor
contract: `docs/agents/semantic-contract.md`.

**Binding invariant:** the whole layer is behind `feature = "semantic"` (a
build-time gate, default off). With it off the base guardrail is byte-for-byte
unchanged, builds pure-Rust with zero new deps, and pays no cost. The shipped
binary is built with it **on** (release enables the feature); users control it
like any rule (`[rules] semantic = "off"` skips the index and the download —
there is no dedicated toggle beyond the rules surface). The model auto-downloads
on first use with progress + a loud, verbalized skip offline. The index lives in
its own `.argot/semantic-index.json` so `scorer-config.json` is untouched. Its
findings are never folded into the base catch/false-alarm metric. Dev/CI test
with `ARGOT_SEMANTIC_MODEL=<gguf path>` to skip the download; `ARGOT_OFFLINE=1`
forbids all network.

### Architecture layer (`--features arch`)

The `layering` rule (group `architecture`): flags an internal module-dependency
edge that reverses the repo's learned layer direction, closes a cycle, or lands
on a (near-)sink. Same build-time-gate shape as `semantic` — off in dev/CI base
loops, ON in releases. Validated at 244/252 (96.8%) real recall / 0 control FPs
across 23 corpora (evidence in `docs/research/evidence/`); `just arch-verify`
is the ~25 s fixture-recall regression guard.

### Integrity layer (`--features integrity`)

The `integrity` rule group: `test-deleted` (error) — a test removed while the
production code it exercised still exists; `test-disabled` (error) — a
skip/ignore marker added or a test gutted to a vacuous pass; `test-weakened`
(**warn** by default — reported, never fails `check`) — assertions
excised/tautologized/widened or an expected literal retargeted. All three fire
only when the changeset also touches production source (tests-only commits
are suite curation, not gaming) and are pinned confidence `suspicious`. Same
build-time-gate shape as `semantic`/`arch` — off in dev/CI base loops, ON in
releases. Per-repo gates are learned at fit from a mini-replay of the repo's
accepted history and stored in `.argot/integrity.json` (a rebuildable sibling
of `scorer-config.json`). Validated at 94.1% catch (144/153 authored gaming
fixtures, 22 corpora / 11 languages), 0/102 legitimate-refactor controls
fired, and 1.24% of replayed accepted test-touching commits flagged at gating
severity; evidence in `docs/research/evidence/test-integrity-capstone.md`.
`just integrity-verify` is the fixture-recall + control regression guard.

### Scripted rules (`--features script`)

Runtime community rules, no recompilation: a rule is
`.argot/rules/<name>/rule.toml` (schema/api versions, default severity,
language scope) + `check.rhai` (detection logic; runs once per changed file).
Host API v1 does the heavy lifting natively — `ts_query()` (tree-sitter),
`import_attested()`/`callee_attested()` (the fitted voice model's facts via
the engine's `ModelFacts` port), `file`/`hunks` scope, `report`/`report_span`.
Sandbox: no I/O, print captured, 1M-op + depth/size caps, 100 ms wall clock
per file — a runaway rule is disabled for the run, never hangs the check.
Custom findings carry reason `custom:<name>` (syntactic mapping — suppression
hot paths need no registry) and behave exactly like built-ins across
`[rules]`/`--rule`, `rule=` inline scopes, `[[mute]]`, and every output
format. `argot rules test [name]` is the fixture-based authoring loop
(`tests/<case>/{input.<ext>, expected.json}`). Same gate shape: pure-Rust
(rhai), ON in releases.

### Structural signal (`--features structural`)

`argot-rules-voice/src/scoring/structural.rs`: 0-usage AST-bigram
foreignness. Real signal but not gatable (no threshold gives acceptable
over-fire everywhere) — kept feature-gated, NON-GATING, **off in releases**.
Don't re-chase gatability; the evidence record explains why.

Production symbols (types, files, functions) must be named after domain concepts — never after research artefacts (`era`, `phase`, `PhaseNa…`, etc.); those labels belong in eval/research code only.

## Key conventions

- Language/corpus-agnostic core (see below); errors via `anyhow`/`thiserror`.
- Dependency versions are pinned for parity with the original Python engine (tree-sitter grammars, `tokenizers`, libgit2 via `git2`) — see the comments in the root `Cargo.toml`. Don't bump them without re-checking the golden/parity suites. (`tokenizers` was moved 0.22 → 0.23 with parity re-verified; `git2` stays on 0.19 until libgit2 fixes the 1.9.x blame segfault — the `blame_survives_empty_email_author` canary is the gate.)
- Rust edition 2021, toolchain pinned in `rust-toolchain.toml`. Clippy runs as `-D warnings`; no `#![allow(...)]` blanket suppressions.
- Test files: **production files carry no test code** — each module's unit tests live in a sibling `tests.rs` (`#[cfg(test)] mod tests;` → `<module>/tests.rs`), still compiled out of every release build; parity/golden suites in `crates/argot-core/tests/*_parity.rs` (compare Rust output to fixtures captured from the old Python engine) exercise the fully composed pipeline through the facade.

## Testing

Write tests alongside any new logic — not 100% coverage, but enough for a fast feedback loop. Aim to cover:
- Core logic correctness (shapes, invariants, non-trivial conditions)
- Smoke tests for new entry points

For non-trivial production logic (scoring math, threshold decisions, cluster logic), write unit tests that test behaviour, not implementation: assert on outputs for given inputs, not on internal state or call sequences. Tests should survive a refactor that preserves semantics.

## Language and corpus independence

Production scoring code (the `argot-rules-*` crates and `argot-engine`) must be language-agnostic and corpus-agnostic. No hardcoded references to Python, TypeScript, FastAPI, faker-js, or any other specific language or corpus. Those appear only in fixtures, benchmarks, and eval scripts (language-*specific* code belongs in `argot-lang`'s per-language adapters, behind the uniform `LanguageAdapter` surface). A scorer that only works on Python repos is not a production scorer.

## Code quality

The codebase is strict by design (clippy runs as `-D warnings`). When a check fails:
- Diagnose the exact root cause before fixing
- Prefer targeted fixes (`#[allow(clippy::specific_lint)]` on one item, with a one-line reason) over global config changes
- Never add broad suppressions (crate-level `#![allow(...)]`, blanket `#[allow(warnings)]`) to make errors go away

We aim for clean architecture and clean code; lint-suppression debt compounds and is the wrong knob to turn when a check fails. The right knob is the underlying code.

## Toolchain

Rust toolchain pinned in `rust-toolchain.toml` (via `rustup`). `mise` manages the peripheral tools: `just 1.49.0` · `lefthook 2.1.6` · `bun 1.3.12` (landing site only).

Build/lint/test: `cargo` · `rustfmt` · `clippy` (`-D warnings`). Releases: `cargo-dist` (`dist-workspace.toml`).

## Research workflow

Benchmarks are expensive. Default to the cheapest signal first:

1. **Dirty experiment script** in `benchmarks/` — quick, ugly code is fine; what matters is the number, not the code.
2. **Scoped bench run** on one or two corpora — enough to confirm or kill a hypothesis.
3. **Full corpus bench** — final confirmation of a strong signal, or era-closing baseline. Not a default step.

Keep evidence of every experiment in `docs/research/evidence/` regardless of outcome. Clean up experiment scripts once results are recorded — they don't need to survive, the evidence does.

## Agent-facing product surfaces (keep in sync)

When a command, flag, rule, or exit code changes, these ship the change to
users and must move together:

- `skills/` — the four shipped skills (`argot-setup`, `argot-check`,
  `argot-review-pr`, `argot-setup-ci`); workflow procedures, not command
  catalogs — `argot --help` stays the source of truth.
- `crates/argot-cli/src/mcp.rs` — the MCP server (`argot mcp`).
- `.claude-plugin/` — the Claude Code plugin (bundles the skills + MCP server).
- `action.yml` — the GitHub Action; its inputs table is documented in
  `landing/src/content/docs/ci.md`.
- `AGENTS.md` — the usage contract for coding agents, published at
  argot.tmonier.com/docs/agents/ and in `llms.txt`.
- `landing/` — the site + docs (`landing/src/content/docs/`, i18n copy in
  `landing/src/i18n/{en,fr}.ts`; benchmark numbers are CI-fed from
  `landing/src/data/*.json` — never hand-edit a metric).

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Four-role vocabulary for solo maintainer (no `needs-info`). See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context layout; `docs/research/` serves as ADR. See `docs/agents/domain.md`.
