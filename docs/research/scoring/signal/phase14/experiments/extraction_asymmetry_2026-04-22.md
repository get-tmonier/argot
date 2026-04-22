# Phase 14 Experiment 7 — Calibration/Inference Extraction Asymmetry (2026-04-22)

## §1. Extraction Length Distributions

Calibration uses raw AST-extracted function/class bodies (`sample_hunks(fastapi, n=100, seed=0)`).
Inference uses file-start-to-hunk-end extraction (`_extract_file_to_hunk_end`), reconstructed
from the 1373 records in `real_pr_base_rate_hunks_fix1_2026_04_22.jsonl`.

| Set | n | Lines (min/p25/med/p75/p95/max) | Chars (min/p25/med/p75/p95/max) |
|-----|---|--------------------------------|--------------------------------|
| calibration | 100 | 6 / 7 / 10 / 16 / 101 / 303 | 142 / 230 / 359 / 618 / 3711 / 8070 |
| inference (all) | 1,373 | 3 / 151 / 557 / 1708 / 3883 / 4644 | 42 / 5066 / 19610 / 54628 / 153550 / 185210 |
| inference (flagged) | 258 | 74 / 797 / 1800 / 3156 / 4266 / 4627 | 1971 / 32374 / 73308 / 126920 / 170819 / 185210 |

### Ratios (inference_all / calibration)

| Stat | Lines ratio | Chars ratio |
|------|-------------|-------------|
| median | 55.70x | 54.62x |
| p95    | 38.28x | 41.37x |

### Pre-registered verdict: **ASYMMETRY CONFIRMED**

**Criteria:**
- ASYMMETRY CONFIRMED requires: median(inf_lines) ≥ 3× median(cal_lines) AND p95(inf_lines) ≥ 5× p95(cal_lines)
- ASYMMETRY REJECTED requires: both stats within 50% of each other

**Result:** median ratio = 55.70x (threshold 3×), p95 ratio = 38.28x (threshold 5×)

Both conditions for CONFIRMED are satisfied: inference extractions are substantially longer than calibration hunks. The scorer sees much more context at inference time than at calibration time, which is a plausible driver of the false-positive rate.
