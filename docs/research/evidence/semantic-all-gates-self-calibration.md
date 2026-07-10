# Semantic layer: gate-ready everywhere via fit-time self-calibration ("all-gates" era)

**Date**: 2026-07-10 (overnight autonomous run) · **Branch**: `feat/semantic-layer`
· **Commits**: `28ab16e6` → `7fed4d08` · **Definitive bench binary**: `c5dfff3e`

## Goal and result

Treat all four detectors — Foreign, Reinvention (F1), Placement (F2),
Architecture — as gating, and get every (detector, corpus) cell on the 31-corpus
suite to **recall ≥ 85 % (or a clean abstain = 0 fires) and clean-commit
over-fire ≤ 3 %/hunk**, measured leak-free (fit @ `HEAD~150`, replay).

**Result: 31/31 corpora green on the definitive real bench.**

| detector | before | after |
|---|---|---|
| F1 reinvention | recall ~90 %, FP > 3 % on 15/31 (jellyfin 7.7, curl 6.9, laravel 6.8, junit5 6.5, rubocop 6.4 …) | recall 85–100 %, FP 0–2.78 % on all 31 |
| F2 placement | recall < 85 % on ~19 (composer 0, laravel 39, jellyfin 43, hugo 52, hono 54, rubocop 57, guava 59, scrapy 60 …); FP outliers fmt 20 %, junit5 5.5 % | recall 86.5–99.0 % on all 25 judging corpora; FP ≤ 1.46 %; fmt/junit5 → 0; 6 honest abstains |
| Foreign (base) | gate-level | byte-untouched (feature-gated changes only) |
| Architecture | 97 % / 0 control-FP | re-verified identical (244/252, 0/140) |

Full per-corpus final table: `landing/src/data/semantic.json` (consolidated from
`benchmarks/results/sem_all_{A,B}.jsonl`). Aggregates: F1 recall min 85 / med 94,
raw FP 1.33 %/hunk overall; F2 recall min 86 / med 96.

## Method: capture once, sweep offline

An env-gated feature dump (`ARGOT_SEM_DUMP`, commit `28ab16e6`) records one JSON
line per check-time candidate (structural features, f16 embedding, top-40
neighbours, fire outcome). One capture pass per corpus (fit @HEAD + planted
fixtures; fit @HEAD~150 + full commit replay) made **every rule variant a pure
offline computation**. The offline re-implementation was validated against the
binary first: **0 disagreements** across thousands of candidates on 9 corpora.
All threshold sweeps and rule experiments below ran on that data; only the
winners were ported to Rust and re-measured with real fits.

## F2 placement v2 (`placement.rs` rewrite)

1. **Adaptive area walk** — a directory holding > 50 % of its parent's functions
   *or* > 25 % of all functions is a container; the walk descends and stops at
   the first non-dominant directory (areas at mixed depths). Fixes the fixed
   depth-2 failure where `src/Composer` swallowed a whole corpus into one area
   (composer transplant recall was *structurally* 0 %) and the guava+android
   mirror (each ~48 % of the tree, so neither cleared a parent-share-only test).
2. **Entangled-area flow-merge** — area pairs where ≥ 30 % of one side's top-10
   neighbours land in the other are merged (union-find); calibration may merge
   deeper (τ down to 0.10). This is what turns fmt's 20 % clean-commit FP into a
   clean abstain: `src → include/fmt` flow is 0.41 — a header-only library has
   no separable architecture to judge.
3. **Fit-time self-calibration** — grid over (τ, k ∈ {10,15,20}, z ∈ {0,1});
   fire rule = modal(top-k merged areas) ≠ own area ∧ own-area count ≤ z.
   Full-enumeration transplant simulation (every sampled fn × every foreign
   area) + in-place over-fire; pick max simulated recall s.t. over-fire ≤ 2.5 %;
   **below 85 % simulated recall the sense is disabled** — abstain, not noise.
4. **Substance floor** — candidates under 6 body lines abstain. Killed all eight
   faker `date_time` 5-line-stub FPs whose "evidence" neighbours sat at cosine
   0.66–0.75 (background noise).

Abstains on the final run: bat, commander, express, ink, rich, fmt — flat or
single-blob layouts. Everything else judges at 86.5–99.0 % recall.

Rejected on the way (numbers in `.scratch/all-gates/LOG.md`): per-area affinity
quantiles (recall ∝ over-fire, no separation), member-of-modal-area conjunct
(kills recall proportionally), deeper-k-only sweeps (composer capped ~81 %/3 %).

## F1 reinvention v3 + conservative mode (`redundant.rs`)

Static rule changes (validated on all 31, zero recall casualties):
- **Unconditional 5-line substance floor** (was weak-overlap-only; junit5's
  `assertTrue`/`isDone` 3–4-line stubs fired through the strong-overlap paths —
  11.4 %/hunk → 1.6 %).
- **Rare-callee-only family exemption** (dense ≥ 5 neighbours or symbol df ≥ 20
  is a family member unless a shared *rare* callee proves a specific
  reimplementation; callee-Jaccard alone let per-locale provider families
  through at 100 % overlap on generic helpers).
- **Same-directory margin ≥ 0.10** — co-located protocol-variant families
  (curl's `cf_h3_proxy_*` next to `cf_h2_proxy_*`) sit in a crowd
  (cos₁−cos₂ ≈ 0.01–0.06) while genuine reinventions match one original
  (median margin 0.17). curl 7.4 % → 2.7 %.
- **Identical-single-callee normal path** — both sides built around the same
  one helper confirms at cos ≥ 0.78 (excalidraw recall 80 → 90 %, zero FP cost).

**Conservative mode** (the saleor problem): repos practicing systematic parallel
implementation (checkout/order webhook mirrors, per-entity search modules) sat
at 7.5 %/hunk with no static per-fire separator — v4–v7 experiments (rare-tier
tightening, merged-area margin, broadened family filter, global margins,
commit-level batch grouping) all traded some other corpus below gate. The fix is
per-repo, decided at fit by a **git mini-replay with zero extra embedding**:
functions ADDED over the last 150 first-parent commits (tree diff + parsing the
*old* file versions for symbol sets) scored with the v3 rule against the older
entries. Trigger = estimate ≥ 9 % ∧ ≥ 100 recent functions (binomial-CI guard —
junit5's 11 % on 62 recents vs a measured 1.6 %/hunk killed the bare threshold)
∧ **twin-pair rate < 35 %** (share of index fns whose top-2 cross-file
neighbours share one symbol: a maintained mirror tree — guava+android at 48 % —
collapses every query's margin, and conservative mode there would blind the
sense: guava planted recall 94 % → 0 % if mis-triggered). Conservative gates:
cos₁ ≥ 0.85 ∧ margin ≥ 0.05.

Trigger matrix, measured on both fit points (window via extra HEAD~300 fits):
saleor 10.5 % / 14.6 % → conservative (85 % recall / 2.36 % FP);
curl 14.0 % / 18.8 % → conservative (89 % / 1.23 %); junit5, gh-cli, rubocop,
guava → standard. The Rust estimator reproduced the offline prediction exactly
(saleor real fit: est 10.48 % on 315 recents vs predicted 10.5 %/313).

Rejected triggers: LOO self-fire (gh-cli head-LOO 14 % > saleor's — ±4 pp
between fits), file-recency proxies (file-level touch too blunt), corpus margin
medians (mirrors *raise* index-side margins while collapsing query-side ones).

## Ops incidents (both worth remembering)

1. **Stranded clones**: most `benchmarks/data/*/.repo` clones were detached at
   old commits, left by killed bench runs (curl exactly at a previous fit_sha;
   fastapi at Apr-2025 vs master Jul-2026 → 6/20 fixtures targeted nonexistent
   code, recall read 0.60). Fixture-target resolution arbitrated the canonical
   state per corpus; fastapi/ink/faker/faker-js restored to branch tips and
   re-captured. Canonical heads live in `.scratch/all-gates/summaries/*.json`.
2. **Silent embedder degradation**: three concurrent giant fits exhausted Metal;
   `Embedder::ready()` errors were `.ok().flatten()`ed into "no model", so fits
   skipped the semantic index and checks fired nothing — the first definitive
   run recorded guava/powershell F1 "0 % recall" where the truth was "no
   embedder". Fixed in `c5dfff3e`: the fit eprintlns the load error and
   sem_bench/sem_fp abort when a check's stderr shows the semantic pass
   degraded. Operational rules: max 2 concurrent fits; never run `arch-verify`
   concurrently with `sem_all` (same clones — the race is exactly how the
   clones got stranded).

## Costs

Check time: unchanged in practice (all new gates are O(1) over already-fetched
neighbours; the area walk is O(index) built per scorer). Fit time: +10–20 % on
the largest corpora (placement calibration's sampled neighbour scan, capped at
8 000 queries, plus the mini-replay's git diff + old-version parses — no extra
embedding anywhere).
