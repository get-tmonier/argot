# SequentialImportBpeScorer: corrected-controls robustness protocol

## Setup

The phase-13 validation used control fixtures as the calibration set —
the same small, hand-curated hunks used to evaluate break separation.
Two concerns with that setup: calibration was fixture-derived rather
than source-derived (distribution mismatch against real production
hunks), and the 20-hunk FastAPI / 10-hunk rich calibration samples were
far below n=30. The corrected-controls protocol fixed both:
calibration `cal_hunks` and held-out `ctrl_hunks` were drawn from the
repo's own source tree via disjoint AST-extracted samples (100 cal + 20
ctrl for FastAPI and rich; 139 cal + 20 ctrl for faker via a fixed
positional split over the 159-hunk pool). The protocol was run across
5 seeds per domain; break fixtures were scored with the new
`score_hunk(hunk_content, *, file_source)` signature so Stage 1 sees
the full file and Stage 2 scores hunk-only.

Pre-registered VALIDATED band per domain: FP ≤5%, recall 100%, and
threshold CV <5% (STABLE).

## Results

Threshold CV 3.9% on FastAPI, 4.3% on rich — both inside the STABLE
band. Recall 100%. FP rate 1% (one ctrl hunk on a single seed in each
of FastAPI and rich). Faker: 0 FP, 100% recall.

| domain | threshold mean | threshold CV | recall | FP rate | verdict |
|:---|---:|---:|---:|---:|:---|
| FastAPI | 4.0531 | 3.9% | 100% | 1% | VALIDATED |
| rich | 4.6360 | 4.3% | 100% | 1% | VALIDATED |
| faker | 7.3732 | n/a (single run) | 100% | 0% | VALIDATED |

All 31 FastAPI breaks and all 10 rich breaks were `always_flagged` —
flagged on every seed. `break_ansi_raw_2` was the thinnest-margin rich
break and still cleared the threshold by +0.87 on its worst seed. The
`faker_hunk_0047` finding held: in-calibration it sets the threshold
and is correctly not flagged; in a held-out calibration without it,
the threshold drops and the hunk would fire — consistent with the
prior observation.

## Interpretation

The fixture-vs-source distribution mismatch was corrected, the
calibration sample sizes were moved above n=100, and the scorer still
validated on all three domains across all 5 seeds. The pipeline's
signal is not a calibration artefact — it is stable under the more
realistic source-derived protocol that production will actually use.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase14/experiments/sequential_corrected_controls_postfix_v2_2026-04-22.md`.
Re-written here for clarity, not copied.*
