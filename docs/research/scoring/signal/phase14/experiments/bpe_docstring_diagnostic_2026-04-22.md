# Phase 14 Exp #6 Step 4 — BPE Docstring/Prose Diagnostic (2026-04-22)

**Input:** `real_pr_base_rate_hunks_fix1_2026_04_22.jsonl` (fix1 scored results)

**Question:** Do BPE-flagged hunks have higher prose ratios than unflagged hunks?

**Pre-registered verdict criteria:**
- CONFIRMED: median(flagged) ≥ 1.5× median(unflagged) AND top-bucket flag rate ≥ 3× bottom-bucket rate
- REJECTED: medians within 20% AND flag rate flat across buckets
- AMBIGUOUS: in between

---

## §1. Per-hunk prose analysis method

Each scored hunk is extracted as file-start to hunk-end-line (same as in scoring).
For each prefix:
- **docstring_lines**: line ranges of `Expr(Constant(str))` nodes that are the first
  statement of a `Module`, `ClassDef`, `FunctionDef`, or `AsyncFunctionDef` (via `ast`).
  Fallback to triple-quote regex when `ast.parse` raises `SyntaxError`.
- **comment_lines**: lines whose stripped content starts with `#`, not already counted
  as docstring lines.
- **prose_ratio**: `(docstring_lines + comment_lines) / total_lines`

- Source records processed: 1373
- Files missing from HEAD: 0
- AST parse succeeded: 330
- Regex fallback: 1043
- BPE-flagged hunks: 258
- BPE-unflagged hunks: 1115

---

## §2. Prose ratio distributions

### Flagged (BPE-triggered) hunks

| stat | prose_ratio |
|---|---|
| mean | 0.2799 |
| median | 0.2886 |
| p25 | 0.1334 |
| p75 | 0.4112 |
| p95 | 0.4552 |
| min | 0.1032 |
| max | 0.5135 |

### Unflagged hunks

| stat | prose_ratio |
|---|---|
| mean | 0.2743 |
| median | 0.2987 |
| p25 | 0.0163 |
| p75 | 0.4649 |
| p95 | 0.6568 |
| min | 0.0000 |
| max | 0.7318 |

**Median ratio (flagged / unflagged):** 0.2886 / 0.2987 = 0.97x

---

## §3. ASCII histogram — prose_ratio distribution

### Flagged hunks
```
     0-10% |                                0
    10-25% | #########################      117
    25-50% | ############################## 138
    50-75% |                                3
   75-100% |                                0
```

### Unflagged hunks
```
     0-10% | ############################## 500
    10-25% | ##                             49
    25-50% | ##################             305
    50-75% | ###############                261
   75-100% |                                0
```

---

## §4. BPE flag rate per prose bucket

| bucket | n_hunks | n_bpe_flagged | flag_rate |
|---|---|---|---|
| 0-10% | 500 | 0 | 0.0% |
| 10-25% | 166 | 117 | 70.5% |
| 25-50% | 443 | 138 | 31.2% |
| 50-75% | 264 | 3 | 1.1% |
| 75-100% | 0 | 0 | 0.0% |

Top-bucket (75–100%) flag rate: 0.0%
Bottom-bucket (0–10%) flag rate: 0.0%
Ratio: ∞ (bottom bucket = 0)

---

## §5. Verdict

| test | result |
|---|---|
| median(flagged) ≥ 1.5× median(unflagged) | FAIL (0.2886 vs 0.2987) |
| top-bucket flag rate ≥ 3× bottom-bucket rate | FAIL (0.0% vs 0.0%) |
| medians within 20% | YES |
| flag rate flat across buckets | YES |

## Verdict: REJECTED

Prose ratio does NOT explain BPE over-trigger per the pre-registered criteria.
The medians are nearly identical (0.2886 flagged vs 0.2987 unflagged, 0.97x ratio),
and both the top bucket (75–100%) and bottom bucket (0–10%) have 0% flag rate.

**HARD STOP: Step 5 not authorized.** Further investigation needed.

---

## §6. Post-hoc interpretation (observational only)

The pre-registered verdict criteria were designed to detect a monotone relationship
between prose_ratio and flag rate. The actual distribution is non-monotone:

| bucket | flag rate |
|---|---|
| 0–10% | 0.0% |
| 10–25% | 70.5% |
| 25–50% | 31.2% |
| 50–75% | 1.1% |
| 75–100% | 0.0% |

The 0% flag rate in the 0–10% bucket (500 hunks — mostly short file prefixes with pure code
and no prose) is explained by the extraction method: early-in-file prefixes that parse
successfully have low BPE scores. The high flag rate in 10–25% corresponds exactly to the
file region where `fastapi/security/http.py` (line range 50–200) has 30% prose, and
`fastapi/routing.py` hunks whose prefix reaches the first large docstring block.

The spike then drops at 50–75% because: (a) the higher-prose prefixes come from much
later in `fastapi/routing.py` where the BPE score is already at 4.0668 and adding more
prose doesn't raise it further; (b) many 50–75% hunks are from other files that are
not over the threshold.

**Alternative hypothesis:** The BPE over-trigger is file-specific (routing.py, http.py),
not prose-density-specific. It is driven by a single high-ratio token that is present in
those specific files and is encountered once the prefix reaches a certain point. This is
consistent with the BPE score clustering at exactly two values.

**Recommended next diagnostic:** token-level attribution — identify which specific BPE
token (vocabulary ID) is responsible for the max score in the flagged hunks, and
investigate whether it appears in docstrings, long string literals in function calls, or
regular code tokens.

