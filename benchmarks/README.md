# benchmarks — evaluation harness data

The benchmark harness lives in `crates/argot-bench` (Rust, workspace member,
never published). What lives here is its data:

- `catalogs/` — per-corpus break fixtures the scorer is evaluated against —
  the recall ground truth, expensive to recreate. **36 corpora across the 12
  supported languages** (listed under [Acknowledgements](#acknowledgements)),
  authored under the frozen [`catalogs/RUBRIC.md`](catalogs/RUBRIC.md)
  (issue #92). A catalog directory may also hold an `argot.toml` — the
  configuration a maintainer of that repository would write — and the
  fixture-set manifests for the architecture and integrity layers.
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

Canonical scoring config = the current production defaults (n_cal=100, K=7
seeds, cluster_rare=2 + per-corpus auto-detect, parse-error host fallback
ON). The convention-rarity stage is **off in production** — *secondary
coverage*, never gated (`catalogs/RUBRIC.md`), and a co-headline false-alarm
driver — so `fit`/`check` never enable it and expose no user-facing flag; it
survives only as the internal `CalibrateOptions.enable_conventions` field
(default off) that the benchmark harness can flip to measure the
with/without trade-off. In the `catalog` continuity mode,
`--no-parse-error-fallback --no-conventions` reproduces the era-14 baseline;
the remaining era-14 substrate knobs (`--rarity-weighting`,
`--calibration-source diff`, `--enable-shape-primitives`) default off — see
`docs/research/evidence/era14-final.md` and `era15-production-path.md`.

Parity vs the old Python engine is locked in by the golden fixtures under
`crates/argot-core/tests/` and documented in `docs/rust-port/`. The Rust
harness reproduces the era-13.5 Python bench baseline exactly (108/115,
identical uncaught set).

## Acknowledgements

Every number argot publishes is measured against the real history of these
open-source projects. The benchmark would not exist without them, and we are
grateful to their maintainers and contributors.

- **Python** — [fastapi](https://github.com/tiangolo/fastapi), [rich](https://github.com/Textualize/rich), [faker](https://github.com/joke2k/faker), [saleor](https://github.com/saleor/saleor), [wagtail](https://github.com/wagtail/wagtail), [scrapy](https://github.com/scrapy/scrapy)
- **TypeScript** — [hono](https://github.com/honojs/hono), [ink](https://github.com/vadimdemedes/ink), [faker-js](https://github.com/faker-js/faker), [excalidraw](https://github.com/excalidraw/excalidraw), [outline](https://github.com/outline/outline)
- **JavaScript** — [express](https://github.com/expressjs/express), [commander](https://github.com/tj/commander.js), [eslint](https://github.com/eslint/eslint)
- **Go** — [gh-cli](https://github.com/cli/cli), [hugo](https://github.com/gohugoio/hugo)
- **Rust** — [ripgrep](https://github.com/BurntSushi/ripgrep), [bat](https://github.com/sharkdp/bat)
- **Java** — [guava](https://github.com/google/guava), [junit5](https://github.com/junit-team/junit5)
- **C#** — [powershell](https://github.com/PowerShell/PowerShell), [jellyfin](https://github.com/jellyfin/jellyfin)
- **C** — [redis](https://github.com/redis/redis), [curl](https://github.com/curl/curl)
- **C++** — [rocksdb](https://github.com/facebook/rocksdb), [fmt](https://github.com/fmtlib/fmt)
- **Ruby** — [homebrew](https://github.com/Homebrew/brew), [rubocop](https://github.com/rubocop/rubocop)
- **PHP** — [laravel](https://github.com/laravel/framework), [composer](https://github.com/composer/composer)
- **Object Pascal** — [castle-engine](https://github.com/castle-engine/castle-engine), [mormot2](https://github.com/synopse/mORMot2), [uos](https://github.com/fredvs/uos), [ideu](https://github.com/fredvs/ideU), [mseide-msegui](https://github.com/mse-org/mseide-msegui)
- **Multi-language** — [dagster](https://github.com/dagster-io/dagster)

**Argot vendors and redistributes none of this code.** `benchmarks/data/` is
gitignored: the harness clones each repository at a SHA pinned in
`targets.yaml`, reads its history locally, and ships nothing from it. Each
project remains under its own license, held by its own authors — follow the
links above for terms.

What *is* committed here is argot's own work: the break fixtures under
`catalogs/`, authored against the frozen [`RUBRIC.md`](catalogs/RUBRIC.md), and
the per-corpus `argot.toml` each catalog carries — the configuration a
maintainer of that repository would plausibly write (vendored trees, generated
code, and demo directories excluded from the voice model).

Corpora are chosen for history depth and idiom variety across the twelve
supported languages, never for how well argot scores on them. A corpus is
never swapped or dropped because a fixture fails to fire — that is a finding to
report, per the rubric.

