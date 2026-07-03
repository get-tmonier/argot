# Issue #92 Phase B — repo-manifold outlier on frozen CodeRankEmbed

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Scout:
`benchmarks/phaseb_manifold_outlier.py` (disposable). Env: torch 2.12 / MPS,
`nomic-ai/CodeRankEmbed` frozen (the encoder [pretrained-encoder-coderankembed.md]
found separates home-vs-foreign at injected-AUC 0.94 but scored 0.50 on the
in-distribution *mutations* via the JEPA head).

## Question

The goal flagged "a density / kNN-outlier use of a pretrained code embedding" as
the most promising untried recall direction — the 0.50 mutation AUC was measured
as JEPA-head *distance*, never as a **repo-manifold outlier**. Does an outlier
score (kNN cosine distance to a manifold of the repo's own code windows)
separate the curated hard-class breaks from idiomatic controls? Tested at two
granularities: whole-region (20-line windows) and **localized** (8-line windows,
score = max window outlier — to surface a one-line break a whole-region
embedding would drown). Positives = curated break fixtures (own `Break:`
annotation comments stripped — leakage). Controls = held-out corpus regions
(leak-free: their windows are not in the manifold). Metric = AUC(break vs
control), per category.

## Results (AUC, localized unless noted)

| corpus | OVERALL | strong class | dead class |
|---|---:|---|---|
| laravel (PHP) | **0.685** | naming_shape_break **0.92** | wrong_error_discipline **0.50** |
| rich (Python) | **0.672** | curses (foreign API) **0.86** | print_manual 0.60 |
| redis (C) | **0.369** | — (all ≤0.50) | wrong_error_discipline **0.27**, all **inverted** |

Per-class, laravel: naming_shape 0.92, wrong_concurrency 0.75,
wrong_api_within_known_lib 0.73, wrong_error_discipline 0.50, foreign_import
0.57 (n=2). Localized ≥ whole-region on nearly every class, but the lift is
small (overall 0.653 → 0.685 laravel). redis inverts because low-level C is so
diverse the control manifold is diffuse (control median distance 0.65 vs
laravel 0.58) — idiomatic C windows are themselves outliers, so breaks do not
stand out.

## Conclusion — not the lever

The repo-manifold outlier is **weak and inconsistent** (overall 0.37–0.69 across
three languages, all below the 0.85 gate), with only sporadic per-class hits
(laravel naming 0.92, rich curses 0.86 — both lexically-visible foreign
surface, which the import/call_receiver stages already reach) and **inverted on
C**. The one class every method misses — `wrong_error_discipline`, an
argument-level semantic break where every token/callee is attested and only the
*value/context* differs (`trigger_error(E_USER_ERROR)` for a missing key vs the
repo's `E_USER_WARNING` deprecation sites) — stays at chance (≤0.50) everywhere.

This corroborates the prior negatives (JEPA 0.71 shuffled-plateau, joint-MLM
inverted 0.43, CodeRankEmbed+JEPA-head 0.51) with a *new, untried* method, and
strengthens the honest read: **frozen-embedding methods do not recover the
in-vocabulary hard-class recall**; the lexically-visible classes they can catch
are already caught by the classical stages. Representation is not the gap —
*localization of a semantic argument-level deviation* is, and a whole/near-whole
embedding cannot resolve it.

## Remaining open door

Per-token (non-joint) MLM surprise was never measured (joint-masking, which
failed, masks all hunk tokens at once and cannot use intra-hunk structure). It
is the one method that could target the argument-level class directly. Measured
next ([issue92-phaseB-pertoken-mlm.md]).

## Product tension (recorded)

Even the sporadic wins (naming 0.92) would need CodeRankEmbed at **check** time
to embed the diff hunk — a 137M-param transformer. Bundling via
`candle` + `include_bytes!` fp16 weights (~280 MB binary) is within the stated
constraint but would fundamentally change argot's "small single static binary"
identity. Moot given the signal is below gate.
