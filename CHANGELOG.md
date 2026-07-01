# Changelog

## Rewritten in Rust — one static binary, provably identical, much faster

argot's engine (previously Python) and CLI (previously TypeScript/Bun) are now a
**single statically-linked Rust binary**. No Python, no Node, no `uv` — one file,
instant startup, and the generic BPE baseline is embedded (no model download).

This is a **behaviour-preserving port, verified byte-for-byte**, not a
rewrite-and-hope. Against the previous engine, on all six benchmark corpora
(fastapi, rich, faker, faker-js, hono, ink), and confirmed on **both Linux and
macOS**:

- identical `dataset.jsonl` (exact bytes)
- identical per-hunk BPE scores (max diff `0`)
- identical calibrated thresholds
- identical AUC and recall; identical-or-better false-positive rate

…and it's materially faster:

| command | Rust | Python | speedup |
|---|---|---|---|
| `extract` | 2.95s | 15.2s | **5.2×** |
| `calibrate` | 2.2s | 7.8s | **3.5×** |
| `check` | 0.015s | 0.345s | **~23×** |

### Install

```sh
# curl
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh
# or npm
npm install -g @tmonier/argot
```

Both fetch the prebuilt `argot` binary for your platform (macOS arm64, Linux x64)
— no other dependencies.

### Under the hood

- Parity is locked in by golden test suites (`crates/argot-core/tests/*_parity.rs`)
  that compare Rust output to fixtures captured from the old engine, plus a
  numpy-exact calibration sampler (SeedSequence → PCG64 → `choice`) so thresholds
  match bit-for-bit.
- Distribution via [cargo-dist](https://opensource.axo.dev/cargo-dist/); the git
  walker uses vendored libgit2 (no system OpenSSL/SSH) for a portable static build.
- See `docs/rust-port/` for the full parity record.
