# Contributing to argot

Thanks for helping make argot better. It's a single statically-linked Rust
binary — no Python, no Node, no runtime deps — so the on-ramp is short.

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Found a
security issue? **Don't open a public issue** — follow the private disclosure
process in [SECURITY.md](SECURITY.md).

## Dev setup

You need the Rust toolchain and [`just`](https://github.com/casey/just):

```sh
git clone https://github.com/get-tmonier/argot && cd argot
# rustup reads the pinned toolchain from rust-toolchain.toml automatically.
just build          # cargo build --release -p argot → target/release/argot
```

Optionally, [`mise`](https://mise.jdx.dev/) installs the peripheral tooling
(`just`, `lefthook`, and `bun` for the landing site):

```sh
mise install
lefthook install    # wire the rustfmt + clippy pre-commit hooks
```

`just` is the canonical interface for every dev command — run `just` (or
`just help`) to see them all.

## The gate: `just verify`

Every change must pass `just verify` before it's committed and before it merges.
It runs the same three checks CI does:

```sh
just verify   # cargo fmt --check + cargo clippy -D warnings + cargo test
```

- **Formatting** is enforced (`cargo fmt --check`). Run `just verify-fix` to
  auto-apply.
- **Clippy runs as `-D warnings`.** Don't reach for a blanket `#![allow(...)]`;
  diagnose the root cause and, if a lint genuinely doesn't apply, use a
  targeted `#[allow(clippy::specific_lint)]` on the one item with a one-line
  reason. See [CLAUDE.md](CLAUDE.md#code-quality).
- **Tests** include in-module unit tests and the parity/golden suites (below).

Write tests alongside new logic — behaviour-focused (assert on outputs for given
inputs, not internal state), enough for a fast feedback loop, not 100% coverage.

## Verification and scored-output changes

The composed integration/golden suites live under `crates/argot-core/tests/`.
Do not bump pinned parser, tokenizer, or `git2` dependencies without re-running
the relevant suites: those pins protect stable output. A change that alters
scored output is a research change, not routine refactoring; benchmark it with
the appropriate `just bench…` command and record evidence under
`docs/research/evidence/`.

## Proposing a new language or corpus

Language support ships **after** it's benchmarked, never on the promise of a
tree-sitter parser existing. To add one (this is the shape of issues
[#42–#49](https://github.com/get-tmonier/argot/labels/help%20wanted)):

1. Add a `LanguageAdapter` implementation under `crates/argot-lang/` (parsing,
   tokenization, import/callee extraction, prose masking, and extension routing)
   and register its extensions through that substrate.
2. Pin 2–3 real corpora at specific commit shas.
3. Hand-craft ~15–20 anomaly fixtures per corpus, mirroring the existing
   catalog format under `benchmarks/`.
4. Run `extract → fit → check`, and record recall + false-positive numbers in
   `docs/research/evidence/`.
5. The bar is **recall ≥ 85% and FP ≤ 2%** — the same one Python and TypeScript
   clear. Update the README "Supported languages" table.

Have a corpus you'd like validated but can't do the work yourself? Open a
language/corpus request issue — it's the most useful thing you can file.

## Conventions

- **Language- and corpus-agnostic engine.** `argot-engine` and `argot-rules-*`
  must not hardcode a specific language, framework, or corpus. Language-specific
  parsing belongs in `argot-lang`; corpus literals belong only in fixtures,
  benchmarks, and evaluation scripts.
- **Domain names, not research artefacts.** Production symbols and doc-comments
  are named after what the code does — never after research labels (`era`,
  `phase`). Those breadcrumbs belong in `docs/research/`.
- Agent/maintainer conventions (issue tracker, triage labels, domain docs) live
  under [`docs/agents/`](docs/agents/).

## Pull requests

- One focused change per PR; keep the diff reviewable.
- Fill in the PR template: what changed, why, and tick the `just verify` box.
- The landing site (`landing/`) is the only non-Rust piece; if you touch it, run
  `just landing-check`.

By contributing you agree your work is licensed under the repository's MIT
license.
