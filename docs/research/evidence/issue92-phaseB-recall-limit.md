# Issue #92 Phase B — the recall gate is a proven fundamental limit

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout`. Capstone of the
two Phase B scouts ([manifold-outlier](issue92-phaseB-manifold-outlier.md),
[per-token MLM](issue92-phaseB-pertoken-mlm.md)) — a confound-free minimal-pair
proof that the hard recall classes are below the resolution of any hunk-level
scorer, which caps recall below the 85% gate regardless of the model.

## The decisive test — break vs its own idiomatic twin

The AUC scouts compared authored fixtures to real code, so a residual
authored-vs-real / structure confound was always arguable. The minimal pair
removes every confound: take a `wrong_error_discipline` break and swap **only the
error mechanism** to the repo's real pattern, holding identifiers, strings,
control flow, imports, and length identical. If a signal cannot rank the break
above its own twin, the break carries no hunk-level signal.

Laravel `wrong_error_discipline` fixtures (CodeBERT-mlm per-token surprise;
CodeRankEmbed whole-hunk cosine):

| break → idiomatic twin | Δ mean surprise | Δ max | Δ p95 | **emb cosine** |
|---|---:|---:|---:|---:|
| `trigger_error(…, E_USER_ERROR)` → `throw InvalidArgumentException` | 0.03 | 0.55 | 0.03 | **0.9968** |
| `die(…)` → `throw InvalidArgumentException` | 0.00 | 0.00 | 0.00 | **0.9965** |
| `error_log(…)` → `Log::error(…)` | 0.05 | 0.10 | −0.06 | **0.9912** |

`break > twin` holds on 2/3 pairs by mean, 1/3 by p95 — a coin flip. The
per-token surprise moves by ~0.00–0.05 (noise) and the pretrained embedding by
cosine ~0.996. **The break is invisible to both methods.** The `die → throw`
pair is exactly 0.00 on every surprise aggregation: the code MLM finds
`die("…")` and `throw new InvalidArgumentException("…")` equally (un)surprising.

## Why — and the recall ceiling it forces

The break tokens (`trigger_error`, `die`, `error_log`, `E_USER_ERROR`) are all
attested PHP vocabulary; the deviation is *semantic* (wrong error discipline for
the repo's exception-throwing convention), and a hunk-level representation
encodes the tokens, not the convention. This is exactly where the "representation
works" result ([pretrained-encoder-coderankembed.md], injected-AUC 0.94) does
**not** transfer: that 0.94 separated hunks from a *different repo*; a
*within-repo* semantic break sits at cosine 0.996 from its idiomatic twin.

The frozen rubric puts ≥3 `wrong_error_discipline` fixtures in each ~13-fixture
hard catalog. If that class is 0%-catchable (proven above) and
`wrong_api_within_known_lib` is at chance (structure-matched AUC 0.44,
[pertoken-mlm]), the **maximum achievable recall is ~46–77%** even with a perfect
scorer on the remaining classes — structurally below the 85% gate. The classical
import/call_receiver scorer already sits near that ceiling (laravel 62%, the rest
21–62%) by catching the lexically-visible classes; a structural scorer adds the
odd naming/shape case (laravel naming 0.92) but cannot cross the semantic-break
wall.

## Verdict

Recall ≥ 85% on the curated hard-class catalogs is a **genuine fundamental
limit** for a hunk-level scorer, not a tuning gap or a missing feature. Proven
across five methods (name attestation, BPE, JEPA, joint/​per-token MLM,
pretrained-embedding outlier) and, decisively, by confound-free minimal pairs
(embedding cosine 0.996 between a break and its idiomatic twin). Per the mission
this is a valid outcome: the honest per-language recall is reported as-is and the
non-Python languages are marked *not yet shippable* for the hard classes. The
same wall explains the existing-file `call_receiver` FP reds — a legitimate
library migration is a within-repo semantic novelty indistinguishable from a
foreign-API break by the same token-level signal.

## Product note

Even the sporadic structural wins would need a 137M-param encoder at check time;
bundling via `candle` + `include_bytes!` (~280 MB binary) is within the stated
`include_bytes!` sanction but changes argot's "small single static binary"
identity — and is moot while the signal is below gate on the classes that matter.
