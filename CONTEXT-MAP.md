# Context Map

argot is a single Rust binary — no CLI/engine split. The architecture lives in
one Cargo workspace:

- **`crates/argot-core`** — the engine (pure library): git walk, tokenize/BPE,
  scorers, calibration, check, evidence. Language- and corpus-agnostic. Produces
  and reads the fit artifacts in `.argot/`.
- **`crates/argot-cli`** — the clap CLI that wires the pipeline into the single
  `argot` binary (`extract` → `train` → `calibrate` → `check`, or `fit`).

See the **Architecture** section of `CLAUDE.md` for the module layout, and
`docs/rust-port/` for the port's parity record.

## Shared concept

**voice profile** — the model of a repo's voice, learned from its git history and
persisted under `.argot/` (`dataset.jsonl`, `scorer-config.json`). The pipeline
commands produce it; `check` reads it to score new hunks.
