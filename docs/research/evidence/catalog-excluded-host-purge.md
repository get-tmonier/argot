# Catalog amendment — purge fixtures hosted under excluded paths

**Date:** 2026-07-14 · **Branch:** `fix/launch-punchlist` · Scope:
`benchmarks/catalogs/{dagster,fastapi}/manifest.yaml` + a harness guard in
`crates/argot-bench/src/production.rs`.

## What happened

The 2026-07-09 decision to bench every corpus under the `argot.toml` a real
maintainer would write (per-corpus `[exclude].paths`, installed by
`sync_corpus_config`) made a subset of existing fixtures structurally
uncatchable, and nobody purged them:

- **dagster** excludes `js_modules/` (the JS UI is peripheral to the Python
  library's voice). With no TypeScript corpus left, the fit calibrates no
  TypeScript threshold at all, and the 20 fixtures hosted under
  `js_modules/` (17 gated foreign, 3 secondary) can never fire. Verified: a
  production-defaults fit of the same worktree **without** the exclusion
  calibrates all four languages (typescript threshold 6.99).
- **fastapi** excludes `docs_src/` and `scripts/` (tutorial snippets, dev
  tooling). 15 fixtures were hosted under `docs_src/` (4 gated, 11
  secondary); python itself stays calibrated, but hits on user-excluded
  paths are suppressed and never reach `hits[]`, so they can't count.

The 2026-07-14 full run surfaced this as a drop in the gated-visible
foreign catch (604/618 → 595/618) concentrated on dagster — a config
artifact deflating the headline, not a detection regression.

## The amendment

Under the RUBRIC, a fixture that fails to fire is a finding, never a reason
to swap the fixture — but these fixtures no longer *measure* anything: their
hosts are outside the benched configuration by deliberate scope decision.
Removed from the manifests (break files deleted with them):

- dagster: 20 fixtures hosted under `js_modules/` (all TS: framework_swap
  1-4, state_management 1-2, routing 1-2, styling 1-2, data_fetching 1-2,
  foreign_import zustand/framer_motion/react_hook_form, foreign_concurrency
  rxjs/comlink, foreign_api zod/xlsx_dynamic/react_query). 40 → 20 fixtures.
- fastapi: 15 fixtures hosted under `docs_src/` (exception_handling 1-4,
  downstream_http 1+3, async_blocking 1+3, dependency_injection_1,
  serialization_2, routing_3, foreign_concurrency
  apscheduler/celery/eventlet/gevent). 56 → 41 fixtures.

Dagster's TS foreign coverage is not replaceable within this scope decision
(the repo has no non-`js_modules` TypeScript); TS foreign coverage lives in
the seven dedicated TS/JS corpora.

## The guard

`run_corpus_production` now loads the synced corpus config and **fails the
run** if any fixture's `host_file` is suppressed
(`ArgotConfig::path_suppressions().is_suppressed`), listing the offending
ids — a catalog/config divergence can no longer silently deflate (or a
future exclusion silently inflate) the published number.

## Re-scoring

The definitive post-purge run regenerates `landing/src/data/benchmarks/*`
and `landing/src/data/foreign.json` from one run at one commit; the
benchmarks page and README numbers follow it.
