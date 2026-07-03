# Issue #92 Phase A — FP root-cause diagnosis (evidence)

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Source data:
`benchmarks/results/holdout-loo/` (temporal-holdout FP, per-hit reason/score) and
`benchmarks/results/recall-loo/` (production-path per-fixture `max_score`).
Baseline reproduced exactly from a fresh scoped run (`--mode holdout --corpus
redis` → 0.71% existing / 61.29% new-file, matching
[issue92-honest-rebench.md](issue92-honest-rebench.md)).

Two findings below are decisive and reshape what Phase A can and cannot do.

## Finding 1 — missed hard-class breaks score **0.00**, not near-miss

Per-fixture `max_score` on the curated catalogs (production path, LOO
thresholds). Caught breaks fire via `call_receiver`/`import` in the 6.5–11.8
band; **every uncaught break scores exactly 0.00** — it produces no signal at
any stage.

| Corpus | caught band (score) | uncaught | uncaught score |
|---|---|---|---|
| redis | 6.51, 7.04, 9.52 | 11/14 | all 0.00 |
| rocksdb | 6.59, 7.24, 7.92 | 10/13 | all 0.00 |
| homebrew | 7.09, 7.66, 9.11, 10.22 | 8/13 | all 0.00 |
| laravel | 8.67–11.75 (8 caught) | 5/13 | all 0.00 |
| excalidraw | 7.68–10.17 (9 caught) | 5/14 | all 0.00 |

The uncaught classes are uniform: `wrong_error_discipline`, most
`wrong_api_within_known_lib`, `naming_shape_break`. They are composed of tokens
and callees the repo already attests, so unigram-surprise + name-attestation
emit nothing. **No threshold change can recover a score-0 fixture.** The recall
gate (≥85%, currently met only by Python) is therefore **unreachable by any
calibration/threshold work** — it requires the structurally-different scorer
(Phase B). This is the dominant blocker for the mission.

## Finding 2 — existing-file `call_receiver` FP and true-break scores fully overlap

The existing-file FP reds are `call_receiver`-dominated: bat 33/39, rocksdb
134/139, hugo 24/30, homebrew 21/21, rubocop 7/10. Their hunk scores:

- bat existing-file FPs: 10.75, 10.86, 11.55, 11.64, 12.99 (threshold 8.87)
- caught true breaks (same scorer): 6.5–11.75 across corpora

The FP band (≈10–13) sits **inside and above** the true-break band (≈6.5–11.8).
Raising the existing-file threshold to suppress bat's FPs would suppress true
breaks first. bat's worst FP is the `libgit2 → gitoxide` migration (commit
c5e6f6aa): a legitimate library swap introduces a whole new `gix::*` callee
surface — indistinguishable, in name-attestation space, from a foreign-API
break. **This is a fundamental limit of the name-surprise scorer, provable from
the score overlap** (feeds Phase C). Existing-file `call_receiver` reds
(bat, hugo, homebrew, rubocop, rocksdb-cpp) are not fixable by tuning.

## Finding 3 — new-file floods are three distinct mechanisms

New-file hits by reason + file class (holdout-loo archives):

| Corpus | new hits | mechanism | reasons |
|---|---:|---|---|
| excalidraw | 30 | **native-language idiomatic new files** (TTDDialog chat feature, hooks, stores — all `.ts/.tsx`) | cr 17 / bpe 10 / import 3 |
| rocksdb | 27 | **minority-language** (Python tooling + C-API gen in a C++ repo — all `.py`) | import 22 / cr 5 |
| redis | 19 | **vendored dependency** (18/19 are `deps/tre/lib/*`, the third-party TRE regex lib) | cr 7 / bpe 5 / import 7 |

These need different, independently-principled fixes:

1. **New-file threshold (file-level LOO).** The calibration threshold is the max
   over cal hunks sampled from fit-corpus files; the BPE side is already scored
   leave-one-file-out, but the `call_receiver` cluster branches do not fire on
   cal hunks (new files are not cluster-routed at fit), so the threshold has
   ~no call_receiver mass. A real new file *is* nearest-cluster-routed and its
   unattested callees add `alpha`/`cluster_bonus`, landing above a threshold
   that never saw that contribution. Fix: calibrate a **separate new-file
   threshold** by scoring each fit file *as if newly added* (its own callees
   removed from the global attested set via `callee_file_counts` subtraction;
   its tokens removed from BPE counts — the LOO already does the latter), take
   the max/quantile, and apply it only to new-file hunks (path absent from the
   fit `repo-corpus.txt`). This gates the bpe+call_receiver axis, not the import
   tripwire. Directly targets excalidraw-new (30 → ~import-only 3). Leaves the
   existing-file threshold and curated recall (fixtures spliced into existing
   files) untouched. This is the file-level analogue of the shipped BPE
   token-LOO, and is **distinct** from rejected-fix #3 (LOO of the
   call_receiver *attestation set*, which starved the cluster-rare estimate).

2. **Minority-language attestation guard.** rocksdb-new is 22/27 `import` on new
   `.py` files: with few Python files in the fit corpus, the import scorer's
   `is_foreign` treats ordinary Python imports as foreign. Principled fix: gate
   a language's import tripwire (and/or apply only the new-file threshold) when
   the fit corpus has fewer than K files of that language — foreignness is
   unattested below a minimum sample. No domain literals; the gate is on
   attestation strength (file count), measured from the fit corpus.

3. **Vendored dependency (redis `deps/tre`).** This is foreign C code correctly
   flagged; the bench scores it FP only because the vendoring commit is
   legitimate. Options: honest known-behavior (vendoring foreign code flags — a
   defensible catch), or exclude vendored paths already covered by
   `suppress::recommended_excluded` (verify). Not a scorer bug. Do not
   path-hack for a green number.

## What Phase A can move (honest ceiling)

- **Fixable:** excalidraw-new (30→~3, PASS), the native-language portion of any
  new-file flood, and rocksdb-new via the minority-language guard.
- **Not fixable by tuning (fundamental limit, prove & mark honest):**
  existing-file `call_receiver` reds (bat/hugo/homebrew/rubocop/rocksdb-cpp),
  and the entire hard-class recall gap on all non-Python languages.

Phase A improves FP on a subset. The mission's gate-clearing on recall (and on
the migration-class existing-file FP) hinges on Phase B, not on calibration.

## Phase A result — new-file threshold (file-level LOO), validated

Implemented a separate **new-file threshold**: calibration scores each fit file
*as if newly added* (cluster routing off, real check-time `alpha`, attestation
requiring `df ≥ 2` — exact file-LOO), takes the median-over-seeds max, floors it
at the existing-file threshold, and writes `new_file_threshold` per language into
`scorer-config.json`. `check` applies it only to hunks whose file was absent from
the fit corpus (`SequentialImportBpeScorer::is_fit_file`, keyed by the same
repo-relative path the scorer routes clusters on). Existing-file scoring, the
recall path (fixtures splice into existing host files), and configs predating the
field are all untouched. Root cause it closes: calibration passed `alpha = 0`
while check passes `alpha = 2`, so the threshold held zero unattested-callee mass
— any novel callee crossed it, and a whole-new file (all callees novel) flooded.

**excalidraw (the pure native-language new-file case), holdout w120, same fit SHA:**

| split | baseline | with new-file threshold |
|---|---|---|
| new-file FP | 21.28% (30/141) ❌ | **2.84% (4/141) ✅** |
| existing-file FP | 3.45% (40/1159) | **3.45% (40/1159)** (byte-identical) |

Reason breakdown new-file: `cr 17 / bpe 10 / import 3` → `cr 1 / import 3`. The
threshold suppressed all 10 bpe and 16/17 call_receiver new-file hits; the 3
import hits remain by design (import is a tripwire, ungated by the calibrated
threshold). Existing-file reasons unchanged (`cr 16 / import 24`). Unit tests:
`as_new_fires_alpha_on_singleton_df_callees`, `knows_file_tracks_fit_membership`;
`check_evidence_parity` updated to pin both thresholds. 351 core tests green.

### Full 24-corpus `--mode honest` re-bench (clean run, corrected code)

A recall regression surfaced on the first re-bench: **faker 16/16 → 13/16**. Root
cause — faker's provider files (`faker/providers/*/__init__.py`) are
data-dominant (lists of fake names/domains), so they are filtered out of
clustering; `is_fit_file`'s cluster-membership proxy misclassified those known
host files as *new*, and fixtures spliced into them were judged against the
higher new-file threshold and suppressed. Fix: snapshot the authoritative
fit-corpus file set (`corpus_files`, repo-relative, **including** data-dominant
files) into `scorer-config.json`; `check` detects new files as
`!corpus_files.contains(path)`, falling back to cluster membership only for
configs predating the field. faker restored to 16/16; excalidraw new-file win
intact (2.84%).

Clean full re-bench (`benchmarks/results/nft-full2/`), all 24 corpora:

- **Existing-file FP: zero regressions** — every corpus byte-identical to the
  baseline table (bat 11.54%, rocksdb 6.23%, jellyfin 9.73%, …). The new-file
  threshold provably never touches existing-file hunks.
- **Recall: zero regressions** — all 19 recall corpora unchanged from baseline
  (rich 11/16, laravel 8/13, guava 8/14, redis 3/14, …); faker 16/16.
- **New-file gates newly cleared (8):** excalidraw 21.3→2.8, powershell 20→0,
  hugo 13→0, jellyfin 14.1→2.6, junit5 8.7→0, ripgrep 7.7→0, wagtail 27.3→0,
  gh-cli 7.8→1.7. Already-green corpora also improved (fastapi 3.9→0.4, homebrew
  4.7→0, saleor 3.7→0, rich 3.1→1.4).
- **New-file still red (6), all import-dominated or special:** rocksdb 40.0%
  (import 22 — Python-in-C++), redis 32.3% (import 7 + cr 3 — vendored TRE lib),
  fmt 20.0% (import), rubocop 9.1% (import, thin n=11), outline 6.2% (import 2),
  laravel 11.5% (cr 7 — new dev-tooling feature files whose PHP as-new threshold
  ceiling did not rise above the existing bar). The import residue is the
  minority-language import problem (Finding 3, mechanism 2) — the new-file
  threshold correctly does not gate the import tripwire; that is the
  import-guard follow-on.

Reproduced independently on a fix-verify run (faker 16/16, excalidraw
existing/new 3.45%/2.84%). Net: **8 new-file gates cleared, zero existing-file or
recall regression on any of the 10 languages.** The existing-file reds
(bat/hugo/homebrew/rubocop/rocksdb-cpp/jellyfin/fastapi) are unchanged — those
are the call_receiver / import fundamental-limit and structural-recall problems,
not addressable by the new-file threshold (Findings 1–2).

## Import residue — considered and left as an honest limit

The residual new-file reds are import-dominated: rocksdb 40% (22 import on new
`.py` — Python tooling in a C++ repo), redis 32% (vendored TRE C lib), fmt 20%
(a new `src/fmt-c.cc` C-API subsystem — *primary* language), rubocop 9.1% (1
hit, thin n=11), outline 6.2% (2 new util files). A **minority-language import
guard** (suppress the import tripwire for a language with fewer than K fit
files) would clear *only* rocksdb — fmt/redis/outline are the primary language,
not a minority. And any specific K is a knob tuned to make one corpus green,
which the mission's rubric forbids ("never tune to a bar"). The honest read: the
import residue is the same fundamental ambiguity as [Finding 2] on the import
axis — a foreign-import tripwire cannot distinguish a new file *legitimately
adding a dependency* (new C API, vendored lib, new tooling) from a foreign-voice
break. The new-file threshold fixed the calibratable (bpe/call_receiver) mass;
this residue is reported red as a known limitation, not tuned away.

## Phase B / recall

Recall's hard-class gap is the dominant blocker and is a *proven* limit — a
structural scorer (pretrained-embedding manifold-outlier, per-token MLM) was
seriously attempted and both plateau at ~0.65 AUC once fairly controlled. See
[issue92-phaseB-manifold-outlier.md] and [issue92-phaseB-pertoken-mlm.md]. The
existing-file call_receiver reds (bat/hugo/homebrew/rubocop/fastapi) are the
same limit on the FP side (Finding 2, score overlap). Both are reported red and
the affected languages marked not-yet-shippable, per the mission.
