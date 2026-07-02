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
./target/release/argot-bench --corpus faker           # one corpus, production mode
./target/release/argot-bench --mode both              # full run + gap column (SLOW)
```

Two modes since era 15:

- **production** (default, the headline): every catalog fixture is planted
  into its host file on disk, staged with real git, and judged by the actual
  `argot fit` → `run_check --staged` pipeline. The FP control replays each
  corpus's last 30 commits through `check --commit` (`--fp-commits`).
  Seconds-to-a-minute per corpus.
- **catalog** (continuity): the historical in-process harness scoring.
  10–15 minutes per corpus — a full catalog run is expensive; scope to
  `--corpus` while iterating and save full runs for era-closing baselines.

`--mode both` adds the catalog↔production recall gap column — the tracked
path-fidelity metric (non-negative on every corpus as of era 15).

Canonical scoring config = the era-15 production defaults (n_cal=100, K=7
seeds, cluster_rare=2 + per-corpus auto-detect, parse-error host fallback
ON, convention-rarity stage ON). `--no-parse-error-fallback
--no-conventions` reproduces the era-14 catalog baseline; the remaining
era-14 substrate knobs (`--rarity-weighting`, `--calibration-source diff`,
`--enable-shape-primitives`) default off — see
`docs/research/evidence/era14-final.md` and `era15-production-path.md`.

Parity vs the old Python engine is locked in by the golden fixtures under
`crates/argot-core/tests/` and documented in `docs/rust-port/`. The Rust
harness reproduces the era-13.5 Python bench baseline exactly (108/115,
identical uncaught set).
