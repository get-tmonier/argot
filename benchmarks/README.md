# benchmarks — evaluation harness data

The benchmark harness lives in `crates/argot-bench` (Rust, workspace member,
never published). What lives here is its data:

- `catalogs/` — per-corpus break/control fixtures (sample `.py` / `.ts` files
  the scorer is evaluated against). These are the AUC / recall /
  false-positive ground truth and are expensive to recreate. Library corpora
  (fastapi, rich, faker, hono, ink, faker-js, dagster) plus application
  corpora (saleor, wagtail, excalidraw, outline — issue #66).
- `targets.yaml` — the corpus definitions (repo URLs + pinned SHAs) the
  harness clones.
- `data/` — cloned corpora + cached per-SHA extract datasets (gitignored).

Run it:

```
cargo build --release -p argot-bench
./target/release/argot-bench --corpus ink --quick     # smoke
./target/release/argot-bench                          # full run, all targets
```

Canonical config = the era-13.5 production defaults (n_cal=100, K=7 seeds,
cluster_rare=2 + per-corpus auto-detect). Era-14 substrate knobs
(`--rarity-weighting`, `--calibration-source diff`,
`--enable-shape-primitives`, `--enable-parse-error-fallback`) default off;
see `docs/research/evidence/era14-final.md` for why.

Parity vs the old Python engine is locked in by the golden fixtures under
`crates/argot-core/tests/` and documented in `docs/rust-port/`. The Rust
harness reproduces the era-13.5 Python bench baseline exactly (108/115,
identical uncaught set).
