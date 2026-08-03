# argot fuzzing

[cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) harnesses for
argot's untrusted-input parsers. argot is pointed at repositories it does not
control, so its byte-level parsers must not panic, hang, or blow up memory on
adversarial input.

## Targets

| Target | Surface |
| --- | --- |
| `tokenize` | `argot_lang::tokenize::tokenize` — the full tree-sitter parse + leaf-token + BPE tokenization path that `extract` runs over every source file. |
| `ts_parse` | `argot_lang::ts_parse::parse` — raw tree-sitter parse-tree construction, isolating the grammar/ABI layer. |

The other untrusted surface — git diffs and blobs — is parsed by **libgit2** (a C
library, via `git2`). It is best fuzzed upstream rather than here; argot's own
code around it propagates errors rather than unwrapping (`git_walk.rs`).

## Running

Requires a nightly toolchain and `cargo-fuzz`. The `+nightly` is not optional:
the repo's `rust-toolchain.toml` pins `stable`, and a toolchain file wins over
`rustup default`, so a bare `cargo fuzz` builds with stable and dies on
`-Zsanitizer=address`. (`RUSTUP_TOOLCHAIN=nightly` works too — that is what CI
uses.)

```sh
rustup toolchain install nightly
cargo install cargo-fuzz

# Run from the repo root (cargo-fuzz uses the default `fuzz/` directory).
# Build both targets (a cheap "does it still compile" check):
cargo +nightly fuzz build

# Fuzz a target (Ctrl-C to stop; or bound it):
cargo +nightly fuzz run tokenize
cargo +nightly fuzz run ts_parse -- -max_total_time=60
```

A crash writes a reproducer under `fuzz/artifacts/<target>/`; replay it with
`cargo +nightly fuzz run <target> <artifact-path>`.

CI runs a short weekly smoke (`.github/workflows/fuzz.yml`) — a bounded run per
target, which also rebuilds them, so the harnesses can't silently rot. It is
deliberately off the PR path: nightly + libFuzzer + an instrumented rebuild of
every tree-sitter grammar is too heavy for every PR.
