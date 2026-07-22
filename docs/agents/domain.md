# Domain Docs

## Before exploring, read these

- **`CLAUDE.md`** — the current single-binary architecture map and crate layout.
- **`CLAUDE.md`** — Architecture section: the crate/module layout.
- **`docs/research/`** — serves as ADR for this repo. Read entries that touch the area you're about to work in.
- **`docs/rust-port/`** — the Rust port's parity record + cutover plan.
- **Touching the semantic layer** (`crates/argot-rules-semantic/`) — read `docs/agents/semantic-contract.md` (self-calibration invariants) and the "Semantic layer" section of `CLAUDE.md` first.
- **Touching calibration** — `docs/agents/calibration-contract.md` is binding.

If any of these files don't exist, proceed silently. Don't flag their absence upfront.

## Layout

```
/
├── CLAUDE.md               ← architecture + conventions
├── docs/research/          ← system-wide decisions (ADR)
├── docs/rust-port/         ← port parity record
└── crates/
    ├── argot-lang/         ← language substrate and adapters
    ├── argot-engine/       ← rule-blind engine
    ├── argot-rules-*/      ← independent detector slices
    ├── argot-core/         ← facade + composition root
    └── argot-cli/          ← clap CLI → `argot` binary
```

## Use consistent vocabulary

When your output names a domain concept, use the term defined in `CLAUDE.md`. Don't drift to synonyms.

## Flag research conflicts

If your output contradicts a finding in `docs/research/`, surface it explicitly rather than silently overriding:

> _Contradicts research/04-import-graph-breakthrough.md — but worth revisiting because…_
