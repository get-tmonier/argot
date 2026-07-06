# Issue #92 — honest re-bench after the calibration fixes

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Protocol as
in [issue92-temporal-holdout-baseline.md](issue92-temporal-holdout-baseline.md)
(leak-free temporal holdout; commit-level bootstrap 95% CIs).

## What changed between baseline and this run

1. **Leave-one-file-out (LOO) calibration** (`bpe_score_excluding`): the
   threshold is now calibrated on hunks scored as if their file were not in
   the corpus — the condition check meets on post-fit code. The BPE model is
   a unigram count table, so LOO is exact count subtraction.
2. **Repo-owned import namespaces**: Go (go.mod module paths), Java
   (`package` declarations), C# (`namespace` declarations) now attest their
   own namespaces as prefixes — never-before-imported *internal* symbols no
   longer trip the foreign-import stage (guava: 183 of 223 honest FPs were
   `import static com.google.common.…`).
3. **Rejected fix (recorded):** LOO for the call-receiver attestation. At
   check time the cluster branches only fire on files the fit clustered
   (new files are not cluster-routed), so memorized calibration is already
   symmetric there; a LOO probe starved the cluster-rare fire-rate estimate
   (4/4 → 0/4 on the check fixture) and flipped keep/disable the wrong way.
   Reverted after measurement.
4. **Deleted invalid metric**: production mode's ancestor-replay FP control
   is gone; `--mode honest` (new default) = production recall + holdout FP.
5. **gh-cli re-pinned** to upstream `397876d3` — the old pin was a local
   commit carrying planted fixture files (corpus contamination found during
   fixture authoring).

## Temporal-holdout FP after the fixes (gates: existing ≤ 2%, new-file ≤ 5%)

| Corpus | Lang | FP existing | FP new-file | Verdict (existing / new) |
|---|---|---:|---:|---|
| fastapi (w1200) | python | 6.58% [4.56–11.88] (113/1718) | 3.88% [1.58–9.87] (20/516) | ❌ / ✅ |
| rich | python | 2.81% [1.34–4.71] (11/391) | 3.11% [1.06–11.94] (9/289) | ❌ / ✅ |
| saleor | python | 0.94% [0.00–3.24] (8/855) | 3.70% (1/27) | ✅ / ✅ |
| wagtail (w250) | python | 1.69% [0.55–3.05] (9/531) | 27.27% (3/11, thin) | ✅ / ❌(thin) |
| hono | typescript | 1.39% [0.24–3.30] (5/359) | 0.00% (0/6) | ✅ / ✅ |
| faker-js | typescript | 0.09% [0.00–1.02] (1/1149) | 0.00% (0/122) | ✅ / ✅ |
| excalidraw | typescript | 3.45% [1.02–7.64] (40/1159) | 21.28% [9.90–34.21] (30/141) | ❌ / ❌ |
| outline | typescript | 2.99% [1.32–6.62] (28/935) | 12.50% [0.00–27.03] (4/32) | ❌ / ❌ |
| gh-cli | go | 2.30% [0.98–3.63] (14/608) | 7.78% [4.47–13.30] (42/540) | ❌ / ❌ |
| hugo | go | 5.89% [3.16–8.97] (30/509) | 13.04% [0.00–30.77] (3/23) | ❌ / ❌ |
| ripgrep | rust | 0.95% [0.00–2.48] (3/317) | 7.69% (1/13) | ✅ / ❌ |
| bat (w250) | rust | 11.54% [7.60–16.67] (39/338) | — (0/0) | ❌ / — |
| guava | java | 2.06% [1.14–3.43] (50/2424) | 0.00% (0/2) | ❌(borderline) / ✅ |
| junit5 (w300) | java | 2.93% [0.75–5.98] (11/375) | 8.70% (2/23, thin) | ❌ / ❌(thin) |
| powershell (w800) | csharp | 1.78% [1.01–2.86] (32/1799) | 20.00% [10.00–34.78] (6/30) | ✅ / ❌ |
| jellyfin | csharp | 9.73% [7.10–13.17] (43/442) | 14.10% [2.50–28.26] (11/78) | ❌ / ❌ |
| redis | c | 0.71% [0.16–1.39] (4/561) | 61.29% [0.00–67.86] (19/31) | ✅ / ❌ |
| curl | c | 0.25% [0.00–0.85] (1/400) | 0.00% (0/2) | ✅ / ✅ |
| rocksdb | cpp | 6.23% [4.23–8.79] (139/2230) | 49.09% [19.44–77.94] (27/55) | ❌ / ❌ |
| fmt (w500) | cpp | 2.63% [1.35–4.14] (24/913) | 57.14% [42.86–74.29] (20/35) | ❌ / ❌ |
| homebrew | ruby | 4.59% [2.21–7.89] (21/458) | 4.69% [0.35–25.00] (6/128) | ❌ / ✅ |
| rubocop (w250) | ruby | 6.96% [4.49–9.27] (55/790) | 54.55% (6/11, thin) | ❌ / ❌ |
| laravel | php | 0.84% [0.00–2.21] (4/476) | 11.48% [0.00–24.53] (7/61) | ✅ / ❌ |
| composer | php | 0.00% (0/478) | 3.77% [0.00–7.32] (2/53) | ✅ / ✅ |

All rows are properly sampled (≥300 eligible hunks except where a new-file
split is marked "thin"); per-target windows are pinned in `targets.yaml`.
Baseline → now, FP(existing): rich 26.3→2.8, fastapi 23.2→6.6, guava
10.8→2.1, jellyfin 26.7→9.7, bat 44.7→11.5, gh-cli 5.3→2.3, redis 4.5→0.7,
homebrew 14.0→4.6. **10 of 24 corpora pass the existing-file ≤ 2% gate**
(saleor, wagtail, hono, faker-js, ripgrep, powershell, redis, curl, laravel,
composer; baseline under the honest protocol: 2). Widening the under-sampled
corpora flipped junit5's apparent 0.0% to a real 2.93% — an under-sampling
false pass, exactly why the sample bar exists. New-file flooding is fixed
where BPE drove it; the residual red on both splits is now **call_receiver**
(bat 33/39, rocksdb, hugo, gh-cli-new 37/42, jellyfin +21 convention) —
cluster-bonus contributions on legitimately new callee names. That is a
scorer-design limitation, not a calibration leak (see rejected fix #3), and
is reported red.

## Honest recall — curated hard-class catalogs (gate: ≥ 85%)

Production path (fixture planted on disk, real fit + `check --staged`), LOO
thresholds. New catalogs follow `benchmarks/catalogs/RUBRIC.md` (frozen
before scoring: ≥3 wrong_error_discipline, ≥2 wrong_concurrency, ≥3
wrong_api_within_known_lib, ≥2 naming_shape_break, ≤2 foreign_import).

| Corpus | Lang | Recall | Verdict |
|---|---|---:|---|
| rich | python | 11/16 (68.8%) | ❌ |
| fastapi | python | 32/32 (100%) | ✅ (legacy catalog) |
| faker | python | 16/16 (100%) | ✅ (legacy catalog, softer classes) |
| saleor | python | 14/14 (100%) | ✅ (legacy catalog) |
| wagtail | python | 14/14 (100%) | ✅ (legacy catalog) |
| hono | typescript | 12/17 (70.6%) | ❌ |
| faker-js | typescript | 8/17 (47.1%) | ❌ |
| ink | typescript | 15/17 (88.2%) | ✅ (legacy catalog) |
| excalidraw | typescript | 9/14 (64.3%) | ❌ |
| outline | typescript | 9/14 (64.3%) | ❌ |
| dagster | multi | 18/24 (75.0%) | ❌ |
| gh-cli | go | 5/13 (38.5%) | ❌ |
| ripgrep | rust | 4/13 (30.8%) | ❌ |
| guava | java | 8/14 (57.1%) | ❌ |
| powershell | csharp | 7/13 (53.8%) | ❌ |
| redis | c | 3/14 (21.4%) | ❌ |
| rocksdb | cpp | 3/13 (23.1%) | ❌ |
| homebrew | ruby | 5/13 (38.5%) | ❌ |
| laravel | php | 8/13 (61.5%) | ❌ |

**Aggregate honest recall: 201/301 (66.8%). No language other than Python
meets the 85% gate; on the hard-rubric catalogs specifically, no language
meets it.** The miss
pattern is uniform: foreign-import and some concurrency/API fixtures fire;
in-vocabulary breaks (error discipline, naming shape, API misuse composed
of tokens the repo already uses) do not. This is the flip side of the
honest threshold: the leaky calibration used to sit low enough that those
fixtures crossed — on the same distribution that produced 5–45% FP on
legitimate code. The old "100% recall / FP ≤ 2%" pairs were two views of
the same leak.

## The honest trade-off, stated plainly

With a unigram-surprise scorer, "unseen but idiomatic" and "foreign voice"
overlap heavily in score space. The honest operating point catches
tripwire-class breaks (foreign imports, strongly foreign API surfaces) at
low FP; it does not deliver the advertised hard-class recall at any
threshold. Closing that gap needs a scorer that models *sequence/structure*
rather than token rarity — future research, not tuning.

## Skeptical reproduction (fresh clones)

rich, guava and laravel were re-cloned from GitHub into an empty data dir
(no cached datasets, no fitted artifacts, no planted fixtures possible) and
re-run end-to-end via `argot-bench --mode honest`. Every number reproduced
**exactly** — FP existing/new and recall identical to the tables above
(deterministic pipeline + pinned SHAs): rich 2.81%/3.11% + 11/16, guava
2.06%/0.00% + 8/14, laravel 0.84%/11.48% + 8/13.

## Sample-bar follow-up (widened windows)

The five under-sampled corpora were re-run with per-target `holdout_window`
sized to clear the 300-hunk bar (now pinned in `targets.yaml`):

| Corpus | Window | FP existing | FP new-file |
|---|---:|---:|---:|
| wagtail | 250 | 1.69% [0.55–3.05] (9/531) — python split 0.65% (3/459), typescript 8.33% (6/72) | 27.27% (3/11, thin) |
| junit5 | 300 | 2.93% [0.75–5.98] (11/375) | 8.70% (2/23, thin) |
| rubocop | 250 | 6.96% [4.49–9.27] (55/790) | 54.55% (6/11, thin) |
| powershell | 800 | 1.78% [1.01–2.86] (32/1799) | 20.00% [10.00–34.78] (6/30) |
| fmt | 500 | 2.63% [1.35–4.14] (24/913) | 57.14% [42.86–74.29] (20/35) |

junit5's clean 0.00% at w120 becomes a real 2.93% properly sampled (an
under-sampling false pass — the sample bar earning its keep); rubocop's 3.46%
worsens to 6.96%; powershell holds its pass (1.08%→1.78%); wagtail flips
8.46%→1.69% to a pass; fmt improves 3.17%→2.63% but stays red. These rows
supersede the w120 rows in the main table above.

Artifacts: `benchmarks/results/holdout-loo{,2}/`, `benchmarks/results/recall-loo/`,
`benchmarks/results/fresh-verify/` (git-ignored, regenerable via
`argot-bench --mode honest`).
