# argot-bench

Reproducible benchmark harness for argot's production scorer. Scores a
catalog of hand-crafted "paradigm break" fixtures against the real-PR
hunks of six pinned open-source repos, and reports recall, false-positive
rate, AUC, threshold stability, and per-category breakdowns.

The harness is the source of truth for any claim about argot's
performance. The root README quotes the headline; every other number
lives in this tree.

## Quick run

```bash
just bench-quick   # ~1 min — 1 PR × 1 fixture per category × 1 seed (fastapi only by default)
just bench         # ~1.5h first time, ~20 min with caches — all 6 corpora
just bench-corpus fastapi   # one corpus, 5 seeds, full catalog
just verify-bench  # ruff + mypy + pytest on benchmarks/
```

Outputs land in `benchmarks/results/<timestamp>/`:
- `report.md` — human-readable aggregated markdown
- `<corpus>.json` — raw per-fixture + per-control scores (gitignored)

A committed snapshot of the latest full run lives at
[`benchmarks/results/baseline/latest/report.md`](results/baseline/latest/report.md)
and next to it under a dated `20260423T155121Z/` folder for historical diffs.

## Corpora

Six repos pinned to specific SHAs for reproducibility
(see [`targets.yaml`](targets.yaml)):

| Corpus | Language | PRs | Why |
|---|---|---|---|
| **fastapi** | python | 1 HEAD + historical walk | Async-first web framework; strong Pydantic/Depends voice |
| **rich** | python | 1 HEAD + historical walk | Terminal UI library; tight renderer/console vocabulary |
| **faker** | python | 1 HEAD + historical walk | Deterministic fake-data library; provider-heavy |
| **hono** | typescript | 5 pre-merge snapshots | Edge-runtime web framework |
| **ink** | typescript | 5 pre-merge snapshots | React components for CLI UIs |
| **faker-js** | typescript | 5 pre-merge snapshots | TS port of faker with locale data |

Python corpora have one HEAD SHA; the bench walks history from there.
TypeScript corpora use 5 pre-merge PR snapshots each to capture
"hunks in review" rather than merged history (git history on TS repos
often collapses PRs into a single squash commit).

## Fixture catalogs

Each corpus has 5–10 break categories with 1–5 fixtures per category,
rationales grounded in corpus evidence, and line-precise hunk bounds.
See `benchmarks/catalogs/<corpus>/manifest.yaml`.

| Corpus | Categories | Fixtures |
|---|---|---|
| fastapi | 9 | 31 |
| rich | 5 | 10 |
| faker | 5 | 5 |
| hono | 5 | 15 |
| ink | 5 | 15 |
| faker-js | 5 | 15 |
| **total** | **34** | **91** |

Example categories: `framework_swap` (Django CBV in a FastAPI app),
`async_blocking` (`time.sleep` inside an async def), `serialization`
(manual `json.dumps` when Pydantic is idiomatic), `foreign_rng`
(`Math.random` inside a deterministic faker-js provider).

## Methodology

Per corpus, per seed:

1. **Clone** the pinned SHA (cached across runs).
2. **Extract** training-quality hunks from the repo via `argot-extract`
   (the production pipeline's data step — not a bench-specific tokenizer).
3. **Calibrate** the production scorer on `n_cal=100` sampled hunks from
   the repo.
4. **Score** every catalog fixture as a break candidate.
5. **Score** every real PR hunk as a control.

Repeat across 5 independent seeds (Python corpora) or 5 seeds on the
primary PR plus 4 additional PRs (TypeScript) to measure threshold
stability.

### Metrics

| Metric | Meaning |
|---|---|
| **AUC** | Area under the ROC curve for break (catalog) vs control (real PR) scores. 1.0 = perfect separation; 0.5 = chance. |
| **Recall** | Fraction of catalog fixtures flagged as breaks. Reported overall and per category. |
| **FP rate** | Fraction of real PR hunks that crossed the flag threshold. |
| **Separation gap** | `min(break_score) − max(control_score)`. Positive = clean separation; negative = overlap. |
| **Threshold CV** | Coefficient of variation of the calibrated threshold across 5 seeds. Low CV = reproducible. |
| **Calibration stability** | Jaccard overlap of top-scored calibration hunks across seeds. |
| **Stage attribution** | Whether each break was caught by the import-graph stage (`import`), the BPE log-ratio stage (`bpe`), or missed (`none`). |

### What each metric tells you

- **High AUC, low recall** = the scorer orders breaks above controls but
  the calibrated threshold is too high. The ranking is useful; the cut
  isn't.
- **Low AUC** = the scorer can't tell some breaks from idiomatic code at
  all. Token novelty alone isn't enough for that category.
- **High FP rate with known culprits** = often data/locale/test files
  that are structurally unusual but not breaks. Calibration filter issue,
  not a scorer issue.
- **Low separation gap with high AUC** = breaks and controls overlap but
  the bulk of the break mass sits above the bulk of the control mass.
  Acceptable, but sensitive to calibration drift.

## Current baseline

From [`latest/report.md`](results/baseline/latest/report.md)
(run `20260423T155121Z`):

| Corpus | AUC | Recall | FP | N_fix | N_ctrl |
|:---|---:|---:|---:|---:|---:|
| fastapi | 0.9915 | 69.4% | 0.3% | 31 | 8,494 |
| rich | 0.9933 | 90.0% | 1.0% | 10 | 11,502 |
| faker | 0.9295 | 100.0% | 1.7% | 5 | 12,603 |
| hono | 0.7853 | 60.0% | 0.6% | 15 | 49,032 |
| ink | 0.9881 | 86.7% | 1.1% | 15 | 14,668 |
| faker-js | 0.8568 | 20.0% | 5.0% | 15 | 71,535 |

Threshold CV is 0% on every corpus — runs are fully deterministic
across seeds.

### Known weaknesses (flagged by this baseline)

1. **Data/locale/test files dominate the near-FP tail on every corpus.**
   The top-5 false positives across all six corpora are consistently
   things like `rich/_emoji_codes.py`, `src/locales/en/book/title.ts`,
   `faker/providers/currency/ru_RU/__init__.py`, and test files. These
   files are structurally unusual (long data tables, assertion-heavy
   test bodies) but aren't style breaks. This is a calibration-pool
   pollution issue, not a scorer issue — filtering `**/test*`,
   `**/locales/**`, and auto-detected data tables out of calibration
   should fix most of it.

2. **Semantic breaks are invisible to token-novelty.** Categories like
   faker-js `foreign_rng` (Math.random in a deterministic RNG library)
   and `http_sink` (fetch/axios inside a pure-data generator) score in
   the 0.5–3.8 range because the tokens themselves (`Math.random`,
   `fetch`, `axios`) are perfectly common in JS. The break is
   structural/contextual, not lexical, and needs a different detection
   stage (rule-based or AST-pattern).

3. **Keyword-compatible reframings slip through.** fastapi routing 0/3
   (Flask-style `@app.route` scores 4.31 vs threshold 5.28) — the token
   vocabulary is too similar to FastAPI routes to clear the threshold,
   even though the structural idiom is different.

## Reading a report

The generated `report.md` has, per corpus:

- **Summary** — AUC, recall, FP, threshold, separation gap, sample sizes.
- **Score distribution** — quantiles for breaks vs controls; shows where
  the threshold sits relative to both.
- **Per-category detail table** — recall, hits/total, mean/min/max break
  score, fixture IDs.
- **Per-fixture table** (expandable `<details>`) — every fixture's score,
  flagged status, reason, file, line range, and rationale.
- **Missed fixtures** — explicit callout with distance-to-threshold and
  rationale for each unflagged break.
- **Top 5 real-PR controls** — the hunks closest to flagging but not
  flagged; useful for investigating near-FPs.
- **Stage attribution** — import vs bpe vs none, with percentages.

## Updating the baseline

After a run you're satisfied with:

```bash
just bench
cp -r benchmarks/results/<timestamp> benchmarks/results/baseline/<timestamp>
cp benchmarks/results/<timestamp>/report.md benchmarks/results/baseline/latest/report.md
# Strip the large per-corpus JSONs — we only commit report.md for regression diffs
rm benchmarks/results/baseline/<timestamp>/*.json
git add benchmarks/results/baseline/
git commit -m "data(bench): baseline <date> for regression comparison"
```

Only `report.md` is committed. The raw JSONs are large (~41M total,
dominated by per-PR control scores) and reproducible from a rerun.

## Layout

```
benchmarks/
├── README.md                              # this file
├── pyproject.toml
├── targets.yaml                           # 6 pinned corpora
├── catalogs/                              # fixture catalogs per corpus
│   ├── fastapi/
│   │   ├── manifest.yaml
│   │   └── breaks/
│   │       ├── paradigm_break_flask_routing.py
│   │       └── ...
│   └── ...
├── src/argot_bench/
│   ├── cli.py                             # argparse entry point
│   ├── run.py                             # per-corpus orchestrator
│   ├── clone.py                           # git clone + checkout w/ cache
│   ├── extract.py                         # argot-extract subprocess wrapper
│   ├── score.py                           # wraps SequentialImportBpeScorer
│   ├── fixtures.py                        # Catalog/Fixture/PRHost + YAML loader
│   ├── metrics.py                         # AUC, recall, FP, threshold CV, …
│   ├── report.py                          # CorpusReport + markdown renderer
│   └── targets.py                         # targets.yaml loader
├── tests/                                 # 51 unit tests + 1 e2e smoke
└── results/
    ├── <timestamp>/                       # one dir per run, gitignored
    └── baseline/                          # checked-in snapshots
        ├── latest/report.md               # most-recent baseline
        └── <timestamp>/report.md          # historical baselines for diff
```

## See also

- Root [README](../README.md) — what argot is and how to use it
- [`targets.yaml`](targets.yaml) — the exact commit SHAs used for this run
- [`docs/superpowers/plans/2026-04-23-benchmark-harness.md`](../docs/superpowers/plans/2026-04-23-benchmark-harness.md) — the implementation plan this tree was built from
