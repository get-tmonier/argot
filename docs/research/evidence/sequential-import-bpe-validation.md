# SequentialImportBpeScorer: phase-13 domain validation

## Setup

`SequentialImportBpeScorer` wires two axes in series: Stage 1 imports
flags any hunk with ≥1 foreign module (the exact rule from the import
graph pre-filter); Stage 2 BPE catches the rest against a per-repo
calibration threshold (max BPE over the calibration sample). The
hypothesis: Stage 1 handles the 67% that the import axis already covers
with zero FPs, and Stage 2 recovers the stdlib-only breaks that the
import axis cannot see, without pushing FP rate above the 2% band.

Validation reused the phase-13 fixtures across all three corpora —
FastAPI (20 control-fixture calibration hunks, 31 breaks + 20 controls),
rich (10 control-fixture cal hunks, 10 breaks + 10 controls), faker
(159 calibration hunks, 5 breaks). Pre-registered STRONG band: ≥85%
recall, ≤2% FP, *and* `faker_hunk_0047` not flagged.

## Results

100% recall on 46 breaks. 0 FP across 30 held-out controls and 159
faker calibration hunks. All three domains STRONG.

| domain | breaks | recall | controls/cal | FP rate | verdict |
|:---|:---:|---:|:---:|---:|:---|
| FastAPI | 31 | 100% | 20 | 0% | STRONG |
| rich | 10 | 100% | 10 | 0% | STRONG |
| faker | 5 | 100% | 159 | 0% | STRONG |
| **combined** | **46** | **100%** | **189** | **0%** | — |

Stage attribution: on FastAPI, Stage 1 caught 20/31 and Stage 2 BPE
recovered all 11 stdlib-only FNs (`manual_validation`, `bare_except`,
`assert_validation`, `imperative_route_loop`, etc.) cleanly above their
respective BPE thresholds. On rich, Stage 1 caught 6/10 and Stage 2
recovered all 4 (the `ansi_raw` and `print_manual` breaks). On faker,
all 5 breaks cleared Stage 1 and Stage 2 never needed to fire.

`faker_hunk_0047` trace: `import_score = 0`, `bpe_score = 7.3732`, faker
threshold `7.3732`. Condition `bpe_score > threshold` evaluates `7.3732
> 7.3732` → False. Correctly suppressed at `bpe_score = threshold`.

## Interpretation

The two-axis sequential pipeline cleared the gate the single-axis
scorers could not: every break flagged, no false positives, and the
one diagnostic outlier from era 3 was suppressed at exactly the right
boundary. Promoted as the Phase 14 primary scorer, pending the
corrected-controls robustness protocol.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase14/experiments/sequential_import_bpe_phase13_validation_2026-04-22.md`.
Re-written here for clarity, not copied.*
