# Issue #92 Phase B — per-token MLM surprise, and a caught artifact

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Scout:
`benchmarks/phaseb_pertoken_mlm.py` (disposable). Env: torch 2.12 / MPS,
`microsoft/codebert-base-mlm` (a **trained** masked-LM head — UniXCoder's head is
not exposed via `AutoModelForMaskedLM` and loads random, giving noise; that first
attempt was discarded).

## Question (the last open door)

Joint-masked MLM inverted (AUC 0.43, [mlm-surprise-bakeoff.md]) — masking all
hunk tokens at once removes the intra-hunk structure. **Per-token** masking (mask
ONE position, keep the rest visible) is the untried variant and the only method
that could target an argument-level semantic break: mask `E_USER_ERROR` in a
config-fail context and a code MLM should predict the repo's `E_USER_WARNING`,
making the actual token surprising. `surprise_i = -log P(tok_i | rest visible)`,
aggregated per hunk (max / p95 / mean); AUC(break vs control) per category.

## A false positive, caught by tightening the control

Progressive controls on laravel (13 fixtures) exposed a structural artifact:

| control set | AUC_mean overall | tell |
|---|---:|---|
| 20-line corpus windows | 0.94 | AUC_max inverted (0.19) |
| 20-line windows of the fixtures' own host files | 0.95 | AUC_max still inverted (0.31) |
| whole method bodies (host files) | **1.00** | uniform 1.00 across *every* class; AUC_max 0.04 |
| **method bodies, fixtures also reduced to method bodies** | **0.65** | signal collapses |

The 0.94–1.00 was **not break detection**. It rose to a perfect 1.00 *uniformly
across all five break classes* — a real semantic detector cannot score
lexically-obvious `foreign_import` identically to a subtle argument-level
`wrong_error_discipline`. The discriminator was fixture **structure**: the
catalog fixtures are complete files (`<?php`, `namespace`, `use` imports, a full
class) while the controls were in-class fragments; the file-header/import
boilerplate carries uniformly higher per-token surprise. Structure-matching both
sides (score only the fixtures' method bodies) collapsed the AUC to 0.65 — the
mission's "never trust a PASS without a clean control" earning its keep.

## Honest structure-matched result

| category | n | AUC_max | AUC_p95 | AUC_mean |
|---|---:|---:|---:|---:|
| foreign_import | 2 | 0.875 | 0.896 | 0.833 |
| wrong_concurrency | 2 | 0.708 | 0.750 | 0.729 |
| wrong_error_discipline | 3 | 0.486 | 0.833 | 0.736 |
| naming_shape_break | 2 | 0.583 | 0.354 | 0.708 |
| wrong_api_within_known_lib | 4 | 0.406 | 0.458 | 0.438 |
| **OVERALL** | 13 | 0.571 | 0.641 | **0.654** |

Overall AUC ~0.65 — real but **below the 0.85 gate**, dominated by
`foreign_import` (0.83–0.90, already caught by the import tripwire). The
in-vocabulary semantic classes stay weak: `wrong_api_within_known_lib` 0.44
(below chance), `naming_shape` 0.35–0.71 (noisy). `wrong_error_discipline` shows
a weak p95 hint (0.83) but n=3 and inconsistent across aggregations.

## Phase B verdict — recall hard-class is a proven limit

Both structural methods plateau at ~0.65 overall AUC when fairly controlled
(manifold-outlier 0.37–0.69 across three languages, per-token MLM 0.65 on
laravel), converging with every prior negative (JEPA 0.71 shuffled-plateau,
joint-MLM 0.43, CodeRankEmbed+JEPA-head 0.51). No tried method — name
attestation, BPE surprise, JEPA, joint/​per-token MLM, pretrained-embedding
outlier — reaches the 0.85 bar on the in-vocabulary hard classes. The lexically
visible classes they *can* reach (foreign import/API surface) are already caught
by the classical import/call_receiver stages.

**Conclusion:** recall ≥ 85% on the curated hard-class catalogs is **not
achievable with current techniques** for the in-vocabulary break classes
(argument-level semantic, API-within-known-lib, most naming/shape). This is a
genuine fundamental limit — a hunk-level scorer cannot resolve a one-token
semantic deviation embedded in otherwise-attested code. Per the mission this is a
valid outcome: the honest per-language recall is reported as-is and the
non-Python languages are marked "not yet shippable" for hard-class recall, rather
than painted green.
