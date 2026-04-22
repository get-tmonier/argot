# Phase 14 Exp #7 Step 7 — Structural-Refactor Null Test (Experiment G)

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Source data:** fix4 jsonl (`real_pr_base_rate_hunks_fix4_2026_04_22.jsonl`)  
**Excluded PRs:** #14564 (Python 3.9+ syntax upgrade), #14575 (Drop Pydantic v1)

---

## §0. Summary Table

| Metric | fix4 (all 50 PRs) | residual (48 PRs) | delta |
|---|---|---|---|
| PR flag rate | 22.0% (11/50) | **18.8% (9/48)** | −3.2pp |
| Flags total | 81 | **24** | −57 |
| LIKELY_STYLE_DRIFT | 7.4% (6/81) | **16.7% (4/24)** | +9.3pp |
| FALSE_POSITIVE | 85.2% (69/81) | **62.5% (15/24)** | −22.7pp |

Excluding the two refactor PRs removes 70% of flags but leaves the flag rate nearly unchanged (18.8% vs 22.0%). The hypothesis is not confirmed.

---

## §1. Residual Aggregates

- **Total PRs scored:** 48
- **PRs with any source flag:** 9 (18.8%)
- **Total source hunks scored:** 1048
- **Source hunks flagged:** 24 (2.3%)
- **Stage 1 (import):** 7 flags
- **Stage 2 (BPE):** 17 flags
- **Distinct files flagged:** 7
- **BPE score distribution (flagged hunks):** min=0.99, p25=5.49, median=5.51, p75=6.52, max=6.52

---

## §2. Residual §5 Judgment Re-Tally (pulled from fix4, not re-judged)

| PR | Flags | LIKELY | AMBIGUOUS | FP |
|---|---|---|---|---|
| #15280 (vibe()) | 1 | 0 | 1 | 0 |
| #14641 (IncEx re-export) | 1 | 0 | 0 | 1 |
| #14806 (mypy pre-commit) | 1 | 0 | 0 | 1 |
| #14609 (drop pydantic.v1) | 5 | 0 | 0 | 5 |
| #14605 (FastAPIDeprecationWarning) | 5 | 3 | 0 | 2 |
| #14583 (deprecation warnings) | 4 | 0 | 3 | 1 |
| #14371 (fix parameter aliases) | 1 | 0 | 1 | 0 |
| #14512 (tagged union discriminator) | 1 | 0 | 0 | 1 |
| #14306 (traceback endpoint metadata) | 5 | 1 | 0 | 4 |
| **Total** | **24** | **4** | **5** | **15** |

| Judgment | Count | % |
|---|---|---|
| LIKELY_STYLE_DRIFT | 4 | 16.7% |
| AMBIGUOUS | 5 | 20.8% |
| FALSE_POSITIVE | 15 | 62.5% |

**Spot-check against fix4 §5 (3 entries):**
- Flag #9 (PR #14605, `fastapi/dependencies/utils.py`) → LIKELY_STYLE_DRIFT ✓
- Flag #14 (PR #14583, `fastapi/dependencies/utils.py`) → AMBIGUOUS ✓
- Flag #78 (PR #14306, `fastapi/routing.py`) → LIKELY_STYLE_DRIFT ✓

Counts match fix4 §5 exactly. Expected ~4 LIKELY / ~5 AMBIGUOUS / ~15 FALSE_POSITIVE per the spec; observed 4/5/15. No transcription error.

---

## §3. Age-Bracket Breakdown (residual)

| Age bucket | PRs | Src hunks | Flagged | Flag rate | PRs flagged |
|---|---|---|---|---|---|
| ≤90 days | 39 | 896 | 3 | 0.3% | 3 |
| 91–180 days | 9 | 152 | 21 | 13.8% | 6 |

The 91–180 day bucket still dominates after exclusion (13.8% vs 0.3%). Excluding the two structural-refactor PRs reduces the bucket from 11 PRs to 9 and from 78 flags to 21, but the relative pattern persists: Dec 2025 PRs are flagged at 46× the rate of recent PRs. The age gradient is a property of the age cohort, not of the two excluded PRs specifically.

---

## §4. Verdict: REJECTED (with precision caveat)

**Flag rate: 18.8% > 15% → REJECTED zone.**  
**Precision (LIKELY_STYLE_DRIFT): 16.7% > 15% → marginally above REJECTED threshold.**

Pre-registered REJECTED band requires flag rate > 15% AND precision < 15%. Flag rate meets the condition; precision (16.7%) falls 1.7pp above the threshold — within one flag of the boundary. Per the spec's gap rule, no verdict upgrade is issued. The result is **REJECTED** with a precision caveat.

Interpretation: removing PRs #14564 and #14575 eliminates 57 flags but moves the PR flag rate only from 22.0% to 18.8% — a 3.2pp reduction against a 15% threshold. Nine of 48 residual PRs are still flagged, and the age-bracket gradient persists almost unchanged. The "structural mass-refactors are the dominant FP source" diagnosis partially holds (those two PRs accounted for 70% of flag volume) but the flag rate problem is not localized to them. The residual 9 flagged PRs represent a substantive and distributed FP problem that two-PR exclusion cannot fix.

Precision improvement is real (85.2% → 62.5% FP rate) but the scorer still misclassifies the majority of residual flags. With only 4 LIKELY_STYLE_DRIFT flags out of 24, precision (16.7%) remains far below any useful operating threshold.

---

## §5. Implication for Experiment H (rolling-window calibration)

**H is still warranted but with tempered expectations.**

The temporal calibration mismatch identified in fix4 §7 is a real mechanism — the 13.8% flag rate in the 91–180 day bucket vs 0.3% for recent PRs confirms that older PRs are systematically over-flagged. Rolling-window calibration would address this by calibrating on a snapshot contemporaneous with each PR rather than on post-merge HEAD.

However, this experiment shows the residual problem is distributed across 9 PRs and multiple flag types, not concentrated in two outliers. Rolling-window calibration will likely reduce the 91–180 day bucket flag rate substantially (the temporal mismatch is the dominant mechanism for that bucket), but it will not eliminate all sources of false positives. The 3 flags in the ≤90-day bucket (#15280, #14641, #14806) represent a separate FP class — vocabulary rarity and import pattern sensitivity — that calibration timing does not address.

**Revised expectation for Exp H:** rolling-window calibration is likely to push the 91–180 day flag rate from ~14% toward 2–5%, potentially bringing overall PR flag rate below 10%. Whether that gets to V1 USEFUL depends on whether the precision gap also closes (temporal mismatch is the dominant cause of FP classification, so calibration should also lift precision).

Proceed with H, but pre-register a precision condition alongside the flag rate threshold.
