# Scoring Research — Roadmap

> Read this at the start of every session. Update it at the end.

**Current phase**: Phase 2 — sizing study (not started)
**Active branch**: `research/scoring-benchmark`
**Last touched**: 2026-04-18
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

- [ ] 01-corpus.md: pin repo URLs + SHAs for 9 target repos (argot self +
      2 per bucket × 4 buckets)
- [ ] Clone + extract each → `.argot/research/datasets/<slug>.jsonl`
- [ ] Concat per bucket; run `corpus benchmark` at bucket sizes × 3 seeds
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

---

## Decisions made

- 2026-04-18 — corpus composition: 2 repos per bucket (TS + Py),
  5 buckets total (see DESIGN.md §Phase 2).
- 2026-04-18 — documentation lives in this repo, not a separate research
  repo.
- 2026-04-18 — all 6 techniques attempted; stop rule = AUC lift < 0.01.
- 2026-04-18 — calibration work (threshold recalibration from percentiles)
  is deferred to a separate branch after this research completes.

## Open questions (parked)

_None yet._

## Session log

- **2026-04-18**: design approved; branch created; DESIGN.md + ROADMAP.md
  committed. Next: write Phase 1 implementation plan via writing-plans skill.
- **2026-04-18**: Phase 1 complete. `argot-corpus concat` and
  `argot-corpus benchmark` land, `extract --repo-name` stamps `_repo`.
  `just research-concat` and `just research-benchmark` wired. Next: Phase 2
  corpus kickoff — pin repo URLs + SHAs in `01-corpus.md`.
