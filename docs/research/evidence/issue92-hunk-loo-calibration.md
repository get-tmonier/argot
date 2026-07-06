# Issue #92 — leave-one-hunk-out calibration: tested and refuted

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Outcome: **rejected**,
change reverted. Companion to [issue92-honest-rebench.md](issue92-honest-rebench.md).

## Hypothesis (calibration over-correction)

The #92 existing-file BPE threshold is calibrated **leave-one-file-out**
(`multi_seed_thresholds` subtracts each cal hunk's *entire host file* token
counts, `calibration.rs`). But at check time an edit to an *existing* file is
scored against the fit snapshot, which still contains that file — only the
newly-added hunk lines are unseen. That is **leave-one-hunk-out**. So the
calibration counterfactual (whole file absent) is strictly harsher than the
check condition (only the hunk absent), which should over-raise the existing
threshold and suppress recall. Proposed fix: exclude only the cal hunk's own
tokens for the existing threshold (`bpe_score_excluding(&blanked,
&bpe.token_counts(&c.hunk))`); keep file-out for the genuinely-new-file
threshold, where the whole file *is* unseen.

## Measurement (production recall + leak-free temporal-holdout FP)

Same pinned SHAs / windows as the honest re-bench; file-LOO (baseline) vs
hunk-LOO (this change):

| Corpus | Recall file→hunk | FP existing file→hunk | Threshold shift |
|---|---|---|---|
| laravel | 8/13 → **8/13** | 0.84% (4/476) → **2.31% (11/476)** ❌ | ~−0.1 |
| rich | 11/16 → **11/16** | 2.81% (11/391) → **2.81% (11/391)** | 7.82 → 7.68 |
| guava | 8/14 → **9/14** (+1) | 2.06% (50/2424) → **3.63% (88/2424)** ❌ | ~+0.0 |

## Why it fails — the threshold is a max over hunk-unique tokens

`bpe_score` is the **max** token surprise over a hunk, and the threshold is the
**max** over calibration hunks — so it is set by the single rarest token in the
whole calibration sample. That token is almost always **hunk-unique** (a rare
identifier appearing only in its hunk), and for a hunk-unique token
leave-one-file-out and leave-one-hunk-out remove the *same* occurrences. Hence
the two thresholds nearly coincide (shifts < 0.15). The over-correction is real
for non-max tokens but invisible to a max-of-max threshold.

The tiny threshold drop that does occur is **net-negative**: +1 catch (one
borderline guava fixture) against **+45 false positives** on real code
(laravel 4→11, guava 50→88), pushing two corpora over the ≤2% existing-file
gate they had passed. Recall barely moves because the misses sit at BPE
surprise ≈ 0 (the `wrong_error_discipline` / attested-vocabulary class, cf. the
minimal-pair proof in [issue92-phaseB-recall-limit.md]) — far below any
threshold — so lowering the bar cannot reach them; it only sweeps in real code.

## Verdict

Leave-one-file-out is not an over-correction that strangles recall; it is a
reasonable, protective operating point. Lowering it trades a large FP increase
for ~zero recall. This is a **third independent confirmation** (after the
AUC sweep and the minimal-pair proof) that BPE surprise cannot separate the
hard breaks from ordinary code at *any* threshold in this region. The Rust
port's scorer/calibration logic is faithful to the Python engine (orchestrator
diff verified); calibration is **ruled out** as the cause of the honest-bench
recall gap. Change reverted; artifacts `benchmarks/results/hunkloo-{recall,fp}/`
(git-ignored, regenerable).
