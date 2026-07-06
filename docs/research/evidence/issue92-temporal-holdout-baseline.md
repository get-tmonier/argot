# Issue #92 — temporal-holdout FP baseline (leak-free), all 10 languages

**Date:** 2026-07-02 · **Branch:** `bench/92-temporal-holdout` (includes the
#90 convention-bar fix) · **Harness:** `argot-bench --mode holdout`
(this branch), release build.

## Method

The old FP controls replayed commits that are **ancestors** of the fit SHA —
train-on-test; FP ~0 by construction. The holdout mode measures the honest
number:

1. **Fit at an old SHA**: `head~window` first-parent steps behind the pinned
   head (default window 120), full non-shallow history, production
   `argot fit` (train → calibrate at defaults).
2. **Replay only strictly-future commits**: every non-merge commit in
   `fit..head` through the production `check --commit` path. Every replayed
   commit is asserted NOT to be an ancestor of the fit SHA (a leak aborts).
3. **Split** every scanned hunk by whether its path existed in the fit tree:
   *existing-file* edits vs *new-file* hunks.
4. **Commit-level bootstrap 95% CIs** (1000 reps; hunks within a commit are
   correlated, so hunk-level resampling would understate).

**Leak-direction control:** on rich, the *same* fit artifact replayed over 30
commits **before** the fit point scores **0/20 hunks (0.00%)**; the commits
after it score 18.7%. The entire published-FP story was the leak.

**Bug found on the way:** `check --commit <sha>` silently scanned 0 hunks for
any commit not reachable from the current checkout (git_walk started its
revwalk at HEAD and filtered). Fixed — filtered walks push the requested SHAs
as walk tips; regression test in `git_walk.rs`.

## Baseline results (frozen gates: FP existing ≤ 2%, FP new-file ≤ 5%)

FP% [bootstrap 95% CI] (hits/hunks). ⚠️ = under-sampled (<300 eligible
hunks). Fit/head pins in `benchmarks/targets.yaml`; per-corpus JSON archives
(incl. every hit with path/reason/score) under
`benchmarks/results/holdout-baseline/` (git-ignored, regenerable).

| Corpus | Lang | Window | Commits | FP existing | FP new-file | FP overall |
|---|---|---:|---:|---:|---:|---:|
| fastapi | python | 1200 | 1200 | 23.17% [16.15–39.72] (398/1718) | 26.55% [12.42–53.98] (137/516) | 23.95% (535/2234) |
| rich | python | 120 | 309 | 26.34% [20.00–33.33] (103/391) | 8.30% [3.15–31.11] (24/289) | 18.68% (127/680) |
| saleor | python | 120 | 120 | 5.61% [2.87–8.93] (48/855) | 29.63% [11.11–48.39] (8/27) | 6.35% (56/882) |
| wagtail ⚠️ | python | 120 | 120 | 8.46% [3.90–14.04] (22/260) | — (0/0) | 8.46% (22/260) |
| hono | typescript | 120 | 137 | 6.41% [3.21–11.38] (23/359) | 0.00% (0/6) | 6.30% (23/365) |
| faker-js | typescript | 120 | 120 | 0.70% [0.10–5.08] (8/1149) | 2.46% [0.00–7.32] (3/122) | 0.87% (11/1271) |
| excalidraw | typescript | 120 | 120 | 5.26% [1.97–9.76] (61/1159) | 27.66% [14.85–41.67] (39/141) | 7.69% (100/1300) |
| outline | typescript | 120 | 126 | 4.81% [2.35–9.63] (45/935) | 18.75% [4.55–37.50] (6/32) | 5.27% (51/967) |
| gh-cli | go | 120 | 212 | 5.26% [2.48–9.31] (31/589) | 11.59% [6.61–21.04] (64/552) | 8.33% (95/1141) |
| hugo | go | 120 | 120 | 11.00% [7.71–14.56] (56/509) | 39.13% [18.18–57.14] (9/23) | 12.22% (65/532) |
| ripgrep | rust | 120 | 120 | 13.56% [7.04–24.16] (43/317) | 46.15% [0.00–100.00] (6/13) | 14.85% (49/330) |
| bat | rust | 250 | 443 | 44.67% [34.46–54.46] (151/338) | — (0/0) | 44.67% (151/338) |
| guava | java | 120 | 120 | 10.77% [3.84–16.72] (261/2424) | 0.00% (0/2) | 10.76% (261/2426) |
| junit5 ⚠️ | java | 120 | 120 | 5.23% [1.69–14.00] (9/172) | 33.33% (1/3) | 5.71% (10/175) |
| powershell ⚠️ | csharp | 120 | 120 | 1.08% [0.00–3.92] (1/93) | 100.00% (1/1) | 2.13% (2/94) |
| jellyfin | csharp | 120 | 176 | 26.70% [21.21–33.42] (118/442) | 33.33% [18.75–48.48] (26/78) | 27.69% (144/520) |
| redis | c | 120 | 120 | 4.46% [1.41–8.67] (25/561) | 77.42% [0.00–85.71] (24/31) | 8.28% (49/592) |
| curl | c | 120 | 120 | 0.50% [0.00–1.34] (2/400) | 0.00% (0/2) | 0.50% (2/402) |
| rocksdb | cpp | 120 | 120 | 8.74% [5.76–12.55] (195/2230) | 56.36% [23.53–81.36] (31/55) | 9.89% (226/2285) |
| fmt ⚠️ | cpp | 120 | 120 | 15.08% [8.04–24.60] (19/126) | — (0/0) | 15.08% (19/126) |
| homebrew | ruby | 120 | 166 | 13.97% [7.18–21.66] (64/458) | 7.03% [3.50–25.00] (9/128) | 12.46% (73/586) |
| rubocop ⚠️ | ruby | 120 | 129 | 7.27% [3.42–11.91] (21/289) | 0.00% (0/1) | 7.24% (21/290) |
| laravel | php | 120 | 171 | 2.73% [0.39–5.88] (13/476) | 11.48% [0.00–24.53] (7/61) | 3.72% (20/537) |
| composer | php | 120 | 141 | 3.35% [1.62–5.69] (16/478) | 7.55% [0.00–14.63] (4/53) | 3.77% (20/531) |

**Verdict: 2 of 24 corpora (curl 0.50%, faker-js 0.70%) meet the FP(existing)
≤ 2% gate.** Every language except C misses on at least one corpus. The
published per-language "FP ≤ 2%" claims do not hold under honest measurement.

Corpus notes:
- okhttp was tried and dropped as the 2nd Java corpus (recent history is
  Kotlin: 1 java hunk / 120 commits); junit5 replaces it.
- fastapi at window 120 yields only 13 hunks (docs-heavy history) → window
  1200. bat re-run at window 250 for sample size (rate unchanged, 44.7%).
- powershell / junit5 / fmt / wagtail / rubocop remain under-sampled at
  window 120; their rates carry wide CIs but none is near the gate except
  powershell.

## Root-cause diagnosis

1. **Not drift.** On rich, FP(existing) by quartile of distance-from-fit:
   23.1% / 36.6% / 24.1% / 20.2% — flooding starts with the first post-fit
   commit, so re-fitting more often would not fix it.
2. **Concentrated + repeated.** 70/103 of rich's existing-file FPs are in 2
   files (traceback.py, cells.py); 72/103 hits are re-flags of the same
   ~20-line region across successive commits. The model flags a legitimate
   novel region once, then re-flags it in every commit that touches it.
3. **BPE dominates.** rich reasons: existing = bpe 89, call_receiver 8,
   convention 3, import 3; new = bpe 23, call_receiver 1.
4. **The calibration is itself train-on-test.** The threshold is
   `median over seeds of max(cal-hunk scores)` where calibration hunks are
   sampled from corpus files the BPE token counts were trained on. Every
   token in a cal hunk is repo-attested, so cal scores are systematically
   deflated; genuinely-unseen-but-idiomatic code lands above the max. Same
   leak as the bench, one layer down.

## LOO scout (dirty experiment, rich, bpe-only)

The BPE model is a unigram count table, so exact leave-one-file-out is
"subtract the held-out file's token counts" — no retraining
(dirty scout script, deleted after recording per the research workflow):

| | threshold | FP existing | FP new-file | breaks caught (bpe-only) |
|---|---:|---:|---:|---:|
| standard calibration | 4.13 | 211/605 (34.9%) | 96/293 (32.8%) | 13/16 |
| LOO calibration | 7.82 | 2/605 (**0.33%**) | 4/293 (**1.37%**) | **0/16** |

(bpe-only replay differs slightly from the production-path numbers above —
no contributions/suppressions — but the deltas are the signal.)

**Both readings matter.** LOO fixes FP completely, and it exposes that in
unigram-surprise space *unseen-idiomatic* and *foreign-break* code overlap
almost totally: the leaky threshold was the only thing making the BPE stage
look discriminative. Production recall also rides on the import tripwire and
cluster/convention bonuses, so the full-pipeline recall cost of LOO must be
measured through the real bench before any conclusion.

## Next

1. Implement LOO calibration in `run_calibrate`/bench (production path).
2. Re-bench: production-mode recall + holdout FP across all corpora.
3. Honest recall requires curated spliced fixtures for all 10 languages
   (the ad-hoc foreign-import fixtures are softballs — issue #92).
