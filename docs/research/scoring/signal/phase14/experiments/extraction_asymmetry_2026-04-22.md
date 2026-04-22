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

---

## §2. BPE Token Attribution — What Pegs the Score to 4.0668

**Script:** `engine/argot/research/signal/phase14/experiments/bpe_token_attribution_2026_04_22.py`

### Method

Five flagged `fastapi/routing.py` hunks with `bpe_score ≈ 4.0668` were selected with end_lines
spread across the file (≈200, 800, 1500, 2500, 3500). For each, the extraction (lines 1..end_line)
was tokenized with UnixCoder (`microsoft/unixcoder-base`). Model A was built from all non-excluded
source files in the fastapi repo (496 files, 234 596 total tokens), matching the original scorer's
`_collect_source_files` call. Token LLRs were computed with ε=1e-7; the max is the BPE score.

### Results

| Hunk end_line | Extraction lines | Top token | Token ID | LLR | First appears at line |
|--------------|-----------------|-----------|----------|-----|----------------------|
| 437 | 437 | `pos` | 1060 | 4.066786 | 432 |
| 799 | 799 | `pos` | 1060 | 4.066786 | 432 |
| 1527 | 1527 | `pos` | 1060 | 4.066786 | 432 |
| 2528 | 2528 | `pos` | 1060 | 4.066786 | 432 |
| 3498 | 3498 | `pos` | 1060 | 4.066786 | 432 |

**Same token across all 5 hunks: YES.** The dominant token is `pos` (id=1060), always appearing
first at **line 432** of `routing.py`, with an identical LLR of **4.0668** regardless of how many
more lines the extraction includes (from 437 to 3498 lines).

### Code context at line 432

```python
# routing.py lines 427-441
except json.JSONDecodeError as e:
    validation_error = RequestValidationError(
        [
            {
                "type": "json_invalid",
                "loc": ("body", e.pos),   # ← line 432: e.pos triggers the `pos` token
                "msg": "JSON decode error",
                "input": {},
                "ctx": {"error": e.msg},
            }
        ],
        body=e.doc,
        endpoint_ctx=endpoint_ctx,
    )
    raise validation_error from e
```

`e.pos` is an attribute access on `json.JSONDecodeError` — the character offset of the parse error.
This is production error-handling code, **not** a docstring, string literal, or import.

### Why `pos` has such high LLR

`pos` is rare in the model A corpus (fastapi source) but common in the generic model B corpus
(prose/generic code). In fastapi's production source there are very few occurrences of the bare
token `pos` — the library prefers longer names like `position`, `response`, `positive`. The generic
corpus uses `pos` heavily (command-line parsers, geometry code, text processing). This frequency
mismatch drives a high log-ratio.

### Dominant region

All 5 hunks include lines 1..432 (because every extraction for routing.py extends at least to line
437, the smallest end_line among flagged hunks). The `pos` token is always present once the
extraction reaches line 432. Line 432 is therefore in the **file prefix** — within the first ~9%
of routing.py (432 / 4956 lines).

### Verdict

**CONFIRMS Step 1.** Step 1 found that all flagged routing.py extractions span at least 437 lines
(median ≈ 1800). The token that drives every flag is on line 432, well inside the prefix that all
extractions share. The scorer is not detecting anything in the hunk-specific diff — it is latching
onto a single token in the shared prefix that all these extractions include by construction.
This is a textbook manifestation of the calibration/inference asymmetry confirmed in §1.
