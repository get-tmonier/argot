# Research Branch — Phase 1–14 History

This branch (`research/v1`, formerly `research/cleanup`) contains the Phase 1–14 research code and scripts used to develop and validate argot's production scorer. It is **not actively maintained** — it is a reference for how Phase 14's `SequentialImportBpeScorer` was developed and validated.

## What's here

- **`engine/argot/research/signal/phase14/`** — Phase 14 scorer, adapters, filters, parsers, calibration, and experiment scripts.
- **`engine/argot/research/signal/phase13/`** — Earlier phase experiments (AST contrastive, JEPA baseline).
- **`engine/argot/acceptance/catalog/`** — Acceptance-test fixture manifests (corpus JSONL files stripped; regenerate via reproducibility script).
- **`docs/research/`** — Phase-by-phase scoring roadmap notes.

## Layout

```
engine/argot/research/signal/phase14/
  scorers/          # SequentialImportBpeScorer + tests
  adapters/         # Python and TypeScript LanguageAdapter implementations
  filters/          # data_dominant heuristic
  parsers/          # prose line ranges, TreeSitter parity
  calibration/      # random hunk sampler
  experiments/      # ~30 experiment scripts from the Phase 14 sweep
```

## Running a standalone experiment (no corpus needed)

The `test_data_dominant.py` unit test runs without any pre-generated corpora:

```bash
cd engine
uv run pytest argot/research/signal/phase14/filters/test_data_dominant.py -v
# 12 passed in ~0.04s
```

## Running corpus-dependent experiments

Most experiment scripts under `experiments/` load pre-scored JSONL files that were stripped to slim this branch. Regenerate them first:

```bash
# From repo root — see reproducibility/ for targets.yaml with pinned SHAs
bash reproducibility/regenerate-corpora.sh <data-dir>
# Then run an experiment, e.g.:
cd engine
uv run python argot/research/signal/phase14/experiments/structural_refactor_null_test_2026_04_22.py
```

## History recovery

The tag `research/phase-14-pre-cleanup` on origin contains the **full pre-cleanup history**, including all stripped corpus blobs. To recover any stripped file:

```bash
git show research/phase-14-pre-cleanup:engine/argot/acceptance/catalog/fastapi/corpus.jsonl > corpus.jsonl
```

## Blog post

TODO: link to post when published.
