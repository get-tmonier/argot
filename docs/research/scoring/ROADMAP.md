# Scoring Research — Roadmap

> Read this at the start of every session. Update it at the end.

**Current phase**: Phase 2 — sizing study (in progress — corpus reclassification, re-running small + xlarge benchmarks)
**Active branch**: `research/scoring-benchmark`
**Last touched**: 2026-04-19
**Spec**: [`DESIGN.md`](DESIGN.md)

---

## Phase 1 — benchmark infrastructure

- [x] 1a. Add `--repo-name` to `argot-engine extract`; stamp `_repo` on records
- [x] 1b. Add `argot-engine corpus concat` utility
- [x] 1c. Add `argot-engine corpus benchmark` batch runner
- [x] Unit tests for downsample/stratify + concat
- [x] `just research benchmark` target wired
- [x] Verify suite green


## Phase 2 — sizing study

- [x] 01-corpus.md: pin repo URLs + SHAs — initial pass complete; reclassified
      after audit (see decisions below)
- [x] Clone + extract: argot, ruff, click, vite, effect, pydantic, django done
- [ ] Clone + extract: vscode (xlarge TS) + TBD small TS repo
- [x] Concat + benchmark: medium (7k) and large (20k) complete — results valid
- [ ] Rebuild + re-benchmark: small (3k) and xlarge (60k) — pending new repos
- [ ] 02-sizing-study.md: AUC-vs-size table + interpretation

**Blocking**: Phase 1 complete.

## Phase 3 — technique experiments

Each gets its own branch + PR. Measurement protocol: re-run Phase 2 corpus
benchmark on the variant, compute AUC delta per bucket, write a log.

- [ ] 03-context-after.md — wire `context_after` into training (**S**)
- [ ] 04-embed-dim.md — 128 → 256 (**XS**)
- [ ] 05-epochs.md — 50 → 200 (**XS**)
- [ ] 06-char-ngrams.md — TF-IDF char n-grams (**S**)
- [ ] 07-imports-scope.md — imports as a separate signal (**M**)
- [ ] 08-path-embed.md — file path embedding (**M**)

**Stop rule**: if AUC lift < 0.01 across all buckets, document and move on
without merging code.

**Blocking**: Phase 2 complete (baseline numbers).

## Phase 4 — synthesis

- [ ] 99-synthesis.md: overall findings, ranked technique list, final
      recommendations for the separate calibration branch
- [ ] Update `docs/scoring.md` with user-facing numbers (minimum repo size,
      model quality characteristics)

**Blocking**: Phase 3 techniques each resolved.

## Phase 7.0 — honest evaluation rebuild

- [x] 7.0 eval rebuild — honest corpus with 12 repos (6 buckets × 2 langs),
      pinned SHAs, per-lang benchmarking. `mutations.py` with 4 mutations
      (case_swap, debug_inject, error_flip, quote_flip). `_benchmark_one`
      emits synthetic_auc_mean + per-mutation AUCs + cross_auc_same_lang.

**Blocking**: None; new baseline for Phase 7.1+.

---

## Decisions made

- 2026-04-18 — corpus composition: 2 repos per bucket (TS + Py),
  5 buckets total (see DESIGN.md §Phase 2).
- 2026-04-18 — documentation lives in this repo, not a separate research
  repo.
- 2026-04-18 — all 6 techniques attempted; stop rule = AUC lift < 0.01.
- 2026-04-18 — calibration work (threshold recalibration from percentiles)
  is deferred to a separate branch after this research completes.
- 2026-04-18 — corpus reclassification: buckets assigned by extracted record
  count, not repo name/fame. zod (14,715 records) dropped — medium-scale, no
  clean bucket. typescript-compiler (35,246 records, 94% JS fixtures) dropped
  in favour of vscode. ruff moved from medium to small (3,343 records matches
  small target). click + vite confirmed as medium. small TS slot TBD.

## Open questions (parked)

_None yet._

## Session log

- **2026-04-18**: design approved; branch created; DESIGN.md + ROADMAP.md
  committed. Next: write Phase 1 implementation plan via writing-plans skill.
- **2026-04-18**: Phase 1 complete. `argot-corpus concat` and
  `argot-corpus benchmark` land, `extract --repo-name` stamps `_repo`.
  `just research-concat` and `just research-benchmark` wired. Next: Phase 2
  corpus kickoff — pin repo URLs + SHAs in `01-corpus.md`.
- **2026-04-18**: Phase 2 in progress. Extracted 7 repos. Medium + large
  benchmarks complete and valid. Corpus audit triggered reclassification:
  buckets now assigned by record count; zod and typescript-compiler dropped;
  vscode added for xlarge TS; small TS slot TBD. benchmark fix: streaming
  JSONL load in `corpus.py` (was crashing on 9.5 GB xlarge file). Re-running
  small + xlarge benchmarks after new repo acquisition.
- **2026-04-19**: Phase 7.0 complete. Honest corpus pinned (12 repos — httpx, requests, ky, zod, fastapi, flask, vite, rollup, pydantic, django, effect, angular — SHAs recorded in `.argot/research/datasets-v2/SHAS.md`). `mutations.py` with 4 mutations (case_swap, debug_inject, error_flip, quote_flip) landed with unit-test coverage. `_benchmark_one` now emits `synthetic_auc_mean` + per-mutation AUCs + `cross_auc_same_lang`. Sanity run on small-py confirmed end-to-end (cross_auc_same_lang=0.628, synthetic_auc_mean=0.507 — 3/4 mutations invisible to TF-IDF as expected; Phase 7.1 tests char_ngrams/BPE). Note: chalk and axios were dropped from small-ts (JS-heavy); replaced with zod (14,575 pure TS records). Next: Phase 7.1 — re-baseline 4 existing encoders on the new eval.
