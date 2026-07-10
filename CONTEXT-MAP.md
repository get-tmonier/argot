# Context Map

argot is a single Rust binary — no CLI/engine split. The architecture lives in
one Cargo workspace:

- **`crates/argot-core`** — the engine (pure library): git walk, tokenize/BPE,
  scorers, calibration, check, evidence, the rule registry (`rules.rs` — 7
  rules / 3 groups), and the feature-gated semantic (`--features semantic`) and
  architecture (`--features arch`) layers. Language- and corpus-agnostic.
  Produces and reads the fit artifacts in `.argot/`.
- **`crates/argot-cli`** — the clap CLI that wires the pipeline into the single
  `argot` binary (`extract` → `train` → `calibrate` → `check`, or `fit`), plus
  the user-facing commands (`init`, `review`, `replay`, `voice-diff`,
  `inspect`, `status`, `describe-voice`, mutes) and the MCP server (`argot mcp`).

See the **Architecture** section of `CLAUDE.md` for the module layout, and
`docs/rust-port/` for the port's parity record.

## Shared concept

**voice profile** — the model of a repo's voice, learned from its git history and
persisted under `.argot/` (`dataset.jsonl`, `scorer-config.json`, plus
`semantic-index.json` for the embedding rules and `repo-corpus.txt`, the list of
files that shaped the fit). The pipeline commands produce it; `check` reads it
to score new hunks.
