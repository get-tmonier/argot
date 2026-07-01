# benchmarks — evaluation fixtures

The Python benchmark harness was removed in the Rust cutover. What remains here is
**data, not code** — the hand-crafted evaluation ground truth, preserved for a
future Rust benchmark:

- `catalogs/` — per-corpus break/control fixtures (sample `.py` / `.ts` files the
  scorer is evaluated against). These are the AUC / recall / false-positive
  ground truth and are expensive to recreate.
- `targets.yaml` — the corpus definitions (repo URLs + PR SHAs) the harness clones.
- `data/` — cloned corpora (gitignored).

Parity vs the old Python engine is now locked in by the golden fixtures under
`crates/argot-core/tests/` and documented in `docs/rust-port/` +
`docs/research/evidence/`. A Rust re-implementation of the scoring benchmark
(AUC / recall / fp over these catalogs) is tracked as follow-up work.
