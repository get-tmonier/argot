# Acceptance Catalog Design

**Date:** 2026-04-20
**Branch:** research/phase-7-honest-eval
**Status:** approved, pending implementation plan

## Problem

The existing benchmark pipeline (`corpus benchmark`) was built for encoder research: train on a large mixed corpus, measure JEPA head separation across corpus sizes and seeds. That answers a research question, not a product question.

The product question is: *"Given a model trained on this specific repo's history, does it flag code that doesn't fit this repo's paradigm?"*

Answering this requires:
- Training on a single repo's actual history (not a mixed corpus)
- Testing against paradigm breaks that are genuine for *that repo* (not generic mutations)
- Multi-scope awareness (a repo may have multiple path-scoped models)

## Solution: Acceptance Catalog

A curated catalog of benchmark repos, each with a frozen corpus sample, scope definitions derived from deep codebase analysis, and hand-crafted fixtures that trace back to specific paradigm observations.

Two distinct systems, clearly separated:

- **`engine/argot/acceptance/`** — acceptance catalog + runner. Answers "does it work on real repos?" Deterministic, CI-friendly, repo-specific.
- **`engine/argot/corpus.py` + `benchmark` CLI** — stays for encoder research. Statistical AUC across corpus sizes and seeds. Not acceptance testing.

## Catalog Structure

```
engine/argot/acceptance/
  catalog/
    {repo-name}/
      corpus.jsonl          # ~500-800 frozen records from this repo's history
      scopes.json           # scope defs + paradigm descriptions from analysis
      manifest.json         # fixture list with scope routing, hunk lines, rationale
      fixtures/
        {scope-name}/
          paradigm_break_*.ts / *.py
          control_*.ts / *.py
  runner.py                 # acceptance test runner
```

## Entry Format

### `scopes.json`

Declares path-scoped models and the paradigm observed from deep codebase analysis. Single-scope repos use `path_prefix: ""`.

```json
{
  "scopes": [
    {
      "name": "default",
      "path_prefix": "",
      "paradigm": "Concise description of dominant patterns and what's conspicuously absent — grounded in evidence from the actual codebase, not assumptions."
    }
  ]
}
```

Multi-scope example (future vigie entry):
```json
{
  "scopes": [
    {
      "name": "app",
      "path_prefix": "packages/app/",
      "paradigm": "Effect-TS: Effect.gen/yield*, Layer/Context.Service DI — no Promise, no class instantiation"
    }
  ]
}
```

### `manifest.json`

Routes each fixture to its scope. The `rationale` field links the fixture to a specific observation from the codebase analysis — this is what tells you whether a fixture is still valid as the codebase evolves.

```json
{
  "fixtures": [
    {
      "name": "paradigm_break_callbacks",
      "scope": "default",
      "file": "fixtures/default/paradigm_break_callbacks.ts",
      "hunk_start_line": 5,
      "hunk_end_line": 18,
      "is_break": true,
      "rationale": "ky uses promise chaining via .then() internally — raw callback-style with explicit resolve/reject is never used in the codebase"
    },
    {
      "name": "control_normal_request",
      "scope": "default",
      "file": "fixtures/default/control_normal_request.ts",
      "hunk_start_line": 3,
      "hunk_end_line": 12,
      "is_break": false,
      "rationale": "Standard ky request with options object — idiomatic pattern used throughout"
    }
  ]
}
```

### `corpus.jsonl`

Frozen sample of ~500-800 records extracted from the repo's git history, filtered to this repo's records from the existing bucket datasets. Committed to the argot repo for reproducibility. Format: same as the standard argot extract output (`_repo`, `author_date_iso`, `language`, `context_before`, `context_after`, `hunk_tokens`).

## Runner Behavior

For each catalog entry:

1. Load `scopes.json` — get scope list
2. Load `corpus.jsonl` — the frozen training corpus
3. For each scope: filter records by `path_prefix`, train one `pretrained` encoder model (fixed 20 epochs)
4. Load `manifest.json` + fixture files
5. Route each fixture to its scope model via `manifest.json`'s `scope` field
6. Compute per scope: `break_mean`, `control_mean`, `delta`
7. Gate per scope: `delta >= 0.20`
8. Write results to `docs/research/scoring/acceptance/{repo-name}.md`

**Overall gate: all scopes pass.** A multi-scope repo fails if any one scope fails.

## Catalog Onboarding Workflow (per new repo)

Adding a repo to the catalog requires a mandatory analysis step before any fixture is written.

**Step 1 — Codebase analysis**
Read the actual source code. Identify:
- Dominant patterns (error handling, composition, I/O, dependency injection)
- What's conspicuously absent (no classes? no callbacks? no `print()`? no `async/await`?)
- Edges: patterns used *sometimes* that would still be a break in core modules

Output: the `paradigm` field in `scopes.json`. Must be grounded in code evidence, not assumptions.

**Step 2 — Fixture writing from evidence**
Each fixture traces back to a specific observation. The `rationale` in `manifest.json` is the explicit link. Target: 6-10 fixtures per scope (4-6 breaks, 2-4 controls). Controls must be genuinely idiomatic — not just "not wrong" but "typical of this codebase."

**Step 3 — Corpus sample extraction**
Filter records for this repo from the existing bucket datasets by `_repo` field. Sample 500-800 records. Commit to `catalog/{repo}/corpus.jsonl`.

**Step 4 — Validation**
Run the acceptance test. If a break fixture doesn't score above controls: investigate whether the pattern actually exists in the corpus (fixture is wrong) or the corpus is too small. Fix at the source, not by adjusting the threshold.

## Cleanup Scope

The following are deleted as part of Phase 0:

- `engine/argot/benchmark_fixtures/` — replaced by catalog entries
- `engine/argot/scripts/spot_check.py` — replaced by `acceptance/runner.py`
- Phase 7 research scripts under `engine/argot/scripts/` that are no longer active
- `run_benchmark_density` and `_benchmark_one_density` in `corpus.py` — Phase 7.2 dead code

**All docs under `docs/research/` are preserved unchanged.**

The `corpus benchmark` CLI subcommand stays for encoder research. Its help text is updated to make clear it is for encoder experiments, not acceptance testing.

## Incremental Roadmap

### Phase 0 — Cleanup + scaffold
Delete dead code. Create `acceptance/` directory structure. No catalog entries yet — just the framework (runner, schema validation, output format).

### Phase 1 — ky acceptance test
Deep analysis of ky's codebase → `scopes.json` → 8-10 fixtures → corpus sample from bucket data → first acceptance test running end-to-end. **First real acceptance signal.**

### Phase 2 — httpx acceptance test
Same process for httpx (Python counterpart). Second acceptance signal, first cross-language validation.

### Phase 3+ — incremental
zod, requests, then medium-bucket repos as needed. Each adds a new data point. Repos with strong style contrasts (effect vs angular) prioritized.

## Success Criteria

- Phase 0: `acceptance/runner.py` exists, accepts a catalog entry path, runs without error on a minimal fixture set
- Phase 1: ky acceptance test passes gate (`delta >= 0.20` for all scopes)
- Phase 2: httpx acceptance test passes gate
- Long-term: catalog grows incrementally; each new repo entry is the output of genuine codebase analysis, not assumption
