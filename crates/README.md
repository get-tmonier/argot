# argot — Rust workspace

One Cargo workspace builds the statically-linked `argot` binary. The landing
site is separate; the CLI has no Python, Node, or runtime subprocess dependency.

## Crates

- **`argot-lang`** — language adapters, tree-sitter parsing, tokenization, and
  extension routing for all 12 shipped languages.
- **`argot-engine`** — rule-blind orchestration, configuration, artifacts,
  output, corpus walking, and the detector contract.
- **`argot-rules-{voice,semantic,arch,integrity,script}`** — independent rule
  slices; the scripted slice hosts repo-local Rhai rules.
- **`argot-core`** — facade and composition root (`src/compose.rs`) that
  registers the enabled slices.
- **`argot-cli`** — the clap `argot` binary. Run `argot --help` for the live
  command inventory rather than copying it into contributor docs.

## Build / test / run

```sh
just build           # cargo build --release -p argot → target/release/argot
just verify          # cargo fmt --check + clippy -D warnings + cargo test
just dogfood         # end-to-end development-loop signal on a repository
cargo test --workspace
```

## Composition and verification

`argot-core/src/compose.rs` is the composition root: it registers the voice
pass and feature-gated semantic, architecture, integrity, and scripted slices
in stable execution/merge order. Integration and golden suites live under
`crates/argot-core/tests/`; run `just verify` for the contributor gate.

## Toolchain

Stable Rust is pinned by `rust-toolchain.toml`. Dependency versions are pinned
where stable parser/tokenizer/git output requires it; see root `Cargo.toml`.
