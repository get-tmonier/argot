# argot — Rust workspace

The Rust rewrite of argot: one statically-linked `argot` binary replacing the
TypeScript/Bun CLI (`cli/src`) and the Python/UV engine (`engine/argot`). No
Python subprocess, no Bun runtime.

## Crates

- **`argot-core`** — the engine (language- and corpus-agnostic per the root
  `CLAUDE.md`): git walk, tokenisation, BPE, the statistical scorers,
  calibration, and `check`. No hardcoded framework/language literals.
- **`argot-cli`** — the `argot` binary (clap). Subcommands: `extract`, `train`,
  `calibrate`, `fit` (= train + calibrate), `check`, `status`, `list`,
  `update`; no subcommand prints the help banner.

## Build / test / run

```sh
just build-rust      # cargo build --release -p argot-cli  → target/release/argot
just verify-rust     # cargo fmt --check + clippy -D warnings + cargo test --workspace
just dogfood-rust    # full pipeline on a repo; asserts both .py/.ts rows + config
cargo test --workspace
```

## Parity

This is a behaviour-preserving port, gated against the Python engine. Every
module has golden parity tests (fixtures under `argot-core/tests/fixtures/`):
extract is byte-identical to `argot-extract`, BPE `encode` is bit-identical to
the `microsoft/unixcoder-base` tokenizer, and `check` output is byte-identical.
The full parity story, decisions, and any documented divergences (KMeans,
calibration RNG) live in `docs/rust-port/PORTING-NOTES.md`; the benchmark AUC
parity proof is in `docs/research/evidence/rust-port-auc-parity.md`.

## Toolchain

Stable Rust (see `rust-toolchain.toml`); ≥1.85 required by the git2 → url →
ICU4X dependency chain (edition2024). Dependency versions are pinned for parity
(tree-sitter grammars, `tokenizers` 0.22, `git2`/libgit2).
