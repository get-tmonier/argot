# benchmarks — evaluation harness data

The benchmark harness lives in `crates/argot-bench` (Rust, workspace member,
never published). What lives here is its data:

- `catalogs/` — per-corpus break fixtures the scorer is evaluated against —
  the recall ground truth, expensive to recreate. Python/TS library corpora
  (fastapi, rich, faker, hono, ink, faker-js, dagster), application corpora
  (saleor, wagtail, excalidraw, outline — issue #66), and one hard-class
  catalog per remaining language (gh-cli, ripgrep, guava, powershell, redis,
  rocksdb, homebrew, laravel) authored under the frozen
  [`catalogs/RUBRIC.md`](catalogs/RUBRIC.md) (issue #92).
- `targets.yaml` — the corpus definitions (repo URLs + pinned SHAs) the
  harness clones.
- `data/` — cloned corpora + cached per-SHA extract datasets (gitignored).

Run it:

```
cargo build --release -p argot-bench
./target/release/argot-bench --corpus ink --quick     # smoke (recall only)
./target/release/argot-bench --corpus faker           # one corpus, honest mode
./target/release/argot-bench                          # full honest run (SLOW)
```

Modes (issue #92 — every published number is leak-free):

- **honest** (default, the headline): production-path **recall** (every
  catalog fixture planted into its host file on disk, staged with real git,
  judged by the actual `argot fit` → `run_check --staged` pipeline) plus
  **temporal-holdout FP** — fit at an old SHA (`holdout_window` first-parent
  commits behind the pin; per-corpus overrides in `targets.yaml`), replay
  only commits strictly after the fit point, split existing-file vs
  new-file, commit-level bootstrap 95% CIs. Emits the v2 `dashboard.json`.
- **production**: recall only. The old ancestor-replay FP control was
  train-on-test (the replayed commits were already inside the training
  corpus) and has been deleted.
- **holdout**: temporal-holdout FP only.
- **catalog** (continuity): the historical in-process harness scoring.
  10–15 minutes per corpus — scope to `--corpus` while iterating.

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
